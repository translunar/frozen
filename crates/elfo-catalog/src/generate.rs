//! Orchestration for `elfo-catalog gen`: for every (combo, resonance) pair,
//! seed and correct the first family member, continue in both directions,
//! sample and QA every member, and write the on-disk catalog.

use std::path::Path;

use elfo_core::constants::{nd_to_km, nd_to_s, MU_MOON_ND};
use elfo_core::continuation::continue_family;
use elfo_core::elements::{rotating_to_inertial, rv_to_coe};
use elfo_core::forces::ForceModel;
use elfo_core::integrator::Dp54;
use elfo_core::seeds::elfo_seed_resonant_checked;
use elfo_core::shooting::{correct, seed_nodes, Constraint, PeriodicOrbit};
use elfo_core::stability::{monodromy, stability_indices};
use rayon::prelude::*;

use crate::config::{CatalogConfig, ComboCfg, Resonance};
use crate::qa::check_member;
use crate::seedcache::{SeedCache, SeedRecord, SEED_SCHEMA_VERSION};
use crate::writer::{
    decimate, write_catalog, write_f32, ComboOut, ElementsOut, FamilyOut, MemberOut, Provenance,
};

/// Knobs on a generation run that are not part of the catalog's *definition* (which
/// lives in `catalog.toml`) but of how this particular invocation treats the seed
/// cache. Kept out of the config file deliberately: `--write-seeds` is an authoring
/// action, not a property of the catalog.
#[derive(Debug, Clone)]
pub struct GenOptions {
    /// Where the committed first-member seeds live.
    pub seeds: SeedCache,
    /// Update the cache from this run: write converged first members and confirmed
    /// absences back to `seeds/`.
    pub write_seeds: bool,
    /// Attempt families listed in `absent.json` anyway. Set this after touching the
    /// corrector, the seed solver or the force model — the recorded absences are
    /// measurements of *this* code, and they expire when it changes.
    pub retry_absent: bool,
}

impl Default for GenOptions {
    fn default() -> Self {
        GenOptions {
            seeds: SeedCache::new(SeedCache::default_root()),
            write_seeds: false,
            retry_absent: matches!(std::env::var("ELFO_RETRY_ABSENT").as_deref(), Ok("1")),
        }
    }
}

/// Public entry point: load `config`, generate every family, write the
/// catalog (JSON + per-member/preview `.f32` trajectories) under `out`.
pub fn run(config: &Path, out: &Path) -> anyhow::Result<()> {
    run_with(config, out, &GenOptions::default())
}

/// [`run`] with explicit seed-cache options.
pub fn run_with(config: &Path, out: &Path, opts: &GenOptions) -> anyhow::Result<()> {
    let cfg = CatalogConfig::load(config)?;

    let pairs: Vec<(usize, Resonance)> = cfg
        .combos
        .iter()
        .enumerate()
        .flat_map(|(ci, _)| cfg.resonances.iter().map(move |&r| (ci, r)))
        .filter(|(ci, r)| {
            let id = &cfg.combos[*ci].id;
            if !generates(id, *r) {
                eprintln!(
                    "skipped: combo {id} n={r}: M:k families are generated for combo \
                     '{FULL_COMBO_ID}' only"
                );
                return false;
            }
            true
        })
        .collect();

    let results: anyhow::Result<Vec<(usize, Resonance, Outcome)>> = pairs
        .par_iter()
        .map(|&(ci, res)| -> anyhow::Result<(usize, Resonance, Outcome)> {
            let combo = &cfg.combos[ci];
            let outcome = build_family(combo, res, &cfg, out, opts)?;
            Ok((ci, res, outcome))
        })
        .collect();
    let results = results?;

    let mut combos_out: Vec<ComboOut> = cfg
        .combos
        .iter()
        .map(|c| ComboOut {
            id: c.id.clone(),
            name: c.name.clone(),
            terms: c.force_model,
            families: Vec::new(),
        })
        .collect();
    // Cache updates are collected here and applied serially after the parallel
    // sweep: a rayon worker writing into `seeds/` would race every other worker on
    // the same combo's `absent.json`.
    let mut seeds_to_write: Vec<SeedRecord> = Vec::new();
    let mut absences: Vec<(usize, Resonance, String)> = Vec::new();
    let mut present: Vec<(usize, Resonance)> = Vec::new();
    for (ci, res, outcome) in results {
        if let Some(fam) = outcome.family {
            combos_out[ci].families.push(fam);
        }
        if let Some(rec) = outcome.seed {
            present.push((ci, res));
            seeds_to_write.push(rec);
        }
        if let Some(note) = outcome.absent {
            absences.push((ci, res, note));
        }
    }
    if opts.write_seeds {
        write_cache_updates(&cfg, opts, &seeds_to_write, &absences, &present)?;
    }
    for c in combos_out.iter_mut() {
        c.families.sort_by_key(|f| (f.resonance_n, f.closures));
    }

    let provenance = Provenance {
        date: chrono::Utc::now().to_rfc3339(),
        git_hash: git_hash(),
    };
    write_catalog(out, combos_out, provenance)?;
    Ok(())
}

/// A cheap manual clone of `PeriodicOrbit` (it has no `Clone` impl in
/// elfo-core, and all its fields are public, so this needs no core change):
/// used so the same first-member solution can seed both continuation
/// directions independently.
fn clone_orbit(o: &PeriodicOrbit) -> PeriodicOrbit {
    PeriodicOrbit {
        nodes: o.nodes.clone(),
        period: o.period,
        residual: o.residual,
        segment_stms: o.segment_stms.clone(),
    }
}

/// The one combo the expensive `M:k` (`k > 1`) families are generated for.
const FULL_COMBO_ID: &str = "full";

/// Whether `(combo, res)` is in scope for this run.
///
/// The `M:k` (`k > 1`) families correct over the full `k`-closure period, so they
/// carry 2M shooting segments — 200–350 for the entries in `catalog.toml`, against
/// 50 for N = 25 — and the Jacobian SVD is cubic in that. They are generated for the
/// reference force model only; the C22/J3/Earth sensitivity variants of the heavy
/// families are a later, separately budgeted job. Every skip is announced on stderr,
/// so an absent `no-c22/n149_2` is a recorded decision rather than a silent hole.
fn generates(combo_id: &str, res: Resonance) -> bool {
    res.closures == 1 || combo_id == FULL_COMBO_ID
}

/// Hard cap on shooting segments. The corrector SVDs a `(6m+1) × (6m+1)` Jacobian
/// every Newton iteration, which is O(m³): the 173:2 family would ask for 692
/// segments at 4M, a ~64× cost over the 173-segment solve. 400 keeps 2 nodes/rev
/// through the largest configured `M` and clamps the retry instead of the first
/// attempt, so nothing in the current catalog loses its first-pass resolution.
const MAX_SEGMENTS: usize = 400;

/// Try the cached first member for `(combo, res)`. A hit that still converges is
/// worth 5–15 Newton steps of the analytic path; a hit that *stops* converging is
/// reported and falls through, because the honest reading of that is "the physics
/// under this seed changed", not "the run failed".
///
/// The same `2M`-then-`4M` escalation as the analytic path, and it is load-bearing
/// rather than symmetric-for-its-own-sake: some cached seeds were *found* at `4M`
/// (N = 70 is), and `seed_nodes` rebuilds the node set by re-propagating from
/// `state0`, so the warm start is not handed the converged node set — it is handed a
/// nearby one, and it needs the same resolution that found the orbit to close it
/// again. Without the retry, N = 70 would be banked and then silently unusable.
fn warm_start(combo: &ComboCfg, res: Resonance, cache: &SeedCache) -> Option<PeriodicOrbit> {
    let rec = cache.load(&combo.id, res)?;
    let mut tried: Vec<usize> = Vec::new();
    let mut last_err = String::new();
    for mult in [2usize, 4] {
        let m = (mult * res.revs as usize).min(MAX_SEGMENTS);
        if tried.contains(&m) {
            continue;
        }
        tried.push(m);
        // Both the node build and the correction run under `catch_unwind`. The
        // integrator *panics* on a step-size collapse — a deliberate choice
        // elsewhere in elfo-core, since a collapsed step in a catalog solve means a
        // trajectory that has gone hyperbolic or hit the Moon — and a seed stale
        // enough to do that would otherwise take the entire catalog run down from
        // inside a rayon worker. The cache is committed data that outlives the
        // physics it was measured against, so "this seed is no longer usable" has to
        // degrade to a fallback, not an abort. Correctness is unaffected: the
        // fallback is the analytic seed, which is what a cache miss already does.
        let solved = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let nodes = seed_nodes(&combo.force_model, &rec.state0, rec.period_nd, m);
            correct(&combo.force_model, &nodes, rec.period_nd, &Constraint::None)
        }));
        let solved = match solved {
            Ok(r) => r,
            Err(_) => Err("cached seed diverged the integrator".to_string()),
        };
        match solved {
            Ok(orbit) => {
                eprintln!(
                    "warm: combo {} n={res}: cached seed (from {}) converged at m={m}, \
                     residual {:.3e}",
                    combo.id, rec.generated_by, orbit.residual
                );
                return Some(orbit);
            }
            Err(e) => last_err = e,
        }
    }
    eprintln!(
        "warn: combo {} n={res}: cached seed (from {}) no longer converges at m={tried:?} \
         ({last_err}); falling back to the analytic seed",
        combo.id, rec.generated_by
    );
    None
}

/// Seed + correct the first member of the `res` family in `combo`'s force model,
/// retrying once at `m = 4M` segments if `m = 2M` stalls. Returns `None` (having
/// logged why to stderr) if the family is absent — that is data, not an error.
///
/// The seed cache is consulted first and the recorded-absence list second, in that
/// order: a converged seed for a family that also appears in `absent.json` (because
/// it was conquered by the campaign but the list was not cleaned) must win, since it
/// is direct evidence against the absence.
fn first_member(combo: &ComboCfg, res: Resonance, opts: &GenOptions) -> FirstMember {
    let fm = combo.force_model;
    if let Some(orbit) = warm_start(combo, res, &opts.seeds) {
        return FirstMember::Converged(orbit);
    }
    if !opts.retry_absent {
        if let Some(note) = opts.seeds.absent_note(&combo.id, res) {
            eprintln!(
                "absent: combo {} n={res}: skipped, recorded in absent.json ({note}); \
                 set ELFO_RETRY_ABSENT=1 to attempt it anyway",
                combo.id
            );
            return FirstMember::Skipped;
        }
    }
    let (seed, t0) = match elfo_seed_resonant_checked(res.revs, res.closures, &fm) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("absent: combo {} n={res}: seed unreachable: {e}", combo.id);
            return FirstMember::Absent(format!("seed unreachable: {e}"));
        }
    };
    let mut last_err = String::new();
    let mut tried: Vec<usize> = Vec::new();
    for mult in [2usize, 4] {
        let want = mult * res.revs as usize;
        let m = want.min(MAX_SEGMENTS);
        if m < want {
            eprintln!(
                "warn: combo {} n={res}: {mult}M = {want} segments capped at {MAX_SEGMENTS}",
                combo.id
            );
        }
        // With the cap in play 2M and 4M can collapse onto the same m; re-running an
        // identical solve would just burn minutes for an identical answer.
        if tried.contains(&m) {
            continue;
        }
        tried.push(m);
        let nodes = seed_nodes(&fm, &seed, t0, m);
        match correct(&fm, &nodes, t0, &Constraint::None) {
            Ok(orbit) => return FirstMember::Converged(orbit),
            Err(e) => last_err = e,
        }
    }
    eprintln!(
        "absent: combo {} n={res}: corrector stalled at m={tried:?}: {last_err}",
        combo.id
    );
    FirstMember::Absent(format!("analytic seed: corrector stalled at m={tried:?}: {last_err}"))
}

/// What the first-member solve produced, distinguishing the two kinds of absence:
/// one that was *measured* on this run and is therefore worth recording, and one
/// that was skipped *because* it is already recorded.
enum FirstMember {
    Converged(PeriodicOrbit),
    /// Measured absence, with the note to store in `absent.json`.
    Absent(String),
    /// Not attempted: already on the absence list.
    Skipped,
}

/// What one (combo, resonance) pair contributed: its family, if any, plus whatever
/// the seed cache should learn from it.
struct Outcome {
    family: Option<FamilyOut>,
    /// The converged first member, for `--write-seeds`.
    seed: Option<SeedRecord>,
    /// A freshly measured absence, for `--write-seeds`.
    absent: Option<String>,
}

/// Build one (combo, resonance) family end to end: seed/correct, continue in
/// both directions, sample + QA every member, write its files. An `Outcome` with
/// `family: None` means the family is absent (combo present, family missing).
fn build_family(
    combo: &ComboCfg,
    res: Resonance,
    cfg: &CatalogConfig,
    out: &Path,
    opts: &GenOptions,
) -> anyhow::Result<Outcome> {
    let fm = combo.force_model;

    let first = match first_member(combo, res, opts) {
        FirstMember::Converged(o) => o,
        FirstMember::Absent(note) => {
            return Ok(Outcome { family: None, seed: None, absent: Some(note) })
        }
        FirstMember::Skipped => return Ok(Outcome { family: None, seed: None, absent: None }),
    };

    // Snapshot the converged first member *before* continuation consumes it: this,
    // not `kept[0]`, is the seed-cache record. `kept[0]` is the far end of the
    // negative continuation branch, several arclength steps away, and re-seeding
    // from it would walk the family a little further each time it was regenerated.
    let seed_record = SeedRecord {
        schema_version: SEED_SCHEMA_VERSION,
        combo_id: combo.id.clone(),
        revs: res.revs,
        closures: res.closures,
        state0: first.nodes[0],
        period_nd: first.period,
        residual: first.residual,
        generated_by: git_hash(),
    };

    let members_per_direction = cfg.members_for(res);
    let first_pos = clone_orbit(&first);
    let pos = continue_family(&fm, first_pos, members_per_direction, cfg.ds0, 1.0);
    let neg = continue_family(&fm, first, members_per_direction, cfg.ds0, -1.0);

    // Concatenate: reverse the -1 side (dropping its duplicated first
    // member, shared with pos[0]), then the +1 side, giving a family ordered
    // monotonically along the continuation parameter.
    let mut orbits = neg;
    orbits.remove(0);
    orbits.reverse();
    orbits.extend(pos);

    // Sample, compute elements/stability, and QA-filter every candidate
    // member, in family order (so QA jump checks compare adjacent members).
    let mut candidates: Vec<(MemberOut, Vec<[f64; 3]>)> = Vec::with_capacity(orbits.len());
    for orbit in &orbits {
        candidates.push(build_member(&fm, orbit, res));
    }

    let mut kept: Vec<(MemberOut, Vec<[f64; 3]>)> = Vec::with_capacity(candidates.len());
    let mut prev: Option<MemberOut> = None;
    for (member, positions) in candidates {
        let flags = check_member(&member, prev.as_ref());
        let hard_fail = flags
            .iter()
            .any(|f| f.contains("residual") || f.contains("periapsis"));
        for f in &flags {
            eprintln!(
                "QA[{} n={res}]: {}{}",
                combo.id,
                f,
                if hard_fail { " (dropping member)" } else { "" }
            );
        }
        if hard_fail {
            continue;
        }
        prev = Some(member.clone());
        kept.push((member, positions));
    }

    if kept.is_empty() {
        // Absent, but *not* recorded in `absent.json`: the corrector converged, so
        // the seed is good and re-attempting it next run is cheap. What failed was
        // QA, and skipping the family on that basis would hide a QA regression.
        eprintln!("absent: combo {} n={res}: no member survived QA", combo.id);
        return Ok(Outcome { family: None, seed: Some(seed_record), absent: None });
    }

    // Re-index 0..len and write per-member trajectory files plus the
    // decimated family preview.
    let dir = res.dir();
    let mut preview_points: Vec<[f64; 3]> = Vec::new();
    let mut preview_counts: Vec<u32> = Vec::new();
    for (i, (member, positions)) in kept.iter_mut().enumerate() {
        member.index = i;
        member.traj = format!("{}/{dir}/{i}.f32", combo.id);
        write_f32(&out.join(&member.traj), positions)?;

        let dec = decimate(positions, 1000);
        preview_counts.push(dec.len() as u32);
        preview_points.extend(dec);
    }

    let preview_rel = format!("{}/{dir}/preview.f32", combo.id);
    write_f32(&out.join(&preview_rel), &preview_points)?;

    let members: Vec<MemberOut> = kept.into_iter().map(|(m, _)| m).collect();
    Ok(Outcome {
        family: Some(FamilyOut {
            resonance_n: res.revs,
            closures: res.closures,
            members,
            preview: preview_rel,
            preview_counts,
        }),
        seed: Some(seed_record),
        absent: None,
    })
}

/// Apply the run's cache updates: store every converged first member whose numbers
/// actually moved, and merge the freshly measured absences into each combo's
/// `absent.json` (clearing any family that converged this time).
///
/// Records that describe the same orbit as the one already on disk are skipped
/// rather than rewritten. `generated_by` changes on every commit, so rewriting
/// unconditionally would put every seed file in the diff of every regeneration and
/// make "which seeds actually changed?" unanswerable from the git log.
fn write_cache_updates(
    cfg: &CatalogConfig,
    opts: &GenOptions,
    seeds: &[SeedRecord],
    absences: &[(usize, Resonance, String)],
    present: &[(usize, Resonance)],
) -> anyhow::Result<()> {
    let mut written = 0usize;
    for rec in seeds {
        let res = Resonance { revs: rec.revs, closures: rec.closures };
        match opts.seeds.load(&rec.combo_id, res) {
            Some(old) if old.same_orbit(rec) => continue,
            _ => {
                opts.seeds.store(rec)?;
                written += 1;
            }
        }
    }
    for (ci, combo) in cfg.combos.iter().enumerate() {
        let newly_absent: Vec<(Resonance, String)> = absences
            .iter()
            .filter(|(i, _, _)| *i == ci)
            .map(|(_, r, note)| (*r, note.clone()))
            .collect();
        let converged: Vec<Resonance> =
            present.iter().filter(|(i, _)| *i == ci).map(|(_, r)| *r).collect();
        if newly_absent.is_empty() && converged.is_empty() {
            continue;
        }
        opts.seeds.update_absences(&combo.id, &newly_absent, &converged)?;
    }
    eprintln!(
        "seed cache: {written} seed file(s) written or updated under {}",
        opts.seeds.root().display()
    );
    Ok(())
}

/// Sample one member's trajectory at `100*M` uniform times over its own
/// period (t=0 plus `j*T/(100M)` for `j = 1..100M-1`), compute its elements
/// at the minimum-radius sample, stability indices, and r_peri/r_apo from
/// the sampled radii. Returns the `MemberOut` (index/traj left as
/// placeholders, filled in by the caller once QA has decided the final
/// order) and its trajectory positions in km.
///
/// The period is the *full* `k`-closure period, so 100 samples per rev holds for
/// `M:k` families exactly as it does for `N:1`: an `M:2` member gets `100M`
/// samples spread over two node periods.
fn build_member(fm: &ForceModel, orbit: &PeriodicOrbit, res: Resonance) -> (MemberOut, Vec<[f64; 3]>) {
    let total = 100 * res.revs as usize;
    let period = orbit.period;
    let integ = Dp54::default();
    let f = |_t: f64, y: &[f64]| fm.eom(&[y[0], y[1], y[2], y[3], y[4], y[5]]).to_vec();
    let times: Vec<f64> = (1..total)
        .map(|k| period * k as f64 / total as f64)
        .collect();

    let mut states: Vec<[f64; 6]> = Vec::with_capacity(total);
    let mut ts: Vec<f64> = Vec::with_capacity(total);
    states.push(orbit.nodes[0]);
    ts.push(0.0);
    integ.propagate(&f, &orbit.nodes[0], 0.0, period, &times, &mut |t, y| {
        let mut s = [0.0; 6];
        s.copy_from_slice(&y[..6]);
        states.push(s);
        ts.push(t);
    });
    debug_assert_eq!(states.len(), total);

    let positions_km: Vec<[f64; 3]> = states
        .iter()
        .map(|s| [nd_to_km(s[0]), nd_to_km(s[1]), nd_to_km(s[2])])
        .collect();

    let (min_idx, _) = states
        .iter()
        .enumerate()
        .min_by(|a, b| {
            let ra = a.1[0] * a.1[0] + a.1[1] * a.1[1] + a.1[2] * a.1[2];
            let rb = b.1[0] * b.1[0] + b.1[1] * b.1[1] + b.1[2] * b.1[2];
            ra.partial_cmp(&rb).unwrap()
        })
        .expect("at least one sample");

    let sk = states[min_idx];
    let tk = ts[min_idx];
    let (ri, vi) = rotating_to_inertial(&[sk[0], sk[1], sk[2]], &[sk[3], sk[4], sk[5]], tk);
    let coe = rv_to_coe(&ri, &vi, MU_MOON_ND);
    let elements = ElementsOut {
        a_km: nd_to_km(coe.a),
        e: coe.e,
        i_deg: coe.i.to_degrees(),
        omega_deg: coe.aop.to_degrees(),
        raan_deg: coe.raan.to_degrees(),
    };

    let radii_km: Vec<f64> = positions_km
        .iter()
        .map(|p| (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt())
        .collect();
    let r_peri_km = radii_km.iter().cloned().fold(f64::MAX, f64::min);
    let r_apo_km = radii_km.iter().cloned().fold(f64::MIN, f64::max);

    let (nu1, nu2) = stability_indices(&monodromy(orbit));

    let member = MemberOut {
        index: 0,
        state0: orbit.nodes[0],
        period_s: nd_to_s(orbit.period),
        period_nd: orbit.period,
        elements,
        nu1,
        nu2,
        r_peri_km,
        r_apo_km,
        residual: orbit.residual,
        traj: String::new(),
    };
    (member, positions_km)
}

/// Short git commit hash for catalog provenance, `"unknown"` if `git` is
/// unavailable or this isn't a git checkout.
fn git_hash() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_full_combo_gets_rational_families() {
        let n25 = Resonance { revs: 25, closures: 1 };
        let d149 = Resonance { revs: 149, closures: 2 };
        // classical families: every combo, as before
        for id in ["full", "no-c22", "no-j3", "no-earth"] {
            assert!(generates(id, n25), "{id} must still generate N:1 families");
        }
        assert!(generates("full", d149));
        for id in ["no-c22", "no-j3", "no-earth"] {
            assert!(!generates(id, d149), "{id} must skip the heavy M:2 families");
        }
    }
}
