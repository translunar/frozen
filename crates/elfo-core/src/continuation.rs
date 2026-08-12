use nalgebra::DVector;
use crate::constants::R_MOON_ND;
use crate::forces::ForceModel;
use crate::shooting::{build_system, correct, pack, unpack, Constraint, PeriodicOrbit};

pub fn tangent(fm: &ForceModel, orbit: &PeriodicOrbit) -> DVector<f64> {
    let m = orbit.nodes.len();
    let u = pack(&orbit.nodes, orbit.period);
    let (_r, j, _s) = build_system(fm, &u, m, &Constraint::None);
    let svd = j.svd(true, true);
    let vt = svd.v_t.expect("svd");
    let last = vt.nrows() - 1; // right-singular vector for smallest σ
    let t = vt.row(last).transpose();
    &t / t.norm()
}

pub fn continue_family(fm: &ForceModel, first: PeriodicOrbit, steps: usize,
                       ds0: f64, direction: f64) -> Vec<PeriodicOrbit> {
    let m = first.nodes.len();
    let mut members = vec![first];
    let mut ds = ds0;
    let mut t_prev: Option<DVector<f64>> = None;
    while members.len() < steps + 1 {
        let cur = members.last().unwrap();
        let mut t = tangent(fm, cur);
        if let Some(tp) = &t_prev { if t.dot(tp) < 0.0 { t = -t; } }
        else { t *= direction; }
        let u_prev = pack(&cur.nodes, cur.period);
        let u_pred = &u_prev + &t * ds;
        let (nodes_pred, period_pred) = unpack(&u_pred, m);
        let constraint = Constraint::Arclength { tangent: t.clone(), prev: u_prev, ds };
        match correct(fm, &nodes_pred, period_pred, &constraint) {
            Ok(orbit) => {
                let impact = orbit.nodes.iter()
                    .any(|n| (n[0]*n[0]+n[1]*n[1]+n[2]*n[2]).sqrt() < R_MOON_ND);
                if impact { break; }
                t_prev = Some(t);
                members.push(orbit);
                ds = (ds * 1.3).min(4.0 * ds0);
            }
            Err(_) => {
                ds *= 0.5;
                if ds < ds0 / 64.0 { break; }
            }
        }
    }
    members
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{forces::ForceModel, lagrange::lyapunov_seed,
        shooting::{correct, seed_nodes, Constraint}};
    #[test]
    fn lyapunov_family_grows_monotonically() {
        let fm = ForceModel { j2: false, c22: false, j3: false, earth: true };
        let (s, t) = lyapunov_seed(1e-3);
        let first = correct(&fm, &seed_nodes(&fm, &s, t, 3), t, &Constraint::None).unwrap();
        let fam = continue_family(&fm, first, 8, 1e-3, 1.0);
        assert!(fam.len() >= 6, "only {} members", fam.len());
        let energies: Vec<f64> = fam.iter().map(|o| fm.energy(&o.nodes[0])).collect();
        let increasing = energies.windows(2).all(|w| w[1] > w[0]);
        let decreasing = energies.windows(2).all(|w| w[1] < w[0]);
        assert!(increasing || decreasing, "energy not monotone: {energies:?}");
        for o in &fam { assert!(o.residual < 1e-10); }
    }
}
