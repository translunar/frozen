use nalgebra::{Complex, DMatrix, SMatrix};
use crate::shooting::PeriodicOrbit;

pub fn monodromy(orbit: &PeriodicOrbit) -> SMatrix<f64,6,6> {
    let mut m = SMatrix::<f64,6,6>::identity();
    for phi in &orbit.segment_stms { m = phi * m; }
    m
}

pub fn stability_indices(mono: &SMatrix<f64,6,6>) -> (f64, f64) {
    let dm = DMatrix::from_iterator(6, 6, mono.iter().copied());
    let eig = dm.complex_eigenvalues();
    let mut evs: Vec<Complex<f64>> = eig.iter().copied().collect();
    // drop the two eigenvalues closest to 1+0i (phase + energy directions)
    for _ in 0..2 {
        let (idx, _) = evs.iter().enumerate()
            .min_by(|a, b| (a.1 - 1.0).norm().partial_cmp(&(b.1 - 1.0).norm()).unwrap())
            .unwrap();
        evs.remove(idx);
    }
    // greedy reciprocal pairing of the remaining four
    let mut nus = Vec::new();
    while evs.len() >= 2 {
        let l = evs.remove(0);
        let (j, _) = evs.iter().enumerate()
            .min_by(|a, b| (a.1 * l - 1.0).norm().partial_cmp(&(b.1 * l - 1.0).norm()).unwrap())
            .unwrap();
        let _ = evs.remove(j);
        nus.push(0.5 * (l + 1.0 / l).re);
    }
    nus.sort_by(|a, b| b.abs().partial_cmp(&a.abs()).unwrap());
    (nus[0], nus.get(1).copied().unwrap_or(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{forces::ForceModel, constants::*, lagrange::lyapunov_seed,
        shooting::{correct, seed_nodes, Constraint}};
    fn cr3bp() -> ForceModel { ForceModel { j2: false, c22: false, j3: false, earth: true } }

    #[test]
    fn monodromy_structure_and_index_signs() {
        // DRO: stable → both |ν| ≤ 1. L1 Lyapunov: unstable → ν₁ > 1.
        let x0 = 0.04; let vc = (MU_MOON_ND / x0).sqrt();
        let dro_seed = [x0, 0.0, 0.0, 0.0, -(vc + x0), 0.0];
        let t0 = std::f64::consts::TAU / (vc / x0 + 1.0);
        let dro = correct(&cr3bp(), &seed_nodes(&cr3bp(), &dro_seed, t0, 4), t0,
            &Constraint::None).unwrap();
        let m_dro = monodromy(&dro);
        assert!((m_dro.determinant() - 1.0).abs() < 1e-6, "det = {}", m_dro.determinant());
        let (n1, n2) = stability_indices(&m_dro);
        assert!(n1.abs() <= 1.05 && n2.abs() <= 1.05, "DRO ν = {n1}, {n2}");

        let (ls, tl) = lyapunov_seed(1e-3);
        let lyap = correct(&cr3bp(), &seed_nodes(&cr3bp(), &ls, tl, 3), tl,
            &Constraint::None).unwrap();
        let (l1, _) = stability_indices(&monodromy(&lyap));
        assert!(l1 > 1.5, "Lyapunov should be unstable, ν₁ = {l1}");
    }
}
