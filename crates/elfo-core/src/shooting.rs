use nalgebra::{DMatrix, DVector, SMatrix};
use crate::{forces::ForceModel, integrator::{Dp54, propagate_stm}};

pub struct PeriodicOrbit {
    pub nodes: Vec<[f64;6]>,
    pub period: f64,
    pub residual: f64,
    pub segment_stms: Vec<SMatrix<f64,6,6>>,
}

pub enum Constraint {
    None,
    Arclength { tangent: DVector<f64>, prev: DVector<f64>, ds: f64 },
}

pub fn pack(nodes: &[[f64;6]], period: f64) -> DVector<f64> {
    let m = nodes.len();
    let mut u = DVector::zeros(6*m + 1);
    for (i, n) in nodes.iter().enumerate() { for k in 0..6 { u[6*i+k] = n[k]; } }
    u[6*m] = period;
    u
}
pub fn unpack(u: &DVector<f64>, m: usize) -> (Vec<[f64;6]>, f64) {
    let mut nodes = vec![[0.0;6]; m];
    for i in 0..m { for k in 0..6 { nodes[i][k] = u[6*i+k]; } }
    (nodes, u[6*m])
}

pub fn seed_nodes(fm: &ForceModel, state0: &[f64;6], period: f64, m: usize) -> Vec<[f64;6]> {
    let integ = Dp54::default();
    let f = |_t: f64, y: &[f64]| fm.eom(&[y[0],y[1],y[2],y[3],y[4],y[5]]).to_vec();
    let times: Vec<f64> = (1..m).map(|i| period * i as f64 / m as f64).collect();
    let mut nodes = vec![*state0];
    integ.propagate(&f, state0, 0.0, period, &times, &mut |_, y| {
        let mut n = [0.0;6]; n.copy_from_slice(&y[..6]); nodes.push(n);
    });
    nodes.truncate(m);
    nodes
}

pub fn build_system(fm: &ForceModel, u: &DVector<f64>, m: usize, constraint: &Constraint)
    -> (DVector<f64>, DMatrix<f64>, Vec<SMatrix<f64,6,6>>) {
    let (nodes, period) = unpack(u, m);
    let tau = period / m as f64;
    let integ = Dp54::default();
    let nrows = 6*m + 1 + if matches!(constraint, Constraint::Arclength{..}) { 1 } else { 0 };
    let mut r = DVector::zeros(nrows);
    let mut j = DMatrix::zeros(nrows, 6*m + 1);
    let mut stms = Vec::with_capacity(m);
    for i in 0..m {
        let (endstate, phi) = propagate_stm(&integ, fm, &nodes[i], 0.0, tau);
        let fend = fm.eom(&endstate_arr(&endstate));
        let inext = (i + 1) % m;
        for row in 0..6 {
            r[6*i+row] = endstate[row] - nodes[inext][row];
            for col in 0..6 { j[(6*i+row, 6*i+col)] = phi[(row, col)]; }
            j[(6*i+row, 6*inext+row)] -= 1.0;
            j[(6*i+row, 6*m)] = fend[row] / m as f64;
        }
        stms.push(phi);
    }
    r[6*m] = nodes[0][1]; // anchor: y_0 = 0
    j[(6*m, 1)] = 1.0;
    if let Constraint::Arclength { tangent, prev, ds } = constraint {
        r[6*m+1] = tangent.dot(&(u - prev)) - ds;
        for col in 0..6*m+1 { j[(6*m+1, col)] = tangent[col]; }
    }
    (r, j, stms)
}
fn endstate_arr(y: &[f64;6]) -> [f64;6] { *y }

pub fn correct(fm: &ForceModel, nodes: &[[f64;6]], period: f64, constraint: &Constraint)
    -> Result<PeriodicOrbit, String> {
    let m = nodes.len();
    let mut u = pack(nodes, period);
    let (mut r, mut j, mut stms) = build_system(fm, &u, m, constraint);
    for _iter in 0..25 {
        let rn = r.amax();
        if rn < 1e-10 {
            let (nodes, period) = unpack(&u, m);
            return Ok(PeriodicOrbit { nodes, period, residual: rn, segment_stms: stms });
        }
        // Relative, not absolute, singular-value cutoff. A periodic-orbit system is
        // intrinsically rank-deficient: the family tangent is a genuine null
        // direction, and the min-norm solve is only well posed if it is truncated.
        // nalgebra's `eps` is an *absolute* threshold, but the scale of this system
        // is set by the dynamics — σ_max runs from 1e2 to 9e3 across the ELFO
        // resonances, driven by how close a node sits to periapsis — so no fixed
        // number can separate the null mode from the real ones.
        //
        // Measured on the N=25 full-model ELFO with the old 1e-11: the corrector
        // limps to |R| = 3.7e-8 and then stalls with σ_min = 1.674e-11 — barely
        // *above* the cutoff, so the null mode is retained and the solve divides a
        // noise-level residual projection by it, asking for a step of 2.1e-4 against
        // a residual of 3.7e-8 (cond 4.7e14). Nothing the line search can accept.
        //
        // 1e-10·σ_max was chosen by sweep: it converges the ELFO seeds for N = 20…60
        // and every pre-existing orbit, where 1e-8 over-truncates real modes (N ≥ 30
        // stalls at ~1e-7) and 1e-11 is too tight again by N = 60.
        let svd = j.clone().svd(true, true);
        let eps = 1e-10 * svd.singular_values.max();
        let du = svd.solve(&(-&r), eps).map_err(|e| e.to_string())?;
        let mut alpha = 1.0;
        let mut accepted = false;
        for _ in 0..7 {
            let ut = &u + &du * alpha;
            let (rt, jt, st) = build_system(fm, &ut, m, constraint);
            if rt.amax() < rn {
                u = ut; r = rt; j = jt; stms = st; accepted = true; break;
            }
            alpha *= 0.5;
        }
        if !accepted { return Err(format!("stalled at residual {rn}")); }
    }
    Err("max iterations".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{constants::*, forces::ForceModel};
    #[test]
    fn dro_converges_from_circular_seed() {
        let fm = ForceModel { j2: false, c22: false, j3: false, earth: true };
        let x0 = 0.04;
        let vc = (MU_MOON_ND / x0).sqrt();
        let seed = [x0, 0.0, 0.0, 0.0, -(vc + x0), 0.0]; // retrograde in rotating frame
        let t0 = std::f64::consts::TAU / (vc / x0 + 1.0);
        let nodes = seed_nodes(&fm, &seed, t0, 4);
        let orbit = correct(&fm, &nodes, t0, &Constraint::None).expect("DRO should converge");
        assert!(orbit.residual < 1e-10);
        assert!((orbit.period - t0).abs() < 0.2 * t0);
        // periodicity double-check by direct propagation
        let integ = crate::integrator::Dp54::default();
        let f = |_t: f64, y: &[f64]| fm.eom(&[y[0],y[1],y[2],y[3],y[4],y[5]]).to_vec();
        let yf = integ.propagate(&f, &orbit.nodes[0], 0.0, orbit.period, &[], &mut |_,_|{});
        for k in 0..6 { assert!((yf[k] - orbit.nodes[0][k]).abs() < 1e-8, "k={k}"); }
    }
}
