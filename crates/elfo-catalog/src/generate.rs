//! Orchestration for `elfo-catalog gen`: for every (combo, resonance) pair,
//! seed and correct the first family member, continue in both directions,
//! sample and QA every member, and write the on-disk catalog.

use std::path::Path;

use elfo_core::constants::{nd_to_km, nd_to_s, MU_MOON_ND};
use elfo_core::continuation::continue_family;
use elfo_core::elements::{rotating_to_inertial, rv_to_coe};
use elfo_core::forces::ForceModel;
use elfo_core::integrator::Dp54;
use elfo_core::seeds::elfo_seed_checked;
use elfo_core::shooting::{correct, seed_nodes, Constraint, PeriodicOrbit};
use elfo_core::stability::{monodromy, stability_indices};
use rayon::prelude::*;

use crate::config::{CatalogConfig, ComboCfg};
use crate::qa::check_member;
use crate::writer::{
    decimate, write_catalog, write_f32, ComboOut, ElementsOut, FamilyOut, MemberOut, Provenance,
};

/// Public entry point: load `config`, generate every family, write the
/// catalog (JSON + per-member/preview `.f32` trajectories) under `out`.
pub fn run(config: &Path, out: &Path) -> anyhow::Result<()> {
    let cfg = CatalogConfig::load(config)?;

    let pairs: Vec<(usize, u32)> = cfg
        .combos
        .iter()
        .enumerate()
        .flat_map(|(ci, _)| cfg.resonances.iter().map(move |&n| (ci, n)))
        .collect();

    let results: anyhow::Result<Vec<(usize, Option<FamilyOut>)>> = pairs
        .par_iter()
        .map(|&(ci, n)| -> anyhow::Result<(usize, Option<FamilyOut>)> {
            let combo = &cfg.combos[ci];
            let family = build_family(combo, n, &cfg, out)?;
            Ok((ci, family))
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
    for (ci, family) in results {
        if let Some(fam) = family {
            combos_out[ci].families.push(fam);
        }
    }
    for c in combos_out.iter_mut() {
        c.families.sort_by_key(|f| f.resonance_n);
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

/// Seed + correct the first member of the `n`-rev family in `combo`'s force
/// model, retrying once at `m = 4N` segments if `m = 2N` stalls. Returns
/// `None` (having logged why to stderr) if the family is absent — that is
/// data, not an error.
fn first_member(combo: &ComboCfg, n: u32) -> Option<PeriodicOrbit> {
    let fm = combo.force_model;
    let (seed, t0) = match elfo_seed_checked(n, &fm) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("absent: combo {} n={n}: seed unreachable: {e}", combo.id);
            return None;
        }
    };
    for &m in &[2 * n as usize, 4 * n as usize] {
        let nodes = seed_nodes(&fm, &seed, t0, m);
        if let Ok(orbit) = correct(&fm, &nodes, t0, &Constraint::None) {
            return Some(orbit);
        }
    }
    eprintln!(
        "absent: combo {} n={n}: corrector stalled at m=2N and m=4N",
        combo.id
    );
    None
}

/// Build one (combo, resonance) family end to end: seed/correct, continue in
/// both directions, sample + QA every member, write its files. `Ok(None)`
/// means the family is absent (combo present, family missing).
fn build_family(
    combo: &ComboCfg,
    n: u32,
    cfg: &CatalogConfig,
    out: &Path,
) -> anyhow::Result<Option<FamilyOut>> {
    let fm = combo.force_model;

    let Some(first) = first_member(combo, n) else {
        return Ok(None);
    };

    let first_pos = clone_orbit(&first);
    let pos = continue_family(&fm, first_pos, cfg.members_per_direction, cfg.ds0, 1.0);
    let neg = continue_family(&fm, first, cfg.members_per_direction, cfg.ds0, -1.0);

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
        candidates.push(build_member(&fm, orbit, n));
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
                "QA[{} n={n}]: {}{}",
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
        eprintln!("absent: combo {} n={n}: no member survived QA", combo.id);
        return Ok(None);
    }

    // Re-index 0..len and write per-member trajectory files plus the
    // decimated family preview.
    let mut preview_points: Vec<[f64; 3]> = Vec::new();
    let mut preview_counts: Vec<u32> = Vec::new();
    for (i, (member, positions)) in kept.iter_mut().enumerate() {
        member.index = i;
        member.traj = format!("{}/n{n}/{i}.f32", combo.id);
        write_f32(&out.join(&member.traj), positions)?;

        let dec = decimate(positions, 1000);
        preview_counts.push(dec.len() as u32);
        preview_points.extend(dec);
    }

    let preview_rel = format!("{}/n{n}/preview.f32", combo.id);
    write_f32(&out.join(&preview_rel), &preview_points)?;

    let members: Vec<MemberOut> = kept.into_iter().map(|(m, _)| m).collect();
    Ok(Some(FamilyOut {
        resonance_n: n,
        members,
        preview: preview_rel,
        preview_counts,
    }))
}

/// Sample one member's trajectory at `100*N` uniform times over its own
/// period (t=0 plus `k*T/(100N)` for `k = 1..100N-1`), compute its elements
/// at the minimum-radius sample, stability indices, and r_peri/r_apo from
/// the sampled radii. Returns the `MemberOut` (index/traj left as
/// placeholders, filled in by the caller once QA has decided the final
/// order) and its trajectory positions in km.
fn build_member(fm: &ForceModel, orbit: &PeriodicOrbit, n: u32) -> (MemberOut, Vec<[f64; 3]>) {
    let total = 100 * n as usize;
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
