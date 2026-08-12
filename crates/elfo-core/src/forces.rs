use crate::constants::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug)]
pub struct Term { pub c: f64, pub p: [i32; 3], pub n: i32 }

impl Term {
    pub fn eval(&self, r: &[f64;3]) -> f64 {
        let rn = (r[0]*r[0] + r[1]*r[1] + r[2]*r[2]).sqrt();
        self.c * r[0].powi(self.p[0]) * r[1].powi(self.p[1]) * r[2].powi(self.p[2])
            * rn.powi(-self.n)
    }
    pub fn deriv(&self, axis: usize) -> Vec<Term> {
        let mut out = Vec::with_capacity(2);
        if self.p[axis] > 0 {
            let mut p = self.p; p[axis] -= 1;
            out.push(Term { c: self.c * self.p[axis] as f64, p, n: self.n });
        }
        if self.n != 0 {
            let mut p = self.p; p[axis] += 1;
            out.push(Term { c: -self.c * self.n as f64, p, n: self.n + 2 });
        }
        out
    }
}

pub fn terms_value(terms: &[Term], r: &[f64;3]) -> f64 {
    terms.iter().map(|t| t.eval(r)).sum()
}
pub fn terms_grad(terms: &[Term], r: &[f64;3]) -> [f64;3] {
    let mut g = [0.0; 3];
    for t in terms { for k in 0..3 { for d in t.deriv(k) { g[k] += d.eval(r); } } }
    g
}
pub fn terms_hess(terms: &[Term], r: &[f64;3]) -> [[f64;3];3] {
    let mut h = [[0.0; 3]; 3];
    for t in terms {
        for k in 0..3 { for d in t.deriv(k) { for j in 0..3 { for dd in d.deriv(j) {
            h[j][k] += dd.eval(r);
        }}}}
    }
    h
}

/// Perturbing potential U such that a_perturbation = ∇U. Monopole excluded.
/// U_J2  = −(k2/2)(3z²/r⁵ − 1/r³),      k2  = μm J2 R²
/// U_C22 = 3 k22 (x² − y²)/r⁵,          k22 = μm C22 R²
/// U_J3  = −(k3/2)(5z³/r⁷ − 3z/r⁵),     k3  = μm J3 R³
pub fn harmonic_terms(j2: bool, c22: bool, j3: bool) -> Vec<Term> {
    let (mu_m, r_m) = (MU_MOON_ND, R_MOON_ND);
    let mut v = Vec::new();
    if j2 {
        let k2 = mu_m * MOON_J2 * r_m * r_m;
        v.push(Term { c: -1.5 * k2, p: [0,0,2], n: 5 });
        v.push(Term { c:  0.5 * k2, p: [0,0,0], n: 3 });
    }
    if c22 {
        let k22 = mu_m * MOON_C22 * r_m * r_m;
        v.push(Term { c:  3.0 * k22, p: [2,0,0], n: 5 });
        v.push(Term { c: -3.0 * k22, p: [0,2,0], n: 5 });
    }
    if j3 {
        let k3 = mu_m * MOON_J3 * r_m * r_m * r_m;
        v.push(Term { c: -2.5 * k3, p: [0,0,3], n: 7 });
        v.push(Term { c:  1.5 * k3, p: [0,0,1], n: 5 });
    }
    v
}

pub const EARTH_POS: [f64; 3] = [-1.0, 0.0, 0.0];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ForceModel { pub j2: bool, pub c22: bool, pub j3: bool, pub earth: bool }

impl ForceModel {
    pub fn harmonics(&self) -> Vec<Term> { harmonic_terms(self.j2, self.c22, self.j3) }

    fn bx(&self) -> f64 { if self.earth { -(1.0 - MU) } else { 0.0 } }

    pub fn omega_eff(&self, r: &[f64;3]) -> f64 {
        let bx = self.bx();
        let rn = (r[0]*r[0]+r[1]*r[1]+r[2]*r[2]).sqrt();
        let mut u = 0.5 * ((r[0]-bx)*(r[0]-bx) + r[1]*r[1]) + MU_MOON_ND / rn;
        if self.earth {
            let d = [r[0]-EARTH_POS[0], r[1], r[2]];
            u += MU_EARTH_ND / (d[0]*d[0]+d[1]*d[1]+d[2]*d[2]).sqrt();
        }
        u + terms_value(&self.harmonics(), r)
    }

    pub fn accel(&self, s: &[f64;6]) -> [f64;3] {
        let (r, v) = ([s[0],s[1],s[2]], [s[3],s[4],s[5]]);
        let bx = self.bx();
        let rn2 = r[0]*r[0]+r[1]*r[1]+r[2]*r[2];
        let r3 = rn2 * rn2.sqrt();
        let mut a = [
            (r[0]-bx) + 2.0*v[1] - MU_MOON_ND*r[0]/r3,
            r[1]      - 2.0*v[0] - MU_MOON_ND*r[1]/r3,
                                 - MU_MOON_ND*r[2]/r3,
        ];
        if self.earth {
            let d = [r[0]+1.0, r[1], r[2]];
            let dn2 = d[0]*d[0]+d[1]*d[1]+d[2]*d[2];
            let d3 = dn2 * dn2.sqrt();
            for k in 0..3 { a[k] -= MU_EARTH_ND * d[k] / d3; }
        }
        let g = terms_grad(&self.harmonics(), &r);
        [a[0]+g[0], a[1]+g[1], a[2]+g[2]]
    }

    pub fn eom(&self, s: &[f64;6]) -> [f64;6] {
        let a = self.accel(s);
        [s[3], s[4], s[5], a[0], a[1], a[2]]
    }

    pub fn energy(&self, s: &[f64;6]) -> f64 {
        0.5*(s[3]*s[3]+s[4]*s[4]+s[5]*s[5]) - self.omega_eff(&[s[0],s[1],s[2]])
    }
}

#[cfg(test)]
mod term_tests {
    use super::*;
    fn fd_grad(terms: &[Term], r: &[f64;3]) -> [f64;3] {
        let h = 1e-6; let mut g = [0.0; 3];
        for k in 0..3 {
            let (mut rp, mut rm) = (*r, *r); rp[k] += h; rm[k] -= h;
            g[k] = (terms_value(terms, &rp) - terms_value(terms, &rm)) / (2.0 * h);
        }
        g
    }
    #[test]
    fn gradient_and_hessian_match_finite_differences() {
        let terms = harmonic_terms(true, true, true);
        let r = [0.021, -0.013, 0.017];
        let g = terms_grad(&terms, &r);
        let gfd = fd_grad(&terms, &r);
        for k in 0..3 { assert!((g[k]-gfd[k]).abs() < (1e-9 * g[k].abs()).max(1e-12)); }
        let hh = terms_hess(&terms, &r);
        let h = 1e-6;
        for k in 0..3 {
            let (mut rp, mut rm) = (r, r); rp[k] += h; rm[k] -= h;
            let (gp, gm) = (terms_grad(&terms, &rp), terms_grad(&terms, &rm));
            for j in 0..3 {
                let fd = (gp[j]-gm[j])/(2.0*h);
                assert!((hh[j][k]-fd).abs() < (1e-7 * fd.abs()).max(1e-10));
            }
        }
        // Hessian symmetry
        for j in 0..3 { for k in 0..3 { assert!((hh[j][k]-hh[k][j]).abs() < 1e-15); } }
    }
    #[test]
    fn j2_sign_sanity() {
        // J2 potential term is negative over the poles, positive at equator
        let terms = harmonic_terms(true, false, false);
        assert!(terms_value(&terms, &[0.0, 0.0, 0.01]) < 0.0);
        assert!(terms_value(&terms, &[0.01, 0.0, 0.0]) > 0.0);
    }
}

#[cfg(test)]
mod force_tests {
    use super::*;
    #[test]
    fn accel_is_gradient_of_omega_eff_at_zero_velocity() {
        for fm in [
            ForceModel { j2: true, c22: true, j3: true, earth: true },
            ForceModel { j2: true, c22: false, j3: true, earth: false },
        ] {
            let r = [0.019, -0.011, 0.014];
            let s = [r[0], r[1], r[2], 0.0, 0.0, 0.0];
            let a = fm.accel(&s);
            let h = 1e-7;
            for k in 0..3 {
                let (mut rp, mut rm) = (r, r); rp[k] += h; rm[k] -= h;
                let fd = (fm.omega_eff(&rp) - fm.omega_eff(&rm)) / (2.0 * h);
                assert!((a[k] - fd).abs() < (1e-6 * fd.abs()).max(1e-8), "k={k}");
            }
        }
    }
    #[test]
    fn l1_direction_sanity() {
        // between Moon and Earth (x<0), net x-accel at rest points toward Earth
        // beyond L1 (|x| large), toward Moon inside L1 (|x| small)
        let fm = ForceModel { j2: false, c22: false, j3: false, earth: true };
        let inside  = fm.accel(&[-0.05, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let outside = fm.accel(&[-0.40, 0.0, 0.0, 0.0, 0.0, 0.0]);
        assert!(inside[0] > 0.0 && outside[0] < 0.0);
    }
}
