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

        // Spec validation item 5 (monodromy structure) requires reciprocal
        // eigenvalue pairs *and* two unit eigenvalues, not just det = 1.
        // `stability_indices` assumes both properties rather than verifying
        // them — it unconditionally drops the two eigenvalues nearest 1+0i and
        // greedily pairs whatever remains — so a corrupted STM/segment product
        // could otherwise still produce two plausible-looking finite numbers.
        // Check both properties here, on a real monodromy, test-side only:
        // the resonant catalog members carry a *third* near-unity pair
        // (Poincaré-Birkhoff), so this is deliberately not a runtime
        // assertion inside `stability_indices` itself.
        let dm = DMatrix::from_iterator(6, 6, m_dro.iter().copied());
        let eig = dm.complex_eigenvalues();
        let mut evs: Vec<Complex<f64>> = eig.iter().copied().collect();
        let mut dropped = Vec::new();
        for _ in 0..2 {
            let (idx, _) = evs.iter().enumerate()
                .min_by(|a, b| (a.1 - 1.0).norm().partial_cmp(&(b.1 - 1.0).norm()).unwrap())
                .unwrap();
            dropped.push(evs.remove(idx));
        }
        // Measured on this DRO monodromy (4 segments, Dp54 at rtol=atol=1e-12):
        // the dropped pair sits at 1 ± 7.945e-6i — STM accumulation error over
        // 4 segment products, not a corruption — while the reciprocal-pair
        // residuals are ~2.5e-11 and ~7.6e-13, four to five orders tighter.
        // Tolerances reflect that asymmetry rather than reusing one number.
        for lam in &dropped {
            assert!((lam - Complex::new(1.0, 0.0)).norm() < 1e-5,
                "dropped eigenvalue not within tol of 1+0i: {lam}");
        }
        while evs.len() >= 2 {
            let l = evs.remove(0);
            let (j, _) = evs.iter().enumerate()
                .min_by(|a, b| (a.1 * l - 1.0).norm().partial_cmp(&(b.1 * l - 1.0).norm()).unwrap())
                .unwrap();
            let r = evs.remove(j);
            assert!((l * r - 1.0).norm() < 1e-6,
                "greedy pair not reciprocal: {l} * {r} = {}", l * r);
        }

        let (ls, tl) = lyapunov_seed(1e-3);
        let lyap = correct(&cr3bp(), &seed_nodes(&cr3bp(), &ls, tl, 3), tl,
            &Constraint::None).unwrap();
        let (l1, _) = stability_indices(&monodromy(&lyap));
        assert!(l1 > 1.5, "Lyapunov should be unstable, ν₁ = {l1}");
    }
}
