use crate::writer::MemberOut;
use elfo_core::constants::R_MOON_KM;

const RESIDUAL_MAX: f64 = 1e-9;
const PERIAPSIS_MARGIN_KM: f64 = 50.0;
const PERIOD_ND_JUMP_MAX: f64 = 0.05;
const NU1_JUMP_MAX: f64 = 5.0;

/// Sanity-check one catalog member (optionally against the previous member in
/// its family) and return human-readable flags for anything suspicious.
pub fn check_member(m: &MemberOut, prev: Option<&MemberOut>) -> Vec<String> {
    let mut flags = Vec::new();

    if m.residual > RESIDUAL_MAX {
        flags.push(format!(
            "member {}: residual {:.3e} exceeds {:.0e}",
            m.index, m.residual, RESIDUAL_MAX
        ));
    }

    let peri_floor = R_MOON_KM + PERIAPSIS_MARGIN_KM;
    if m.r_peri_km < peri_floor {
        flags.push(format!(
            "member {}: periapsis altitude {:.1} km below floor {:.1} km",
            m.index, m.r_peri_km, peri_floor
        ));
    }

    if let Some(p) = prev {
        let dperiod = (m.period_nd - p.period_nd).abs();
        if dperiod > PERIOD_ND_JUMP_MAX {
            flags.push(format!(
                "member {}: period_nd jumped {:.4} from previous member (limit {:.2})",
                m.index, dperiod, PERIOD_ND_JUMP_MAX
            ));
        }

        let dnu1 = (m.nu1 - p.nu1).abs();
        if dnu1 > NU1_JUMP_MAX {
            flags.push(format!(
                "member {}: nu1 jumped {:.2} from previous member (limit {:.1})",
                m.index, dnu1, NU1_JUMP_MAX
            ));
        }
    }

    flags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::ElementsOut;

    /// Build a plausible `MemberOut` for QA tests, varying only the fields
    /// the check logic inspects.
    fn mk_member(index: usize, residual: f64, r_peri_km: f64, period_nd: f64, nu1: f64) -> MemberOut {
        MemberOut {
            index,
            state0: [0.0; 6],
            period_s: period_nd * elfo_core::constants::TU_S,
            period_nd,
            elements: ElementsOut {
                a_km: 10_000.0,
                e: 0.5,
                i_deg: 57.0,
                omega_deg: 90.0,
                raan_deg: 90.0,
            },
            nu1,
            nu2: nu1,
            r_peri_km,
            r_apo_km: r_peri_km + 5_000.0,
            residual,
            traj: format!("full/n25/{index}.f32"),
        }
    }

    #[test]
    fn qa_flags_planted_violations() {
        let good = mk_member(0, 1e-10, 1900.0, 6.28, 1.2); // helper builds MemberOut
        let bad  = mk_member(1, 1e-6, 1700.0, 6.40, 9.0);
        assert!(check_member(&good, None).is_empty());
        let flags = check_member(&bad, Some(&good));
        assert!(flags.iter().any(|f| f.contains("residual")));
        assert!(flags.iter().any(|f| f.contains("periapsis")));
        assert!(flags.iter().any(|f| f.contains("nu1")));
    }
}
