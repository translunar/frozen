use crate::constants::*;

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
