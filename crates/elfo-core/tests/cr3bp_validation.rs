use elfo_core::{forces::ForceModel, lagrange::lyapunov_seed,
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

#[test]
fn nrho_92_class_orbit_converges_near_published_seed() {
    // Approximate 9:2 L2 southern NRHO (JPL catalog vicinity), barycentric rotating
    // → Moon-centered by subtracting (1−μ). Loose tolerances absorb seed imprecision.
    let seed = [1.02134 - (1.0 - MU), 0.0, -0.18162, 0.0, -0.10176, 0.0];
    let t0 = 1.5092; // ≈ 6.56 days: 9 revs per 2 synodic months
    let nodes = seed_nodes(&cr3bp(), &seed, t0, 6);
    let orbit = correct(&cr3bp(), &nodes, t0, &Constraint::None).expect("converge");
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
