use std::f64::consts::TAU;

pub struct Coe {
    pub a: f64,
    pub e: f64,
    pub i: f64,
    pub raan: f64,
    pub aop: f64,
    pub ta: f64,
}

fn cross(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn norm(a: &[f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn wrap(x: f64) -> f64 {
    let y = x % TAU;
    if y < 0.0 {
        y + TAU
    } else {
        y
    }
}

pub fn coe_to_rv(c: &Coe, mu: f64) -> ([f64; 3], [f64; 3]) {
    let p = c.a * (1.0 - c.e * c.e);
    let r = p / (1.0 + c.e * c.ta.cos());
    let (rpf, vpf) = (
        [r * c.ta.cos(), r * c.ta.sin(), 0.0],
        [
            -(mu / p).sqrt() * c.ta.sin(),
            (mu / p).sqrt() * (c.e + c.ta.cos()),
            0.0,
        ],
    );
    let (co, so, ci, si, cw, sw) = (
        c.raan.cos(),
        c.raan.sin(),
        c.i.cos(),
        c.i.sin(),
        c.aop.cos(),
        c.aop.sin(),
    );
    // R3(-raan) R1(-i) R3(-aop)
    let rot = [
        [co * cw - so * sw * ci, -co * sw - so * cw * ci, so * si],
        [so * cw + co * sw * ci, -so * sw + co * cw * ci, -co * si],
        [sw * si, cw * si, ci],
    ];
    let apply = |x: &[f64; 3]| {
        [
            rot[0][0] * x[0] + rot[0][1] * x[1] + rot[0][2] * x[2],
            rot[1][0] * x[0] + rot[1][1] * x[1] + rot[1][2] * x[2],
            rot[2][0] * x[0] + rot[2][1] * x[1] + rot[2][2] * x[2],
        ]
    };
    (apply(&rpf), apply(&vpf))
}

pub fn rv_to_coe(r: &[f64; 3], v: &[f64; 3], mu: f64) -> Coe {
    let rn = norm(r);
    let vn = norm(v);
    let h = cross(r, v);
    let hn = norm(&h);
    let n = cross(&[0.0, 0.0, 1.0], &h);
    let nn = norm(&n);
    let ev = {
        let c1 = vn * vn - mu / rn;
        let c2 = dot(r, v);
        [
            (c1 * r[0] - c2 * v[0]) / mu,
            (c1 * r[1] - c2 * v[1]) / mu,
            (c1 * r[2] - c2 * v[2]) / mu,
        ]
    };
    let e = norm(&ev);
    let a = 1.0 / (2.0 / rn - vn * vn / mu);
    let i = (h[2] / hn).acos();
    let raan = wrap(f64::atan2(n[1], n[0]));
    let aop = {
        let cosw = dot(&n, &ev) / (nn * e);
        let w = cosw.clamp(-1.0, 1.0).acos();
        wrap(if ev[2] < 0.0 { TAU - w } else { w })
    };
    let ta = {
        let cosv = dot(&ev, r) / (e * rn);
        let t = cosv.clamp(-1.0, 1.0).acos();
        wrap(if dot(r, v) < 0.0 { TAU - t } else { t })
    };
    Coe { a, e, i, raan, aop, ta }
}

pub fn rotating_to_inertial(r: &[f64; 3], v: &[f64; 3], t: f64) -> ([f64; 3], [f64; 3]) {
    let (c, s) = (t.cos(), t.sin());
    let rz = |x: &[f64; 3]| [c * x[0] - s * x[1], s * x[0] + c * x[1], x[2]];
    let vplus = [v[0] - r[1], v[1] + r[0], v[2]]; // v + ẑ×r
    (rz(r), rz(&vplus))
}

pub fn inertial_to_rotating(r: &[f64; 3], v: &[f64; 3], t: f64) -> ([f64; 3], [f64; 3]) {
    let (c, s) = ((-t).cos(), (-t).sin());
    let rz = |x: &[f64; 3]| [c * x[0] - s * x[1], s * x[0] + c * x[1], x[2]];
    let rr = rz(r);
    let vi = rz(v);
    (rr, [vi[0] + rr[1], vi[1] - rr[0], vi[2]]) // v_rot = R⁻¹v_in − ẑ×r_rot
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::MU_MOON_ND;

    #[test]
    fn coe_rv_round_trip() {
        let c = Coe {
            a: 0.02,
            e: 0.6,
            i: 1.0,
            raan: 0.7,
            aop: 1.6,
            ta: 2.1,
        };
        let (r, v) = coe_to_rv(&c, MU_MOON_ND);
        let c2 = rv_to_coe(&r, &v, MU_MOON_ND);
        for (x, y) in [
            (c.a, c2.a),
            (c.e, c2.e),
            (c.i, c2.i),
            (c.raan, c2.raan),
            (c.aop, c2.aop),
            (c.ta, c2.ta),
        ] {
            assert!((x - y).abs() < 1e-10, "{x} vs {y}");
        }
    }

    #[test]
    fn frame_round_trip_and_velocity_offset() {
        let r = [0.02, -0.01, 0.005];
        let v = [0.1, 0.3, -0.2];
        let t = 1.234;
        let (ri, vi) = rotating_to_inertial(&r, &v, t);
        let (rr, vr) = inertial_to_rotating(&ri, &vi, t);
        for k in 0..3 {
            assert!((rr[k] - r[k]).abs() < 1e-12 && (vr[k] - v[k]).abs() < 1e-12);
        }
        // at t=0 positions equal, velocities differ by ẑ×r
        let (r0, v0) = rotating_to_inertial(&r, &v, 0.0);
        assert!((r0[0] - r[0]).abs() < 1e-15);
        assert!((v0[0] - (v[0] - r[1])).abs() < 1e-15); // (ẑ×r)_x = -y
        assert!((v0[1] - (v[1] + r[0])).abs() < 1e-15);
    }
}
