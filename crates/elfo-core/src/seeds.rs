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
            SeedError::TooFewRevs { found, needed } =>
                write!(f, "only {found} apoapsis passages in the search window, need {needed}"),
            SeedError::NodeNeverClosed =>
                write!(f, "rotating-frame node did not close within the search window"),
        }
    }
}

/// One measurement pass: `(node closure time, time of the n-th apoapsis passage)`.
fn measure_closure(fm: &ForceModel, a: f64, n_revs: u32) -> Result<(f64, f64), SeedError> {
    let n = n_revs as f64;
    let s = frozen_state(a, fm);
    // ~400 samples per rev over a little more than one frame period: both the node
    // closure and the N-th apoapsis always land inside this window.
    let tmax = 1.15 * TAU;
    let nsamp = (400.0 * n * 1.15) as usize;
    // half a Kepler rev — see `apoapsis_track`: t = 0 is itself an apoapsis
    let t_min = 0.5 * TAU / n;
    let track = apoapsis_track(fm, &s, tmax, nsamp, t_min);
    let &(t_n, _) = track
        .get(n_revs as usize - 1)
        .ok_or(SeedError::TooFewRevs { found: track.len(), needed: n_revs as usize })?;
    // node closure: rotating-frame node azimuth has swept exactly −2π
    let target = node_azimuth(&s) - TAU;
    for k in 1..track.len() {
        let (ta, pa) = track[k - 1];
        let (tb, pb) = track[k];
        if pa > target && pb <= target {
            return Ok((ta + (tb - ta) * (pa - target) / (pa - pb), t_n));
        }
    }
    Err(SeedError::NodeNeverClosed)
}

/// Solve jointly for the closure period and the semi-major axis that makes `n_revs`
/// revs land exactly on it. Returns `(a, T)`, always a consistent pair: the loop
/// re-measures after every update to `a`, so the returned `T` belongs to the
/// returned `a` even when the iteration cap is reached.
fn closure_period_and_a(fm: &ForceModel, n_revs: u32) -> Result<(f64, f64), SeedError> {
    if n_revs == 0 {
        return Err(SeedError::ZeroRevs);
    }
    let n = n_revs as f64;
    let mut a = (MU_MOON_ND / (n * n)).cbrt();
    let (mut t_close, mut t_n) = measure_closure(fm, a, n_revs)?;
    for _ in 0..8 {
        let ratio = t_close / t_n;
        if (ratio - 1.0).abs() < 1e-11 {
            break;
        }
        a *= ratio.powf(2.0 / 3.0); // Kepler's third law, one Newton step
        let m = measure_closure(fm, a, n_revs)?;
        t_close = m.0;
        t_n = m.1;
    }
    Ok((a, t_close))
}

/// Seed for the `n_revs`-per-closure ELFO frozen family in force model `fm`:
/// rotating-frame state plus closure-period guess, reporting why on failure.
pub fn elfo_seed_checked(n_revs: u32, fm: &ForceModel)
    -> Result<([f64; 6], f64), SeedError> {
    if n_revs == 0 {
        return Err(SeedError::ZeroRevs);
    }
    if !fm.earth {
        // No third body ⇒ node regression is J2-only (≈ 1e-4 rad per frame period at
        // these altitudes), so closure sits on the frame period to five digits; and
        // the near-circular geometry makes apoapsis timing meaningless anyway.
        let a = (MU_MOON_ND / (n_revs as f64).powi(2)).cbrt();
        return Ok((frozen_state(a, fm), TAU));
    }
    let (a, t) = closure_period_and_a(fm, n_revs)?;
    Ok((frozen_state(a, fm), t))
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
        // and the infallible wrapper must not panic on it
        let _ = elfo_seed(0, &fm);
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
