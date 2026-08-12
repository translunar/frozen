use crate::forces::ForceModel;

fn cr3bp() -> ForceModel { ForceModel { j2: false, c22: false, j3: false, earth: true } }

fn bisect_ax(mut lo: f64, mut hi: f64) -> f64 {
    let fm = cr3bp();
    let ax = |x: f64| fm.accel(&[x, 0.0, 0.0, 0.0, 0.0, 0.0])[0];
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if ax(lo) * ax(mid) <= 0.0 { hi = mid; } else { lo = mid; }
    }
    0.5 * (lo + hi)
}
pub fn l1_x() -> f64 { bisect_ax(-0.5, -0.05) }
pub fn l2_x() -> f64 { bisect_ax(0.05, 0.5) }

pub fn lyapunov_seed(amplitude: f64) -> ([f64;6], f64) {
    let fm = cr3bp();
    let xl = l1_x();
    let a = fm.accel_jacobian(&[xl, 0.0, 0.0, 0.0, 0.0, 0.0]);
    let (uxx, uyy) = (a[3][0], a[4][1]); // position-gradient block incl. centrifugal
    let b_ = uxx + uyy - 4.0;
    let disc = (b_ * b_ - 4.0 * uxx * uyy).sqrt();
    let s = 0.5 * (-b_ + disc); // positive root (Uxx·Uyy < 0 at L1)
    let omega = s.sqrt();
    let ratio = -(uxx + omega * omega) / (2.0 * omega); // B/A
    let aamp = amplitude;
    let bamp = ratio * aamp;
    ([xl + aamp, 0.0, 0.0, 0.0, bamp * omega, 0.0], std::f64::consts::TAU / omega)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn l1_l2_positions() {
        // Earth-Moon L1 barycentric ≈ 0.83692, L2 ≈ 1.15568 (μ = 0.012150585)
        // Moon-centered: subtract (1 − μ) = 0.987849
        assert!((l1_x() - (-0.150934)).abs() < 2e-3);
        assert!((l2_x() - 0.167833).abs() < 2e-3);
    }
    #[test]
    fn lyapunov_seed_is_planar_and_periodic_ish() {
        let (s, t_lin) = lyapunov_seed(1e-3);
        assert_eq!(s[2], 0.0); assert_eq!(s[5], 0.0);
        assert!(t_lin > 2.0 && t_lin < 4.0); // in-plane period ≈ 2π/2.33 ≈ 2.69
    }
}
