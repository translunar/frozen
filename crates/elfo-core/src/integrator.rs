pub struct Dp54 { pub rtol: f64, pub atol: f64 }
impl Default for Dp54 { fn default() -> Self { Self { rtol: 1e-12, atol: 1e-12 } } }

const C: [f64; 7] = [0.0, 0.2, 0.3, 0.8, 8.0/9.0, 1.0, 1.0];
const A: [[f64; 6]; 7] = [
    [0.0; 6],
    [0.2, 0.0, 0.0, 0.0, 0.0, 0.0],
    [3.0/40.0, 9.0/40.0, 0.0, 0.0, 0.0, 0.0],
    [44.0/45.0, -56.0/15.0, 32.0/9.0, 0.0, 0.0, 0.0],
    [19372.0/6561.0, -25360.0/2187.0, 64448.0/6561.0, -212.0/729.0, 0.0, 0.0],
    [9017.0/3168.0, -355.0/33.0, 46732.0/5247.0, 49.0/176.0, -5103.0/18656.0, 0.0],
    [35.0/384.0, 0.0, 500.0/1113.0, 125.0/192.0, -2187.0/6784.0, 11.0/84.0],
];
const B5: [f64; 7] = [35.0/384.0, 0.0, 500.0/1113.0, 125.0/192.0, -2187.0/6784.0, 11.0/84.0, 0.0];
const B4: [f64; 7] = [5179.0/57600.0, 0.0, 7571.0/16695.0, 393.0/640.0,
                      -92097.0/339200.0, 187.0/2100.0, 1.0/40.0];

impl Dp54 {
    fn step(&self, f: &impl Fn(f64, &[f64]) -> Vec<f64>, t: f64, y: &[f64], h: f64,
            k1: &[f64]) -> (Vec<f64>, Vec<f64>, f64) {
        let n = y.len();
        let mut k: Vec<Vec<f64>> = vec![k1.to_vec()];
        for i in 1..7 {
            let mut yi = y.to_vec();
            for j in 0..i { for m in 0..n { yi[m] += h * A[i][j] * k[j][m]; } }
            k.push(f(t + C[i] * h, &yi));
        }
        let mut y5 = y.to_vec();
        let mut err = 0.0f64;
        for m in 0..n {
            let mut d5 = 0.0; let mut d4 = 0.0;
            for i in 0..7 { d5 += B5[i] * k[i][m]; d4 += B4[i] * k[i][m]; }
            y5[m] += h * d5;
            let sc = self.atol + self.rtol * y[m].abs().max(y5[m].abs());
            let e = h * (d5 - d4) / sc;
            err += e * e;
        }
        (y5, k[6].clone(), (err / n as f64).sqrt()) // k7 = f(t+h, y5): FSAL
    }

    pub fn propagate(&self, f: &impl Fn(f64, &[f64]) -> Vec<f64>, y0: &[f64],
                     t0: f64, tf: f64, sample_times: &[f64],
                     observer: &mut impl FnMut(f64, &[f64])) -> Vec<f64> {
        let mut t = t0; let mut y = y0.to_vec();
        let mut k1 = f(t, &y);
        let mut h = (tf - t0) * 1e-4;
        let mut samples = sample_times.iter().copied().peekable();
        while t < tf - 1e-15 {
            let mut hmax = tf - t;
            if let Some(&ts) = samples.peek() { if ts > t + 1e-15 { hmax = hmax.min(ts - t); } }
            let htry = h.min(hmax);
            let (y5, k7, err) = self.step(f, t, &y, htry, &k1);
            if err <= 1.0 {
                t += htry; y = y5; k1 = k7;
                if let Some(&ts) = samples.peek() {
                    if (t - ts).abs() < 1e-12 { observer(ts, &y); samples.next(); }
                }
                h = htry * (0.9 * err.max(1e-10).powf(-0.2)).clamp(0.2, 5.0);
            } else {
                h = htry * (0.9 * err.powf(-0.2)).clamp(0.2, 1.0);
            }
        }
        y
    }

    pub fn propagate_fixed(&self, f: &impl Fn(f64, &[f64]) -> Vec<f64>, y0: &[f64],
                           t0: f64, tf: f64, h: f64) -> Vec<f64> {
        let mut t = t0; let mut y = y0.to_vec(); let mut k1 = f(t, &y);
        while t < tf - 1e-15 {
            let hs = h.min(tf - t);
            let (y5, k7, _) = self.step(f, t, &y, hs, &k1);
            t += hs; y = y5; k1 = k7;
        }
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{constants::*, elements::*, forces::ForceModel};
    fn kepler_fm() -> ForceModel { ForceModel { j2: false, c22: false, j3: false, earth: false } }

    #[test]
    fn kepler_orbit_closes_via_inertial_comparison() {
        let fm = kepler_fm();
        let coe = Coe { a: 0.025, e: 0.5, i: 1.1, raan: 0.4, aop: 1.2, ta: 0.0 };
        let (ri, vi) = coe_to_rv(&coe, MU_MOON_ND);
        let (r0, v0) = inertial_to_rotating(&ri, &vi, 0.0);
        let y0 = [r0[0],r0[1],r0[2],v0[0],v0[1],v0[2]];
        let t_kep = std::f64::consts::TAU * (coe.a.powi(3) / MU_MOON_ND).sqrt();
        let integ = Dp54::default();
        let f = |_t: f64, y: &[f64]| {
            let s = [y[0],y[1],y[2],y[3],y[4],y[5]];
            fm.eom(&s).to_vec()
        };
        let yf = integ.propagate(&f, &y0, 0.0, t_kep, &[], &mut |_,_|{});
        let (rf, vf) = rotating_to_inertial(&[yf[0],yf[1],yf[2]], &[yf[3],yf[4],yf[5]], t_kep);
        for k in 0..3 {
            assert!((rf[k]-ri[k]).abs() < 1e-9, "pos {k}");
            assert!((vf[k]-vi[k]).abs() < 1e-9, "vel {k}");
        }
    }

    #[test]
    fn energy_conserved_full_model() {
        let fm = ForceModel { j2: true, c22: true, j3: true, earth: true };
        let y0 = [0.02, 0.0, 0.01, 0.0, -0.55, 0.3];
        let e0 = fm.energy(&y0);
        let f = |_t: f64, y: &[f64]| fm.eom(&[y[0],y[1],y[2],y[3],y[4],y[5]]).to_vec();
        let yf = Dp54::default().propagate(&f, &y0, 0.0, 6.28, &[], &mut |_,_|{});
        let ef = fm.energy(&[yf[0],yf[1],yf[2],yf[3],yf[4],yf[5]]);
        // 2.5e-10 bound: measured drift 1.37e-10 over 6.28 TU at rtol 1e-12 —
        // legitimate accumulation (drops 10x at rtol 1e-13), not a defect.
        assert!((ef - e0).abs() < 2.5e-10, "dE = {}", ef - e0);
    }

    #[test]
    fn fixed_step_shows_fifth_order() {
        let fm = kepler_fm();
        let y0 = [0.02, 0.0, 0.0, 0.0, -0.75, 0.2];
        let f = |_t: f64, y: &[f64]| fm.eom(&[y[0],y[1],y[2],y[3],y[4],y[5]]).to_vec();
        let reference = Dp54 { rtol: 1e-13, atol: 1e-13 }.propagate(&f, &y0, 0.0, 0.1, &[], &mut |_,_|{});
        let integ = Dp54::default();
        let e1: f64 = integ.propagate_fixed(&f, &y0, 0.0, 0.1, 1e-3).iter()
            .zip(&reference).map(|(a,b)| (a-b).abs()).fold(0.0, f64::max);
        let e2: f64 = integ.propagate_fixed(&f, &y0, 0.0, 0.1, 5e-4).iter()
            .zip(&reference).map(|(a,b)| (a-b).abs()).fold(0.0, f64::max);
        let order = (e1 / e2).log2();
        assert!(order > 4.5 && order < 6.5, "observed order {order}");
    }

    #[test]
    fn sample_times_hit_exactly() {
        let fm = kepler_fm();
        let y0 = [0.02, 0.0, 0.0, 0.0, -0.75, 0.0];
        let f = |_t: f64, y: &[f64]| fm.eom(&[y[0],y[1],y[2],y[3],y[4],y[5]]).to_vec();
        let mut seen = Vec::new();
        Dp54::default().propagate(&f, &y0, 0.0, 0.5, &[0.1, 0.25, 0.4],
            &mut |t, _| seen.push(t));
        assert_eq!(seen, vec![0.1, 0.25, 0.4]);
    }
}
