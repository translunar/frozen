// Sources: GM values DE440 (Park et al. 2021); harmonics unnormalized,
// GRGM1200-series derived (J2 = -sqrt(5)*C20bar etc.); R_MOON IAU mean.
pub const GM_EARTH_KM3S2: f64 = 398_600.435507;
pub const GM_MOON_KM3S2: f64 = 4_902.800118;
pub const A_EM_KM: f64 = 384_400.0;
pub const R_MOON_KM: f64 = 1_737.4;
pub const MOON_J2: f64 = 2.0323e-4;
pub const MOON_C22: f64 = 2.2426e-5;
pub const MOON_J3: f64 = 8.46e-6;

pub const MU: f64 = GM_MOON_KM3S2 / (GM_EARTH_KM3S2 + GM_MOON_KM3S2);
pub const MU_MOON_ND: f64 = MU;
pub const MU_EARTH_ND: f64 = 1.0 - MU;
pub const R_MOON_ND: f64 = R_MOON_KM / A_EM_KM;

/// seconds per nondimensional time unit: sqrt(a^3 / (GM_E + GM_M))
pub const TU_S: f64 = 375190.261894658906385;

pub fn km_to_nd(km: f64) -> f64 { km / A_EM_KM }
pub fn nd_to_km(nd: f64) -> f64 { nd * A_EM_KM }
pub fn kms_to_nd(kms: f64) -> f64 { kms * TU_S / A_EM_KM }
pub fn nd_to_kms(nd: f64) -> f64 { nd * A_EM_KM / TU_S }
pub fn s_to_nd(s: f64) -> f64 { s / TU_S }
pub fn nd_to_s(nd: f64) -> f64 { nd * TU_S }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mass_parameter_and_time_unit() {
        assert!((MU - 0.012150585).abs() < 1e-6);
        assert!((TU_S - 375_190.0).abs() < 200.0); // ≈ 4.34 days
        let x = 12345.6;
        assert!((nd_to_km(km_to_nd(x)) - x).abs() < 1e-9);
        assert!((nd_to_kms(kms_to_nd(0.5)) - 0.5).abs() < 1e-12);
    }
}
