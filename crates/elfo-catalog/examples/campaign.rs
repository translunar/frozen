//! Hard-family convergence campaign: drive the corrector into families whose
//! analytic seed sits outside its basin, and bank the result in the seed cache.
//!
//! Background: `docs/superpowers/dual-resonance-implementation-notes.md` measured
//! that the corrector's basin fails across the high-N / low-`a` end of the sweep —
//! N = 45, 50, 55, 70 and all four `M:2` entries are absent from the shipped
//! catalog, and not because of anything to do with rational resonances (the `k = 1`
//! control at the same altitude fails identically). What follows is not a corrector
//! rewrite; it is a way of *handing the existing corrector a better starting point*.
//!
//! Three techniques, escalating:
//!
//! (a) **Kepler rescale.** Take the converged first member of the nearest family
//!     that *does* exist, scale it to the target's semi-major axis (`r × s`,
//!     `v_inertial × s^(-1/2)`), and correct from there. A converged neighbour is a
//!     far better description of the target's geometry than the doubly-averaged
//!     frozen ansatz the analytic seed uses: it already carries the true
//!     eccentricity, the true node rate and the real short-period wobble.
//!
//! (b) **Ladder.** If one jump stalls, bridge it with intermediate resonances
//!     (`40 → 42 → 45`). Each rung is itself a legitimate periodic orbit, whether or
//!     not the catalog asks for it, so every step is a real warm start and not an
//!     interpolation.
//!
//! (c) **Levenberg damping** (`--damped`), for stalls that die at small residual.
//!     Deliberately implemented *here*, on top of elfo-core's public `build_system`,
//!     rather than inside `correct()`: the shipped corrector's behaviour must not
//!     change under the whole catalog just to rescue four families, and a damped
//!     step is a different algorithm, not a tuning of the same one.
//!
//! ```bash
//! cargo run --release -p elfo-catalog --example campaign -- \
//!     --combo full --target 45 --source 40 --write-seeds
//! cargo run --release -p elfo-catalog --example campaign -- \
//!     --combo full --target 50 --source 40 --ladder 45,50
//! ```

use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use elfo_catalog::config::{CatalogConfig, Resonance};
use elfo_catalog::seedcache::{SeedCache, SeedRecord, SEED_SCHEMA_VERSION};
use elfo_core::constants::{km_to_nd, nd_to_km, MU_MOON_ND, R_MOON_ND};
use elfo_core::elements::{coe_to_rv, inertial_to_rotating, rotating_to_inertial, rv_to_coe, Coe};
use elfo_core::forces::ForceModel;
use elfo_core::integrator::Dp54;
use elfo_core::seeds::elfo_seed_resonant_checked;
use elfo_core::shooting::{
    build_system, correct, pack, seed_nodes, unpack, Constraint, PeriodicOrbit,
};
use elfo_core::stability::{monodromy, stability_indices};
use nalgebra::DVector;
use std::f64::consts::PI;

#[derive(Parser)]
#[command(name = "campaign", about = "Conquer a hard ELFO family and bank its seed")]
struct Cli {
    /// Combo id from catalog.toml (`full`, `no-c22`, `no-j3`, `no-earth`).
    #[arg(long, default_value = "full")]
    combo: String,
    /// The family to conquer, in `Resonance` spelling: `45`, `149:2`.
    #[arg(long)]
    target: String,
    /// The converged family to start from. Omit to cold-start the target from its
    /// own analytic seed (i.e. reproduce what the generator already does).
    #[arg(long)]
    source: Option<String>,
    /// Intermediate resonances to bridge through, in order, ending at the target:
    /// `--ladder 42,45`. Each rung warm-starts from the previous one's solution.
    #[arg(long, value_delimiter = ',')]
    ladder: Vec<String>,
    /// Shooting segments for the final target. Defaults to `min(2M, 400)`.
    #[arg(long)]
    segments: Option<usize>,
    /// Use the Levenberg-damped corrector instead of the shipped one.
    #[arg(long)]
    damped: bool,
    /// Iteration cap for the damped corrector.
    #[arg(long, default_value_t = 300)]
    max_iters: usize,
    /// Rebuild the seed at this eccentricity fraction instead of using `seeds.rs`'s
    /// 0.64 (which was tuned at N = 25). Ignored when `--source` is given.
    #[arg(long)]
    e_fraction: Option<f64>,
    /// Store the conquered target in the seed cache.
    #[arg(long)]
    write_seeds: bool,
    #[arg(long, default_value = "catalog.toml")]
    config: PathBuf,
    /// Seed cache root. Defaults to `$ELFO_SEEDS_DIR` or the repo's `seeds/`.
    #[arg(long)]
    seeds: Option<PathBuf>,
}

/// Hard cap on shooting segments, mirroring `generate.rs`: the Jacobian SVD is
/// `(6m+1)³` per Newton step.
const MAX_SEGMENTS: usize = 400;

fn segments_for(res: Resonance, override_: Option<usize>) -> usize {
    override_.unwrap_or_else(|| (2 * res.revs as usize).min(MAX_SEGMENTS))
}

/// Rebuild the frozen seed state at the solver's own `a` but with a chosen
/// eccentricity fraction, replacing `seeds.rs`'s hard-wired `E_FRACTION = 0.64`.
///
/// That constant was chosen by a measured sweep — at N = 25. It sets `e` from the
/// periapsis-altitude budget and then takes `i` off the Lidov–Kozai frozen relation
/// `e = sqrt(1 − (5/3)cos²i)`, so varying it slides the seed *along* the frozen
/// curve rather than off it: every value here is an equally legitimate frozen
/// geometry, just at a different point of the family. Since the corrector's failure
/// on the hard families is a basin failure, and the family is a curve in exactly
/// this direction, sweeping the fraction is a search for a point of that curve that
/// happens to lie inside the basin.
///
/// The closure period is *not* re-solved. `T` depends on `e` through the node rate
/// (`Ω̇ ∝ (1 + 9e²)/√(1 − e²)` at ω = 90°), so an off-nominal fraction gets a
/// slightly inconsistent `T` — which the corrector is free to move, and does.
fn frozen_seed_at(a: f64, e_fraction: f64) -> [f64; 6] {
    let e = (e_fraction * (1.0 - (R_MOON_ND + km_to_nd(200.0)) / a)).clamp(0.05, 0.711);
    let i = (0.6 * (1.0 - e * e)).sqrt().acos();
    let coe = Coe { a, e, i, raan: PI / 2.0, aop: PI / 2.0, ta: PI };
    let (ri, vi) = coe_to_rv(&coe, MU_MOON_ND);
    let (r0, v0) = inertial_to_rotating(&ri, &vi, 0.0);
    let theta = f64::atan2(r0[1], r0[0]);
    let (c, s) = ((-theta).cos(), (-theta).sin());
    let rz = |x: &[f64; 3]| [c * x[0] - s * x[1], s * x[0] + c * x[1], x[2]];
    let (rr, vr) = (rz(&r0), rz(&v0));
    [rr[0], rr[1], rr[2], vr[0], vr[1], vr[2]]
}

/// Semi-major axis (nondimensional) the analytic seed solver lands on for `res`.
/// Used only as the *ratio* `a_target / a_source`, so the systematic ~1 % offset
/// between this and a Keplerian screen cancels.
fn seed_a(fm: &ForceModel, res: Resonance) -> Option<f64> {
    let (s, _) = elfo_seed_resonant_checked(res.revs, res.closures, fm).ok()?;
    let (ri, vi) = rotating_to_inertial(&[s[0], s[1], s[2]], &[s[3], s[4], s[5]], 0.0);
    Some(rv_to_coe(&ri, &vi, MU_MOON_ND).a)
}

/// Kepler-rescale a rotating-frame state to a semi-major axis `s` times larger.
///
/// The scaling law (`r × s`, `v × s^(-1/2)`) is a statement about the *inertial*
/// two-body problem, so the state is taken to the inertial frame first. Applying it
/// directly to the rotating velocity would leave the frame term `ω × r` scaled by
/// `s^(-1/2)` when it must scale by `s`, an error of the same order as the rescale
/// itself at these altitudes.
///
/// The result is then rotated about `+z` so that `y = 0`: the corrector anchors node
/// 0 on that section, and handing it a state off the section spends Newton steps
/// re-establishing an arbitrary labelling choice. (A converged source is already
/// anchored and the scaling preserves `y = 0`, so this is normally a no-op; it earns
/// its place for ladder rungs and hand-supplied states.)
fn kepler_rescale(state: &[f64; 6], s: f64) -> [f64; 6] {
    let (ri, vi) = rotating_to_inertial(&[state[0], state[1], state[2]], &[state[3], state[4], state[5]], 0.0);
    let sv = 1.0 / s.sqrt();
    let r2 = [ri[0] * s, ri[1] * s, ri[2] * s];
    let v2 = [vi[0] * sv, vi[1] * sv, vi[2] * sv];
    let (r0, v0) = inertial_to_rotating(&r2, &v2, 0.0);
    let theta = f64::atan2(r0[1], r0[0]);
    let (c, sn) = ((-theta).cos(), (-theta).sin());
    let rz = |x: &[f64; 3]| [c * x[0] - sn * x[1], sn * x[0] + c * x[1], x[2]];
    let (rr, vr) = (rz(&r0), rz(&v0));
    [rr[0], rr[1], rr[2], vr[0], vr[1], vr[2]]
}

/// Periapsis passages in `[0, T)`: the resonance label, verified rather than
/// assumed. Counts *periapsis* (upward `r·v` crossing) precisely because the seeds
/// sit at apoapsis, where `sin(π) = +1.2e-16` books a spurious crossing at `t = 0`
/// and shifts every index — the historic mislabelling bug in this codebase.
fn count_revs(fm: &ForceModel, s0: &[f64; 6], period: f64) -> usize {
    let integ = Dp54::default();
    let f = |_t: f64, y: &[f64]| fm.eom(&[y[0], y[1], y[2], y[3], y[4], y[5]]).to_vec();
    let nsamp = 400 * (period / 0.1).ceil().max(100.0) as usize;
    let times: Vec<f64> = (1..nsamp).map(|k| period * k as f64 / nsamp as f64).collect();
    let mut prev = s0[0] * s0[3] + s0[1] * s0[4] + s0[2] * s0[5];
    let mut revs = 0usize;
    integ.propagate(&f, s0, 0.0, period, &times, &mut |_, y| {
        let rv = y[0] * y[3] + y[1] * y[4] + y[2] * y[5];
        if prev < 0.0 && rv >= 0.0 {
            revs += 1;
        }
        prev = rv;
    });
    revs
}

/// Levenberg-damped multiple shooting: solve `(JᵀJ + λI) δ = −Jᵀ R` instead of the
/// pseudo-inverse step, shrinking λ on success and growing it on rejection.
///
/// Why this can succeed where the shipped corrector stalls: the shipped step is a
/// truncated-SVD min-norm solve followed by a backtracking line search along that
/// one direction. When the Jacobian is ill-conditioned the min-norm direction is
/// dominated by the near-null modes and is barely a descent direction at all, so
/// every one of the seven halvings fails and the corrector reports a stall while
/// still far from a solution. Damping *rotates* the step towards steepest descent
/// instead of merely shortening it.
///
/// It is strictly slower per iteration (a `(6m+1)²` normal-equation build plus a
/// Cholesky, against one SVD) and it is not used anywhere in the shipped generator.
///
/// **It hands off.** Once the residual is under `HANDOFF`, the shipped `correct()`
/// is tried from the current node set every iteration, and its answer is what gets
/// returned. That is not a nicety: `generate.rs` warm-starts from a cached seed by
/// running `correct()` on it, so a seed that only the damped corrector can reach is
/// a seed the generator cannot use. Requiring the shipped corrector to close the
/// last three orders of magnitude makes every banked seed re-convergeable by
/// definition.
const HANDOFF: f64 = 1e-5;

fn correct_damped(
    fm: &ForceModel,
    nodes: &[[f64; 6]],
    period: f64,
    max_iters: usize,
) -> Result<PeriodicOrbit, String> {
    let m = nodes.len();
    let mut u = pack(nodes, period);
    let (mut r, mut j, mut stms) = build_system(fm, &u, m, &Constraint::None);
    let mut lambda = 1e-6;
    let mut best = r.amax();
    // Handoff attempts are rationed: a failed `correct()` costs up to 25 SVDs, which
    // is more than the damped iteration that would have replaced it. Retry only
    // after another order of magnitude has been won, so the total spend is ~5 calls.
    let mut next_handoff = HANDOFF;
    for it in 0..max_iters {
        let rn = r.amax();
        best = best.min(rn);
        if it % 10 == 0 {
            // Where the max defect sits is the whole diagnosis: row `6i+c` is
            // component `c` of segment `i`'s closure, row `6m` is the `y_0 = 0`
            // anchor. A defect pinned to one segment is a resolution problem at that
            // point of the orbit; one that wanders is a step-quality problem.
            let (arg, _) = r.iter().enumerate().fold((0usize, 0.0f64), |(bi, bv), (i, &v)| {
                if v.abs() > bv { (i, v.abs()) } else { (bi, bv) }
            });
            let (seg, comp) = (arg / 6, arg % 6);
            eprintln!(
                "    iter {it}: |R|inf = {rn:.4e}, |R|2 = {:.4e}, lambda = {lambda:.2e}, \
                 T = {:.9}, argmax = {} ",
                r.norm(),
                u[6 * m],
                if arg == 6 * m { "anchor".to_string() } else { format!("seg {seg} comp {comp}") },
            );
        }
        if rn < 1e-10 {
            let (nodes, period) = unpack(&u, m);
            return Ok(PeriodicOrbit { nodes, period, residual: rn, segment_stms: stms });
        }
        if rn < next_handoff {
            next_handoff = rn * 0.1;
            let (n, p) = unpack(&u, m);
            eprintln!("    handoff attempt at iter {it}, |R| = {rn:.3e}");
            if let Ok(o) = correct(fm, &n, p, &Constraint::None) {
                eprintln!("    handoff: shipped corrector closed it to {:.3e}", o.residual);
                return Ok(o);
            }
        }
        let jt = j.transpose();
        let jtj = &jt * &j;
        let g: DVector<f64> = &jt * &r;
        // **Marquardt** scaling, not plain Levenberg: damp with `λ·diag(JᵀJ)`, not
        // `λ·I`. The columns of this Jacobian are not remotely commensurate — the
        // state columns carry STM entries running to 1e4, while the period column is
        // `f_end / m`, smaller by six orders — so a single scalar λ that is a mild
        // regularisation for the state block is an outright freeze on the period.
        // Measured on N = 45: `λ·I` crawls to a 1.4e-6 floor and stops; `λ·diag`
        // closes to 1e-11.
        let diag: Vec<f64> = {
            let d = jtj.diagonal();
            let floor = d.max() * 1e-12;
            d.iter().map(|&x| x.max(floor)).collect()
        };
        let mut accepted = false;
        // Acceptance is on the 2-norm, which is the objective the LM step actually
        // minimises. Convergence is still reported on the ∞-norm, because that is
        // what `correct()` and the catalog's QA bar are stated in. Accepting on the
        // ∞-norm while stepping on the 2-norm rejects perfectly good steps that trade
        // a spike at one node for a reduction everywhere else.
        let r2 = r.norm();
        for _ in 0..12 {
            let mut a = jtj.clone();
            for d in 0..a.nrows() {
                a[(d, d)] += lambda * diag[d];
            }
            let Some(chol) = a.cholesky() else {
                lambda *= 10.0;
                continue;
            };
            let du = chol.solve(&(-&g));
            let ut = &u + &du;
            let (rt, jjt, st) = build_system(fm, &ut, m, &Constraint::None);
            if rt.norm() < r2 {
                u = ut;
                r = rt;
                j = jjt;
                stms = st;
                lambda = (lambda * 0.1).max(1e-16);
                accepted = true;
                break;
            }
            lambda *= 10.0;
            if lambda > 1e12 {
                break;
            }
        }
        if !accepted {
            return Err(format!("damped: stalled at residual {rn} (lambda {lambda:.1e})"));
        }
    }
    let rn = r.amax();
    if rn < 1e-10 {
        let (nodes, period) = unpack(&u, m);
        return Ok(PeriodicOrbit { nodes, period, residual: rn, segment_stms: stms });
    }
    Err(format!("damped: max iterations at residual {rn} (best {best:.3e})"))
}

/// One rung: warm-start `to` from the converged state of `from`, and correct.
fn attempt(
    fm: &ForceModel,
    from: Option<(&[f64; 6], Resonance)>,
    to: Resonance,
    segments: usize,
    damped: bool,
    max_iters: usize,
    e_fraction: Option<f64>,
) -> Result<PeriodicOrbit, String> {
    // The closure period is *not* Kepler-scaled. It is the node-regression period
    // (~2π per closure at every one of these altitudes), not the orbital period, so
    // `T × s^(3/2)` would move it by 10 % when the truth moves by under 1 %. The
    // analytic seed solver measures it directly — that part of the seed was never
    // the problem — so the rescale supplies the state and the solver supplies T.
    let (analytic, t_target) = elfo_seed_resonant_checked(to.revs, to.closures, fm)
        .map_err(|e| format!("seed unreachable: {e}"))?;

    let state = match from {
        Some((src_state, src_res)) => {
            let a_s = seed_a(fm, src_res).ok_or("source seed unreachable")?;
            let a_t = seed_a(fm, to).ok_or("target seed unreachable")?;
            let s = a_t / a_s;
            eprintln!(
                "  rescale {src_res} -> {to}: a {:.1} km -> {:.1} km (s = {s:.6}), T = {t_target:.6}",
                nd_to_km(a_s),
                nd_to_km(a_t)
            );
            kepler_rescale(src_state, s)
        }
        None => match e_fraction {
            None => {
                eprintln!("  cold start {to}: analytic seed, T = {t_target:.6}");
                analytic
            }
            Some(ef) => {
                // The frozen relation this rebuilds is the Lidov–Kozai one, which
                // only describes the third-body-dominated regime. Without the Earth
                // term `seeds.rs` uses a near-circular J2/J3 frozen geometry
                // instead, and feeding it a Kozai state would be a different orbit
                // entirely rather than a different point of the same family.
                if !fm.earth {
                    return Err("--e-fraction is Lidov–Kozai; it needs the Earth term".into());
                }
                let a = seed_a(fm, to).ok_or("target seed unreachable")?;
                eprintln!(
                    "  cold start {to}: frozen seed at e_fraction {ef}, a = {:.1} km, \
                     T = {t_target:.6}",
                    nd_to_km(a)
                );
                frozen_seed_at(a, ef)
            }
        },
    };

    let nodes = seed_nodes(fm, &state, t_target, segments);
    // The corrector's own residual at iteration 0. Printed so a stall can be read as
    // "made no progress" or "made progress and then ran out of step", which is the
    // difference between a bad seed and a bad corrector.
    let r0 = build_system(fm, &pack(&nodes, t_target), segments, &Constraint::None).0.amax();
    eprintln!("  seed defect at m={segments}: |R| = {r0:.3e}");
    let t0 = Instant::now();
    let out = if damped {
        correct_damped(fm, &nodes, t_target, max_iters)
    } else {
        correct(fm, &nodes, t_target, &Constraint::None)
    };
    match &out {
        Ok(o) => eprintln!(
            "  m={segments}{}: converged, residual {:.3e}, T = {:.6}, {:.1?}",
            if damped { " damped" } else { "" },
            o.residual,
            o.period,
            t0.elapsed()
        ),
        Err(e) => eprintln!(
            "  m={segments}{}: {e} after {:.1?}",
            if damped { " damped" } else { "" },
            t0.elapsed()
        ),
    }
    out
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let cfg = CatalogConfig::load(&cli.config)?;
    let combo = cfg
        .combos
        .iter()
        .find(|c| c.id == cli.combo)
        .ok_or_else(|| anyhow::anyhow!("no combo {:?} in {}", cli.combo, cli.config.display()))?;
    let fm = combo.force_model;
    let cache = SeedCache::new(cli.seeds.clone().unwrap_or_else(SeedCache::default_root));

    let target: Resonance = cli.target.parse().map_err(anyhow::Error::msg)?;

    // Resolve the source: cached if we have it, cold-solved if not (which only
    // works for a family that already converges — that is what makes it a source).
    let source: Option<(Resonance, [f64; 6])> = match &cli.source {
        None => None,
        Some(s) => {
            let res: Resonance = s.parse().map_err(anyhow::Error::msg)?;
            let state = match cache.load(&combo.id, res) {
                Some(rec) => {
                    eprintln!("source {res}: cached (from {}), T = {:.6}", rec.generated_by, rec.period_nd);
                    rec.state0
                }
                None => {
                    eprintln!("source {res}: not cached, cold-solving it first");
                    let o = attempt(&fm, None, res, segments_for(res, None), false, cli.max_iters, None)
                        .map_err(|e| anyhow::anyhow!("source {res} does not converge: {e}"))?;
                    if cli.write_seeds {
                        cache.store(&SeedRecord {
                            schema_version: SEED_SCHEMA_VERSION,
                            combo_id: combo.id.clone(),
                            revs: res.revs,
                            closures: res.closures,
                            state0: o.nodes[0],
                            period_nd: o.period,
                            residual: o.residual,
                            generated_by: git_hash(),
                        })?;
                    }
                    o.nodes[0]
                }
            };
            Some((res, state))
        }
    };

    // The rungs to walk: the ladder if one was given (its last entry is the target),
    // otherwise a single jump straight to the target.
    let mut rungs: Vec<Resonance> = Vec::new();
    for s in &cli.ladder {
        rungs.push(s.parse().map_err(anyhow::Error::msg)?);
    }
    if rungs.last() != Some(&target) {
        rungs.push(target);
    }

    let mut cur: Option<(Resonance, [f64; 6])> = source;
    let mut solved: Option<PeriodicOrbit> = None;
    for (i, rung) in rungs.iter().enumerate() {
        let last = i + 1 == rungs.len();
        let segs = segments_for(*rung, if last { cli.segments } else { None });
        eprintln!("rung {}/{}: {} -> {rung}", i + 1, rungs.len(),
            cur.map(|(r, _)| r.to_string()).unwrap_or_else(|| "cold".into()));
        let from = cur.as_ref().map(|(r, s)| (s, *r));
        match attempt(&fm, from, *rung, segs, cli.damped, cli.max_iters, cli.e_fraction) {
            Ok(o) => {
                cur = Some((*rung, o.nodes[0]));
                solved = Some(o);
            }
            Err(e) => {
                println!("FAILED {} n={target}: rung {rung}: {e}", combo.id);
                return Ok(());
            }
        }
    }

    let orbit = solved.expect("at least one rung");

    // Verification. A converged periodic orbit is not automatically the family that
    // was asked for: the Kepler rescale moves the state to the target's altitude,
    // but the corrector will happily settle on whatever nearby orbit closes. Count
    // the revs, and require the stability indices to be finite (an unusable
    // monodromy would make the member uncatalogable even if it is periodic).
    let revs = count_revs(&fm, &orbit.nodes[0], orbit.period);
    let (nu1, nu2) = stability_indices(&monodromy(&orbit));
    let (ri, vi) = rotating_to_inertial(
        &[orbit.nodes[0][0], orbit.nodes[0][1], orbit.nodes[0][2]],
        &[orbit.nodes[0][3], orbit.nodes[0][4], orbit.nodes[0][5]],
        0.0,
    );
    let coe = rv_to_coe(&ri, &vi, MU_MOON_ND);
    eprintln!(
        "verify: revs in [0,T) = {revs} (want {}), nu1 = {nu1:.4}, nu2 = {nu2:.4}, \
         a = {:.1} km, e = {:.4}, i = {:.2}°",
        target.revs,
        nd_to_km(coe.a),
        coe.e,
        coe.i.to_degrees()
    );
    if revs != target.revs as usize {
        println!(
            "MISLABELLED {} n={target}: converged to a {revs}-rev orbit, residual {:.3e}",
            combo.id, orbit.residual
        );
        return Ok(());
    }
    if !nu1.is_finite() || !nu2.is_finite() {
        println!("UNSTABLE-INDICES {} n={target}: nu1 = {nu1}, nu2 = {nu2}", combo.id);
        return Ok(());
    }

    if cli.write_seeds {
        cache.store(&SeedRecord {
            schema_version: SEED_SCHEMA_VERSION,
            combo_id: combo.id.clone(),
            revs: target.revs,
            closures: target.closures,
            state0: orbit.nodes[0],
            period_nd: orbit.period,
            residual: orbit.residual,
            generated_by: git_hash(),
        })?;
        cache.update_absences(&combo.id, &[], &[target])?;
        eprintln!("wrote {}", cache.seed_path(&combo.id, target).display());
    }
    println!(
        "CONVERGED {} n={target}: residual {:.3e}, T = {:.6}, revs {revs}, nu1 {nu1:.4}",
        combo.id, orbit.residual, orbit.period
    );
    Ok(())
}

fn git_hash() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
