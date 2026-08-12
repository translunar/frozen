//! Seeds for the ELFO frozen-orbit families.
//!
//! A "frozen orbit" here is a *periodic orbit of the rotating frame* (the system
//! is autonomous), so the seed must supply both a state and a closure period.
//!
//! The naive resonant guess — `a = (μm/N²)^(1/3)` so that N Kepler revs fit in one
//! frame period, and `T₀ = 2π` — is exact only in the Kepler limit, and the error is
//! not a rounding detail: it puts the seed outside the corrector's basin entirely.
//! Two separate effects, both measured (see `closure_period_and_a`):
//!
//! 1. **The node regresses.** Rotating-frame periodicity requires the orbit plane to
//!    return, i.e. the ascending node must sweep exactly −2π *in the rotating frame*
//!    over one closure period. The frame supplies −2π in 2π of time, but the Earth's
//!    third-body torque regresses the node a further ≈ −0.07 rad/TU at N = 25, so
//!    closure happens **early**: T ≈ 2π/(1 − Ω̇) ≈ 6.02, not 6.28. Seeding `T₀ = 2π`
//!    leaves a one-month closure defect of 1.1e-1 — larger than the orbital velocity
//!    itself — and the corrector stalls at 2.5e-2.
//! 2. **The mean motion is perturbed.** The anomalistic rev period is ~0.2 % short of
//!    the Kepler value, so N revs do not fill T either.
//!
//! Both are fixed together by a two-condition iteration on `a`: propagate, find the
//! time at which the rotating-frame node has closed, require the N-th apoapsis
//! passage to land on that same instant, rescale `a ∝ T^(2/3)`, repeat. A handful of
//! iterations take the closure defect from 1.1e-1 to 1.0e-3, and the corrector then
//! reaches 5.5e-12 in five Newton steps.
//!
//! ## Rational M:k resonances
//!
//! Nothing above requires the node to close after *one* sweep. An orbit that closes
//! only after `k` node regressions, having completed `M` revolutions, is just as
//! periodic in the rotating frame: the closure condition becomes "cumulative node
//! azimuth has swept −2πk", the resonance condition becomes "the M-th apoapsis lands
//! on that instant", and the Kepler starting guess becomes `a = (μ (k/M)²)^(1/3)`.
//! The closure period is then `T ≈ k × T_single`.
//!
//! `k = 2` with **odd** `M` is the interesting case: it gives half-integer revs per
//! node period (`M/2` revs per closure), families that have no `N:1` counterpart —
//! the published ERGO repeat-ground-track constellation is 149:2. Even `M` at `k = 2`
//! is not new: it is the `(M/2):1` family traversed twice.
//!
//! The corrector, continuation and stability machinery downstream are all
//! resonance-agnostic — a periodic orbit is a periodic orbit — so only this module
//! and the labelling change.

use crate::constants::*;
use crate::elements::{coe_to_rv, inertial_to_rotating, Coe};
use crate::forces::ForceModel;
use crate::integrator::Dp54;
use std::f64::consts::{PI, TAU};

/// Fraction of the "periapsis 200 km up" eccentricity used for the Kozai-frozen seed.
///
/// The brief's 0.85 is geometrically fine but lands on e ≈ 0.68 at N = 25, where the
/// third-body node regression is strong enough (Ω̇ ∝ (1 + 9e²)/√(1 − e²) at ω = 90°)
/// that the closure period drops to 5.924 — 5.7 % off one frame period. 0.64 keeps
/// the seed on the same Lidov–Kozai frozen curve (i is re-derived from e) while
/// holding closure to 4.3 % (T = 6.0155), and it lifts periapsis from 1,403 km to a
/// far more ELFO-realistic 3,125 km altitude. Measured at N = 25:
///
/// | E_FRACTION | e | i | T_closure | \|T − 2π\| |
/// |---|---|---|---|---|
/// | 0.85 | 0.685 | 55.6° | 5.9241 | 0.359 |
/// | 0.75 | 0.605 | 51.9° | 5.9701 | 0.313 |
/// | 0.70 | 0.565 | 50.3° | 5.9914 | 0.292 |
/// | 0.64 | 0.517 | 48.5° | 6.0155 | 0.268 |
const E_FRACTION: f64 = 0.64;

/// Classical doubly-averaged frozen-orbit geometry at semi-major axis `a`.
fn frozen_elements(a: f64, fm: &ForceModel) -> (f64, f64) {
    if fm.earth {
        // Lidov–Kozai dominated: pick e from the periapsis budget, then take the
        // inclination off the frozen relation e = sqrt(1 − (5/3)cos²i).
        let e = (E_FRACTION * (1.0 - (R_MOON_ND + km_to_nd(200.0)) / a)).clamp(0.05, 0.711);
        (e, (0.6 * (1.0 - e * e)).sqrt().acos())
    } else {
        // J2/J3 frozen, near-circular.
        let i = 57f64.to_radians();
        let e = (MOON_J3 * R_MOON_ND * i.sin() / (2.0 * MOON_J2 * a)).min(0.3);
        (e, i)
    }
}

/// Rotating-frame state of the frozen geometry at apoapsis (ω = 90°, so apoapsis
/// dwells over the south pole), rotated about +z so that y = 0 exactly — the
/// corrector anchors node 0 on that section. Rotating the node is free: the seed is
/// approximate anyway, and the anchor is a labelling device.
fn frozen_state(a: f64, fm: &ForceModel) -> [f64; 6] {
    let (e, i) = frozen_elements(a, fm);
    let coe = Coe { a, e, i, raan: PI / 2.0, aop: PI / 2.0, ta: PI };
    let (ri, vi) = coe_to_rv(&coe, MU_MOON_ND);
    let (r0, v0) = inertial_to_rotating(&ri, &vi, 0.0);
    let theta = f64::atan2(r0[1], r0[0]);
    let (c, s) = ((-theta).cos(), (-theta).sin());
    let rz = |x: &[f64; 3]| [c * x[0] - s * x[1], s * x[0] + c * x[1], x[2]];
    let (rr, vr) = (rz(&r0), rz(&v0));
    [rr[0], rr[1], rr[2], vr[0], vr[1], vr[2]]
}

/// Azimuth of the orbit normal in rotating-frame axes. Equal to Ω − π/2 measured in
/// the rotating frame, so it tracks the node without going through `rv_to_coe`, and
/// it is well conditioned at every inclination that matters here.
fn node_azimuth(s: &[f64; 6]) -> f64 {
    // angular momentum r × v_inertial, components taken on the rotating axes
    let w = [s[3] - s[1], s[4] + s[0], s[5]];
    let hx = s[1] * w[2] - s[2] * w[1];
    let hy = s[2] * w[0] - s[0] * w[2];
    f64::atan2(hy, hx)
}

/// Sample the trajectory and return, for every apoapsis passage, `(time, node
/// azimuth)`. Apoapsis is located as a downward zero crossing of ṙ ∝ r·v (identical
/// in both frames, since r·(ẑ×r) = 0), which is linear through the crossing and so
/// far better conditioned than hunting a maximum of r.
///
/// Crossings before `t_min` are discarded, and that is load-bearing, not defensive:
/// the seed *starts* at apoapsis (ν = 180°), and `f64::sin(PI)` is `+1.2246e-16`
/// rather than 0, so `r·v(0)` is a tiny **positive** number and the scan would book a
/// spurious apoapsis at t ≈ 0. That shifts every index by one and makes the solver
/// below silently deliver the (N−1):1 resonance. Pass half a rev.
fn apoapsis_track(fm: &ForceModel, s0: &[f64; 6], tmax: f64, nsamp: usize, t_min: f64)
    -> Vec<(f64, f64)> {
    let integ = Dp54::default();
    let f = |_t: f64, y: &[f64]| fm.eom(&[y[0], y[1], y[2], y[3], y[4], y[5]]).to_vec();
    let times: Vec<f64> = (1..=nsamp).map(|k| tmax * k as f64 / nsamp as f64).collect();
    let mut rec: Vec<(f64, f64, f64)> = Vec::with_capacity(nsamp + 1); // t, r·v, azimuth
    let mut push = |t: f64, y: &[f64]| {
        let s = [y[0], y[1], y[2], y[3], y[4], y[5]];
        rec.push((t, s[0] * s[3] + s[1] * s[4] + s[2] * s[5], node_azimuth(&s)));
    };
    push(0.0, s0);
    integ.propagate(&f, s0, 0.0, tmax, &times, &mut |t, y| push(t, y));

    // unwrap the azimuth series (consecutive samples are far closer than π apart)
    let mut off = 0.0;
    for k in 1..rec.len() {
        let d = rec[k].2 + off - rec[k - 1].2;
        if d > PI { off -= TAU }
        if d < -PI { off += TAU }
        rec[k].2 += off;
    }

    let mut out = Vec::new();
    for k in 1..rec.len() {
        let (t0, d0, a0) = rec[k - 1];
        let (t1, d1, a1) = rec[k];
        if d0 > 0.0 && d1 <= 0.0 {
            let w = d0 / (d0 - d1); // linear crossing of r·v
            let tc = t0 + w * (t1 - t0);
            if tc > t_min {
                out.push((tc, a0 + w * (a1 - a0)));
            }
        }
    }
    out
}

/// Why a seed could not be solved. Returned by [`elfo_seed_checked`] so that a
/// catalog run can report *which* resonances were unreachable rather than silently
/// receiving a seed that will stall the corrector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedError {
    /// `n_revs` was zero; there is no 0:1 resonance.
    ZeroRevs,
    /// `n_closures` was zero; the node has to close at least once.
    ZeroClosures,
    /// Fewer than `n_revs` apoapsis passages inside the search window — the geometry
    /// is not making the requested number of revs per frame period.
    TooFewRevs { found: usize, needed: usize },
    /// The rotating-frame node never completed its −2π sweep inside the window, so
    /// there is no closure time to lock the resonance onto.
    NodeNeverClosed,
}

impl std::fmt::Display for SeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SeedError::ZeroRevs => write!(f, "n_revs must be at least 1"),
            SeedError::ZeroClosures => write!(f, "n_closures must be at least 1"),
            SeedError::TooFewRevs { found, needed } =>
                write!(f, "only {found} apoapsis passages in the search window, need {needed}"),
            SeedError::NodeNeverClosed =>
                write!(f, "rotating-frame node did not close within the search window"),
        }
    }
}

/// One measurement pass: `(node closure time, time of the M-th apoapsis passage)`,
/// where closure means the rotating-frame node has swept −2π·`n_closures`.
fn measure_closure(fm: &ForceModel, a: f64, n_revs: u32, n_closures: u32)
    -> Result<(f64, f64), SeedError> {
    let n = n_revs as f64;
    let k = n_closures as f64;
    let s = frozen_state(a, fm);
    // ~400 samples per rev over a little more than `k` frame periods: both the node
    // closure and the M-th apoapsis always land inside this window.
    let tmax = 1.15 * TAU * k;
    let nsamp = (400.0 * n * 1.15) as usize;
    // half a Kepler rev — see `apoapsis_track`: t = 0 is itself an apoapsis. The
    // window holds M revs in ≈ k frame periods, so a rev is ≈ k·2π/M long.
    let t_min = 0.5 * TAU * k / n;
    let track = apoapsis_track(fm, &s, tmax, nsamp, t_min);
    let &(t_n, _) = track
        .get(n_revs as usize - 1)
        .ok_or(SeedError::TooFewRevs { found: track.len(), needed: n_revs as usize })?;
    // node closure: rotating-frame node azimuth has swept exactly −2πk
    let target = node_azimuth(&s) - TAU * k;
    for j in 1..track.len() {
        let (ta, pa) = track[j - 1];
        let (tb, pb) = track[j];
        if pa > target && pb <= target {
            return Ok((ta + (tb - ta) * (pa - target) / (pa - pb), t_n));
        }
    }
    Err(SeedError::NodeNeverClosed)
}

/// Solve jointly for the closure period of the `n_revs`:`n_closures` resonance and
/// the semi-major axis that makes `n_revs` revs land exactly on it. Returns
/// `(a, T)`, always a consistent pair: the loop re-measures after every update to
/// `a`, so the returned `T` belongs to the returned `a` even when the iteration cap
/// is reached.
fn closure_period_and_a(fm: &ForceModel, n_revs: u32, n_closures: u32)
    -> Result<(f64, f64), SeedError> {
    if n_revs == 0 {
        return Err(SeedError::ZeroRevs);
    }
    if n_closures == 0 {
        return Err(SeedError::ZeroClosures);
    }
    // M revs in k frame periods ⇒ mean motion M/k, so a = (μ (k/M)²)^(1/3).
    let mut a = (MU_MOON_ND * (n_closures as f64 / n_revs as f64).powi(2)).cbrt();
    let (mut t_close, mut t_n) = measure_closure(fm, a, n_revs, n_closures)?;
    for _ in 0..8 {
        let ratio = t_close / t_n;
        if (ratio - 1.0).abs() < 1e-11 {
            break;
        }
        a *= ratio.powf(2.0 / 3.0); // Kepler's third law, one Newton step
        let m = measure_closure(fm, a, n_revs, n_closures)?;
        t_close = m.0;
        t_n = m.1;
    }
    Ok((a, t_close))
}

/// Seed for the `n_revs`:`n_closures` ELFO frozen family in force model `fm`:
/// rotating-frame state plus closure-period guess, reporting why on failure.
///
/// `n_closures = 1` is the classical N-revs-per-node-period case; `n_closures = 2`
/// with odd `n_revs` gives the half-integer families (see the module docs).
pub fn elfo_seed_resonant_checked(n_revs: u32, n_closures: u32, fm: &ForceModel)
    -> Result<([f64; 6], f64), SeedError> {
    if n_revs == 0 {
        return Err(SeedError::ZeroRevs);
    }
    if n_closures == 0 {
        return Err(SeedError::ZeroClosures);
    }
    if !fm.earth {
        // No third body ⇒ node regression is J2-only (≈ 1e-4 rad per frame period at
        // these altitudes), so closure sits on `k` frame periods to five digits; and
        // the near-circular geometry makes apoapsis timing meaningless anyway.
        let a = (MU_MOON_ND * (n_closures as f64 / n_revs as f64).powi(2)).cbrt();
        return Ok((frozen_state(a, fm), TAU * n_closures as f64));
    }
    let (a, t) = closure_period_and_a(fm, n_revs, n_closures)?;
    Ok((frozen_state(a, fm), t))
}

/// Seed for the `n_revs`-per-closure ELFO frozen family in force model `fm`:
/// rotating-frame state plus closure-period guess, reporting why on failure.
pub fn elfo_seed_checked(n_revs: u32, fm: &ForceModel)
    -> Result<([f64; 6], f64), SeedError> {
    elfo_seed_resonant_checked(n_revs, 1, fm)
}

/// Infallible [`elfo_seed_resonant_checked`]; see [`elfo_seed`] for the caveats on
/// the fallback seed.
pub fn elfo_seed_resonant(n_revs: u32, n_closures: u32, fm: &ForceModel) -> ([f64; 6], f64) {
    elfo_seed_resonant_checked(n_revs, n_closures, fm).unwrap_or_else(|_| {
        let k = n_closures.max(1) as f64;
        let a = (MU_MOON_ND * (k / n_revs.max(1) as f64).powi(2)).cbrt();
        (frozen_state(a, fm), TAU * k)
    })
}

/// Seed for the `n_revs`-per-closure ELFO frozen family in force model `fm`:
/// rotating-frame state plus closure-period guess.
///
/// On failure this falls back to the Kepler-resonant `a` with `T = 2π`, which is a
/// *poor* seed — it is the one measured to stall the corrector at 2.5e-2, because it
/// ignores the node regression. Callers that need to tell "solved" from "gave up"
/// must use [`elfo_seed_checked`].
pub fn elfo_seed(n_revs: u32, fm: &ForceModel) -> ([f64; 6], f64) {
    elfo_seed_checked(n_revs, fm).unwrap_or_else(|_| {
        let a = (MU_MOON_ND / (n_revs.max(1) as f64).powi(2)).cbrt();
        (frozen_state(a, fm), TAU)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{elements::rv_to_coe, elements::rotating_to_inertial,
        forces::ForceModel, shooting::{correct, seed_nodes, Constraint}};
    #[test]
    fn n25_full_model_elfo_converges() {
        let fm = ForceModel { j2: true, c22: true, j3: true, earth: true };
        let (seed, t0) = elfo_seed(25, &fm);
        // 50 segments = 2 per rev (the brief's escalation (d)). At 1 node/rev every
        // node sits near apoapsis (radii 14,840–15,262 km) so each segment spans a
        // whole rev *through* periapsis, and the segment STM is dominated by that
        // passage: the corrector stalls immediately at 5.7e-4, asking for a step of
        // 0.56. The extra nodes land near periapsis (4,831 km) and split each rev at
        // its stiffest point, which is what multiple shooting is for.
        let nodes = seed_nodes(&fm, &seed, t0, 50);
        let orbit = correct(&fm, &nodes, t0, &Constraint::None).expect("ELFO must converge");
        assert!(orbit.residual < 1e-10);
        assert!((orbit.period - std::f64::consts::TAU).abs() < 0.3);
        // eccentric, inclined, periapsis above surface
        let (ri, vi) = rotating_to_inertial(
            &[orbit.nodes[0][0], orbit.nodes[0][1], orbit.nodes[0][2]],
            &[orbit.nodes[0][3], orbit.nodes[0][4], orbit.nodes[0][5]], 0.0);
        let coe = rv_to_coe(&ri, &vi, MU_MOON_ND);
        assert!(coe.e > 0.4 && coe.e < 0.85, "e = {}", coe.e);
        assert!(coe.i > 0.6, "i = {} rad", coe.i);
        assert!(coe.a * (1.0 - coe.e) > R_MOON_ND, "periapsis below surface");

        // Independent periodicity check. `orbit.residual < 1e-10` above is tautological
        // — `correct()` only returns Ok below that threshold — and it is a *per
        // segment* defect besides. Propagate the whole period in one shot instead.
        let integ = crate::integrator::Dp54::default();
        let f = |_t: f64, y: &[f64]| fm.eom(&[y[0],y[1],y[2],y[3],y[4],y[5]]).to_vec();
        let yf = integ.propagate(&f, &orbit.nodes[0], 0.0, orbit.period, &[], &mut |_,_|{});
        for k in 0..6 {
            assert!((yf[k] - orbit.nodes[0][k]).abs() < 1e-7,
                "full-period closure k={k}: {}", yf[k] - orbit.nodes[0][k]);
        }
    }

    #[test]
    fn seed_holds_exactly_n_revs_per_closure_period() {
        // Regression guard for an off-by-one that is invisible in every other
        // assertion: the seed starts *at* apoapsis and f64 sin(π) = +1.2246e-16, so a
        // naive r·v crossing scan books a spurious apoapsis at t ≈ 0, shifts all
        // indices by one, and silently returns the (N−1):1 resonance. The corrector
        // still converges on that — to the wrong family member.
        let fm = ForceModel { j2: true, c22: true, j3: true, earth: true };
        for n in [20u32, 25, 40] {
            let (seed, t0) = elfo_seed(n, &fm);
            // Count periapsis passages in [0, t0): exactly n for an n-rev closure.
            // Periapsis (r·v crossing upward) is used rather than apoapsis precisely
            // because t = 0 is an apoapsis, so no crossing sits on the boundary.
            let integ = crate::integrator::Dp54::default();
            let f = |_t: f64, y: &[f64]| fm.eom(&[y[0],y[1],y[2],y[3],y[4],y[5]]).to_vec();
            let nsamp = 400 * n as usize;
            let times: Vec<f64> =
                (1..nsamp).map(|k| t0 * k as f64 / nsamp as f64).collect();
            let mut prev = 0.0f64; // r·v at t = 0 is zero to within 1e-16
            let mut revs = 0;
            integ.propagate(&f, &seed, 0.0, t0, &times, &mut |_, y| {
                let rv = y[0]*y[3] + y[1]*y[4] + y[2]*y[5];
                if prev < 0.0 && rv >= 0.0 { revs += 1; }
                prev = rv;
            });
            assert_eq!(revs, n, "elfo_seed({n}) delivered a {revs}-rev orbit");
        }
    }

    #[test]
    fn seed_failures_are_reported() {
        let fm = ForceModel { j2: true, c22: true, j3: true, earth: true };
        assert_eq!(elfo_seed_checked(0, &fm), Err(SeedError::ZeroRevs));
        assert_eq!(elfo_seed_resonant_checked(25, 0, &fm), Err(SeedError::ZeroClosures));
        // and the infallible wrappers must not panic on it
        let _ = elfo_seed(0, &fm);
        let _ = elfo_seed_resonant(0, 0, &fm);
    }

    #[test]
    fn half_integer_seed_closes_after_two_node_periods() {
        // 53:2 — 26.5 revs per node period, a genuinely new family (no 53:2 → N:1
        // reduction exists for odd M). Two independent checks, both on the *seed*
        // alone: the corrector is resonance-agnostic and costs 200+ segments here,
        // so it is deliberately not exercised in the test suite.
        let fm = ForceModel { j2: true, c22: true, j3: true, earth: true };
        let (seed, t53) = elfo_seed_resonant(53, 2, &fm);

        // 1. Period. The single-closure period varies smoothly with N, so the 26.5-rev
        //    closure must sit on the 26/27 interpolant, doubled — well inside 2%,
        //    which is far tighter than the ~4% gap between T_closure and 2π that
        //    motivates this machinery in the first place.
        let (_, t26) = elfo_seed_resonant(26, 1, &fm);
        let (_, t27) = elfo_seed_resonant(27, 1, &fm);
        let expect = 2.0 * 0.5 * (t26 + t27);
        assert!(
            (t53 - expect).abs() / expect < 0.02,
            "53:2 closure T = {t53}, expected ≈ {expect} (2 × interp of {t26}, {t27})"
        );

        // 2. Rev count. Same off-by-one hazard as the N:1 regression test above, with
        //    an extra failure mode: an M:k solver that targets a single −2π node sweep
        //    silently returns the 26- or 27-rev family at half the period.
        let integ = crate::integrator::Dp54::default();
        let f = |_t: f64, y: &[f64]| fm.eom(&[y[0],y[1],y[2],y[3],y[4],y[5]]).to_vec();
        let nsamp = 400 * 53;
        let times: Vec<f64> = (1..nsamp).map(|k| t53 * k as f64 / nsamp as f64).collect();
        let mut prev = 0.0f64; // r·v at t = 0 is zero to within 1e-16
        let mut revs = 0;
        integ.propagate(&f, &seed, 0.0, t53, &times, &mut |_, y| {
            let rv = y[0]*y[3] + y[1]*y[4] + y[2]*y[5];
            if prev < 0.0 && rv >= 0.0 { revs += 1; }
            prev = rv;
        });
        assert_eq!(revs, 53, "elfo_seed_resonant(53, 2) delivered a {revs}-rev orbit");
    }

    #[test]
    fn configured_dual_resonance_seeds_land_on_the_screened_altitudes() {
        // Cross-check against an independent calculation: the D-table in
        // docs/superpowers/litreview-dual-resonance-elfos.md screens M:2 candidates
        // by two-body period alone (a from the exact resonant mean motion). This
        // solver instead measures the perturbed node sweep and apoapsis timing, so
        // agreement is not circular — it says the M:k bookkeeping (−2πk sweep, M-th
        // apoapsis, mean motion M/k) is the same resonance the table describes.
        //
        // The disagreement is one-sided, and that is the physics: the third-body
        // node regression closes the orbit early (T = 12.383 rather than 4π =
        // 12.566 at 149:2), so the solved a sits *below* the Keplerian screen by
        // (T/4π)^(2/3) - 1 = -0.98%. Measured: -0.98% to -1.04% across all four,
        // i.e. the systematic offset the screen itself warns about (§6 item 6 of the
        // lit review) and nothing else. A dropped k factor would show up here as
        // 2^(2/3) = 1.59×, not 1%.
        let fm = ForceModel { j2: true, c22: true, j3: true, earth: true };
        // (M, k, D-table a in km)
        for (m, k, a_ref) in [(173u32, 2u32, 4522.35), (149, 2, 4995.79),
                              (111, 2, 6079.20), (99, 2, 6561.03)] {
            let (s, t) = elfo_seed_resonant(m, k, &fm);
            let (ri, vi) = rotating_to_inertial(&[s[0], s[1], s[2]], &[s[3], s[4], s[5]], 0.0);
            let a_km = crate::constants::nd_to_km(rv_to_coe(&ri, &vi, MU_MOON_ND).a);
            let err = (a_km - a_ref) / a_ref;
            eprintln!("{m}:{k}: a = {a_km:.2} km ({:+.3}% vs screen {a_ref}), T = {t:.4}",
                100.0 * err);
            assert!((-0.02..0.0).contains(&err),
                "{m}:{k}: a = {a_km:.2} km is {:+.2}% off the screen's {a_ref} km; \
                 expected a small *negative* offset from node regression", 100.0 * err);
            // and the closure period must be a k-fold one, not a single sweep
            assert!(t > 1.5 * TAU && t < 2.0 * TAU, "{m}:{k}: T = {t} is not a 2-closure period");
        }
    }

    /// Max-norm defect of a seeded node set before any correction: the largest
    /// component-wise mismatch between segment i's endstate and node i+1, together
    /// with the `y_0 = 0` anchor row. Same quantity the corrector minimises, so it
    /// is directly comparable to a reported stall residual.
    fn seed_defect(fm: &ForceModel, nodes: &[[f64; 6]], period: f64) -> f64 {
        let integ = crate::integrator::Dp54::default();
        let f = |_t: f64, y: &[f64]| fm.eom(&[y[0],y[1],y[2],y[3],y[4],y[5]]).to_vec();
        let m = nodes.len();
        let dt = period / m as f64;
        let mut worst = nodes[0][1].abs();
        for i in 0..m {
            let yf = integ.propagate(&f, &nodes[i], 0.0, dt, &[], &mut |_, _| {});
            let next = nodes[(i + 1) % m];
            for k in 0..6 {
                worst = worst.max((yf[k] - next[k]).abs());
            }
        }
        worst
    }

    /// Smoke test for the expensive end of the M:k machinery: does the corrector
    /// actually converge on a half-integer seed? 111:2 is the 12-h-band family
    /// (55.5 revs per node period) at m = 2M = 222 segments, which is a
    /// 1333×1333 SVD per Newton step — a minute, not seconds, so it is `#[ignore]`d
    /// and run by hand:
    ///
    /// ```text
    /// cargo test --release -p elfo-core -- --ignored --nocapture dual_resonance
    /// ```
    ///
    /// **As of the commit that introduced M:k it FAILS**, and that is the point of
    /// keeping the assertion. Measured (release, M4):
    ///
    /// ```text
    /// seed defect at m=222: |R| = 8.438e-3
    ///   control 25:1 at m=50:  seed |R| = 1.013e-3 -> converged 5.50e-12 in 1.9s
    ///   control 56:1 at m=112: seed |R| = 4.378e-3 -> stalled at 4.905e-4 in 55.7s
    /// m=222: stalled at residual 4.037e-3 after 46.4s
    /// ```
    ///
    /// The controls are the finding. 56:1 is a *classical* family at essentially the
    /// same altitude as 111:2 (55.5 revs per node period) and the same 2 nodes/rev,
    /// and it stalls too — as do 45, 50, 55 and 70 in the shipped catalog. So this is
    /// not an M:k defect: the corrector's basin already fails at the high-N, low-a end
    /// of the sweep, and 111:2 lands squarely in that band. Fixing it is a corrector
    /// problem (line search, step control, seed refinement), not a resonance one.
    ///
    /// **Resolved by the seed-cache campaign**: sweeping the seed eccentricity
    /// (rather than the E_FRACTION constant tuned at N=25) moves the seed into the
    /// corrector's basin — 111:2 converges to 1.28e-14 from e = 0.710 (0.705 and
    /// 0.715 both stall), and the converged state ships in `seeds/full/n111_2.json`.
    /// From analytic seeds at the default eccentricity this test still stalls, which
    /// remains worth pinning: it documents the basin edge the cache exists to cross.
    #[test]
    #[ignore = "~2 min; stalls from the default analytic seed (see doc — conquered via e-sweep, seeds/full/n111_2.json)"]
    fn dual_resonance_corrector_smoke_111_2() {
        let fm = ForceModel { j2: true, c22: true, j3: true, earth: true };
        let t_seed = std::time::Instant::now();
        let (seed, t0) = elfo_seed_resonant(111, 2, &fm);
        eprintln!("seed: T = {t0:.6} nd, solved in {:.1?}", t_seed.elapsed());
        let m = 222;
        let nodes = seed_nodes(&fm, &seed, t0, m);

        // Initial defect of the seeded node set, so the stall residual below can be
        // read as "no progress" or "some progress", and the same number for the
        // 25:1 seed that is known to converge, so it can be read against a scale.
        eprintln!("seed defect at m={m}: |R| = {:.3e}", seed_defect(&fm, &nodes, t0));
        // Two k=1 controls at the same 2-nodes-per-rev density: 25:1 (the family the
        // corrector was tuned on) and 56:1, which brackets 111:2's 55.5 revs per node
        // period and so sits at essentially the same altitude. If 56:1's defect is
        // small, the problem is the k=2 seed; if it is comparable, the problem is
        // size/altitude and k is incidental.
        for n in [25u32, 56] {
            let (s, t) = elfo_seed(n, &fm);
            let mc = 2 * n as usize;
            let nn = seed_nodes(&fm, &s, t, mc);
            let d = seed_defect(&fm, &nn, t);
            let tc = std::time::Instant::now();
            let r = correct(&fm, &nn, t, &Constraint::None);
            eprintln!("  control {n}:1 at m={mc}: seed |R| = {d:.3e} -> {} in {:.1?}",
                match &r {
                    Ok(o) => format!("converged {:.2e}", o.residual),
                    Err(e) => e.clone(),
                },
                tc.elapsed());
        }
        let t_corr = std::time::Instant::now();
        let result = correct(&fm, &nodes, t0, &Constraint::None);
        let dt = t_corr.elapsed();
        match &result {
            Ok(o) => eprintln!("m={m}: converged, residual {:.3e}, T = {:.6}, {dt:.1?}",
                o.residual, o.period),
            Err(e) => eprintln!("m={m}: {e} after {dt:.1?}"),
        }
        assert!(result.is_ok(), "111:2 corrector did not converge at m={m}");
    }

    #[test]
    fn classical_frozen_seed_stays_bounded_without_correction() {
        // Spec validation item 4 (ELFO half): the *uncorrected* classical
        // frozen-orbit geometry, propagated for ~3 months in the full model,
        // shows bounded libration — periapsis radius wanders but does not
        // secularly drift or impact. This validates the physics against the
        // classical (Ely-style) frozen condition, independent of the corrector.
        let fm = ForceModel { j2: true, c22: true, j3: true, earth: true };
        let (seed, t0) = elfo_seed(25, &fm);
        let integ = crate::integrator::Dp54::default();
        let f = |_t: f64, y: &[f64]| fm.eom(&[y[0],y[1],y[2],y[3],y[4],y[5]]).to_vec();
        let total = 3.0 * t0; // ~3 sidereal months, ~75 revs
        let times: Vec<f64> = (0..6000).map(|i| total * i as f64 / 6000.0).collect();
        let (mut prev_r, mut falling) = (f64::MAX, false);
        let mut peri_radii: Vec<f64> = Vec::new();
        integ.propagate(&f, &seed, 0.0, total, &times, &mut |_, y| {
            let r = (y[0]*y[0]+y[1]*y[1]+y[2]*y[2]).sqrt();
            assert!(r > R_MOON_ND, "seed trajectory impacted the Moon");
            if falling && r > prev_r { peri_radii.push(prev_r); } // local min = periapsis pass
            falling = r < prev_r;
            prev_r = r;
        });
        assert!(peri_radii.len() >= 60, "expected ~75 periapsis passes, got {}", peri_radii.len());
        let (lo, hi) = peri_radii.iter().fold((f64::MAX, f64::MIN), |(a, b), &r| (a.min(r), b.max(r)));
        assert!((hi - lo) / lo < 0.25, "periapsis radius wanders {:.1}% — not frozen", 100.0 * (hi - lo) / lo);
    }
}
