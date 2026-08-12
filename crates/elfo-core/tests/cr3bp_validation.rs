use elfo_core::{forces::ForceModel, integrator::{Dp54, propagate_stm}, lagrange::lyapunov_seed,
    shooting::{correct, seed_nodes, Constraint}, constants::*};

fn cr3bp() -> ForceModel { ForceModel { j2: false, c22: false, j3: false, earth: true } }

#[test]
fn l1_lyapunov_converges_from_linear_seed() {
    let (seed, t_lin) = lyapunov_seed(1e-3);
    let nodes = seed_nodes(&cr3bp(), &seed, t_lin, 3);
    let orbit = correct(&cr3bp(), &nodes, t_lin, &Constraint::None).expect("converge");
    assert!(orbit.residual < 1e-10);
    assert!((orbit.period - t_lin).abs() < 0.1 * t_lin);
}

/// First y = 0 crossing after t = 0: bracketed by a coarse scan (so we cannot
/// converge onto the wrong crossing), then Newton on y(t).
fn first_crossing(fm: &ForceModel, s: &[f64;6], t_max: f64) -> f64 {
    let integ = Dp54::default();
    let f = |_t: f64, y: &[f64]| fm.eom(&[y[0],y[1],y[2],y[3],y[4],y[5]]).to_vec();
    let n = 400;
    let times: Vec<f64> = (1..=n).map(|i| t_max * i as f64 / n as f64).collect();
    let (mut prev_y, mut bracket) = (s[1], None);
    integ.propagate(&f, s, 0.0, t_max, &times, &mut |t, y| {
        if bracket.is_none() && prev_y * y[1] < 0.0 { bracket = Some(t); }
        prev_y = y[1];
    });
    let mut t = bracket.expect("no y = 0 crossing within the search window");
    for _ in 0..50 {
        let y = integ.propagate(&f, s, 0.0, t, &[], &mut |_,_|{});
        let dt = -y[1] / y[4];
        t += dt;
        if dt.abs() < 1e-14 { break; }
    }
    t
}

/// Classic half-period differential correction for a symmetric orbit seeded at a
/// perpendicular crossing (x, 0, z, 0, vy, 0): Newton on (x, vy) drives vx and vz
/// to zero at the next y = 0 crossing. Returns the corrected state and the full
/// period, twice the crossing time.
///
/// This is seed preparation, not library machinery. It matters because the
/// catalog *period* is the fragile input: seed_nodes puts a node near perilune,
/// where |dv/dt| ~ 140, so a 0.5% period error lands that node O(0.5) away from
/// the orbit — hopelessly outside the multiple-shooting Newton basin even though
/// the state itself is good to 1e-6.
fn refine_symmetric(fm: &ForceModel, seed: &[f64;6], t_guess: f64) -> ([f64;6], f64) {
    let integ = Dp54::default();
    let mut s = *seed;
    for _ in 0..25 {
        let th = first_crossing(fm, &s, t_guess);
        let (sc, phi) = propagate_stm(&integ, fm, &s, 0.0, th);
        if sc[3].abs().max(sc[5].abs()) < 1e-12 { return (s, 2.0 * th); }
        // Vary the crossing conditions at fixed y = 0: the crossing time shifts
        // with the perturbation, so subtract f * (dt) with dt = -Phi[1,:]/ydot.
        let fc = fm.eom(&sc);
        let a = |row: usize, col: usize| phi[(row, col)] - fc[row] / sc[4] * phi[(1, col)];
        let (a00, a01, a10, a11) = (a(3, 0), a(3, 4), a(5, 0), a(5, 4));
        let det = a00 * a11 - a01 * a10;
        assert!(det.abs() > 1e-12, "singular half-period correction");
        s[0] += (a11 * -sc[3] - a01 * -sc[5]) / det;
        s[4] += (a00 * -sc[5] - a10 * -sc[3]) / det;
    }
    panic!("half-period seed refinement did not converge");
}

#[test]
fn nrho_92_class_orbit_converges_near_published_seed() {
    // Approximate 9:2 L2 southern NRHO (JPL catalog vicinity), barycentric rotating
    // → Moon-centered by subtracting (1−μ). Loose tolerances absorb seed imprecision.
    let seed = [1.02134 - (1.0 - MU), 0.0, -0.18162, 0.0, -0.10176, 0.0];
    let t0 = 1.5092; // ≈ 6.56 days: 9 revs per 2 synodic months
    // Refine the approximate seed onto the family first; multiple shooting then
    // starts inside its basin. This fixes the period (1.5092 → 1.50207) while
    // barely moving the state (< 3e-6).
    let (state, period) = refine_symmetric(&cr3bp(), &seed, t0);
    assert!((period - t0).abs() < 0.05 * t0, "refinement stayed on the seed's orbit");
    let nodes = seed_nodes(&cr3bp(), &state, period, 6);
    let orbit = correct(&cr3bp(), &nodes, period, &Constraint::None).expect("converge");
    assert!(orbit.residual < 1e-10);
    assert!(orbit.period > 1.45 && orbit.period < 1.57);
    // perilune radius in the published ballpark (≈ 3,200 km): accept 1,800–8,000 km
    let integ = elfo_core::integrator::Dp54::default();
    let fm = cr3bp();
    let f = |_t: f64, y: &[f64]| fm.eom(&[y[0],y[1],y[2],y[3],y[4],y[5]]).to_vec();
    let mut rmin = f64::MAX;
    let times: Vec<f64> = (0..2000).map(|i| orbit.period * i as f64 / 2000.0).collect();
    integ.propagate(&f, &orbit.nodes[0], 0.0, orbit.period, &times, &mut |_, y| {
        rmin = rmin.min((y[0]*y[0]+y[1]*y[1]+y[2]*y[2]).sqrt());
    });
    let rmin_km = nd_to_km(rmin);
    assert!(rmin_km > 1800.0 && rmin_km < 8000.0, "perilune {rmin_km} km");
}
