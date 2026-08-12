# ELFO Family Browser Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Rust catalog generator that computes elliptical lunar frozen orbit (ELFO) families as periodic orbits of the Earth-Moon rotating frame, plus a TypeScript/three.js web cockpit that browses, animates, and compares them across force-model combinations.

**Architecture:** Cargo workspace with `elfo-core` (dynamics, DP54 integrator, STM, multiple-shooting corrector, pseudo-arclength continuation, stability indices) and `elfo-catalog` (CLI that sweeps force-model combos × resonances and writes JSON + binary trajectory files). The web app is a static Vite site (no framework, no WASM) that reads those files. Spec: `docs/superpowers/specs/2026-08-11-elfo-family-browser-design.md`.

**Tech Stack:** Rust 2021 (nalgebra, serde, serde_json, toml, rayon, clap, chrono, anyhow; tempfile as dev-dep), Vite + TypeScript (strict) + three.js + d3-scale + vitest.

## Global Constraints

- All Rust code in `crates/elfo-core` or `crates/elfo-catalog`; workspace root `Cargo.toml` with `resolver = "2"`.
- Nondimensional units internally: LU = 384400 km (Earth-Moon distance), TU = 1/n where n = mean motion of the Earth-Moon system; frame angular rate ω = 1. Dimensional km, km/s, s at file/API boundaries.
- Frame: Moon-centered Earth-Moon rotating frame; **Earth at (−1, 0, 0)** nondim; +z along Earth-Moon orbital angular momentum. three.js scenes must set `camera.up = (0,0,1)` and use these axes directly.
- Integrator tolerances: rtol = 1e-12, atol = 1e-12 (nondim). Corrector convergence: ‖residual‖∞ < 1e-10 (nondim). Catalog QA residual limit: 1e-9.
- Every data file carries `schema_version: 1`. Binary trajectory files are little-endian Float32 xyz triples in **km**, uniform time steps, no header.
- TDD: every task writes its failing test first. Frequent commits; each task ends in a commit.
- Do not add dependencies beyond those listed in Tech Stack.
- Generated catalog data (`web/public/catalog/`) is git-ignored; only the generator config is committed.

## File Structure

```
frozen/
├── Cargo.toml                      # workspace
├── .gitignore
├── catalog.toml                    # real generation config (committed)
├── crates/
│   ├── elfo-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs              # module wiring + re-exports
│   │       ├── constants.rs        # physical constants, nondim scaling
│   │       ├── elements.rs         # Keplerian ↔ Cartesian, rotating ↔ inertial
│   │       ├── forces.rs           # Term calculus, ForceModel, accel, grad, energy
│   │       ├── integrator.rs       # DP54 adaptive + fixed-step, 6-dim and 42-dim (STM)
│   │       ├── lagrange.rs         # L1/L2 location, planar linearization seeds
│   │       ├── shooting.rs         # multiple shooting residual + SVD Gauss-Newton
│   │       ├── continuation.rs     # tangent, pseudo-arclength family builder
│   │       ├── stability.rs        # monodromy, eigenvalue pairing, indices
│   │       └── seeds.rs            # ELFO frozen-orbit seeds per force model
│   └── elfo-catalog/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs             # CLI (clap): gen --config --out
│           ├── config.rs           # TOML config structs
│           ├── qa.rs               # member/family QA checks
│           └── writer.rs           # catalog.json + .f32 + preview writers
└── web/
    ├── package.json, tsconfig.json, index.html, src/style.css
    └── src/
        ├── types.ts                # Catalog/Combo/Family/Member interfaces
        ├── data.ts                 # fetch + parse + cache + prefetch
        ├── state.ts                # pub/sub store, nearest-member, symlog
        ├── scene.ts                # three.js stage: Moon, stack, selected, ghost
        ├── anim.ts                 # rAF loop, satellite marker, trail, speed dial
        ├── ui/leftRail.ts          # toggles, family list, slider, readout
        ├── ui/stabilityPlot.ts     # SVG plot, symlog axis, click-to-select
        └── main.ts                 # wiring
```

Responsibilities are one-per-file as annotated. Later tasks must not restructure earlier files, only extend them.

---

### Task 1: Workspace scaffold + constants

**Files:**
- Create: `Cargo.toml`, `.gitignore`, `crates/elfo-core/Cargo.toml`, `crates/elfo-core/src/lib.rs`, `crates/elfo-core/src/constants.rs`

**Interfaces:**
- Produces: `constants::{GM_EARTH_KM3S2, GM_MOON_KM3S2, A_EM_KM, R_MOON_KM, MOON_J2, MOON_C22, MOON_J3, MU, MU_MOON_ND, MU_EARTH_ND, R_MOON_ND, TU_S, km_to_nd, nd_to_km, kms_to_nd, nd_to_kms, s_to_nd, nd_to_s}` — all `f64` consts / `fn(f64) -> f64`.

- [ ] **Step 1: Scaffold workspace**

`Cargo.toml` (root):
```toml
[workspace]
members = ["crates/elfo-core", "crates/elfo-catalog"]
resolver = "2"
```
`.gitignore`:
```
/target
node_modules
web/public/catalog/
dist
```
`crates/elfo-core/Cargo.toml`:
```toml
[package]
name = "elfo-core"
version = "0.1.0"
edition = "2021"

[dependencies]
nalgebra = "0.33"
serde = { version = "1", features = ["derive"] }
```
`crates/elfo-core/src/lib.rs`:
```rust
pub mod constants;
```
(Create `crates/elfo-catalog` in Task 14; workspace member list may temporarily fail — instead list only `elfo-core` now and add `elfo-catalog` in Task 14.)

- [ ] **Step 2: Write the failing test** (bottom of `constants.rs`)

```rust
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
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p elfo-core`
Expected: FAIL (constants not defined)

- [ ] **Step 4: Implement `constants.rs`**

```rust
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
pub const TU_S: f64 = 375_189.76; // recompute below in test if constants change

pub fn km_to_nd(km: f64) -> f64 { km / A_EM_KM }
pub fn nd_to_km(nd: f64) -> f64 { nd * A_EM_KM }
pub fn kms_to_nd(kms: f64) -> f64 { kms * TU_S / A_EM_KM }
pub fn nd_to_kms(nd: f64) -> f64 { nd * A_EM_KM / TU_S }
pub fn s_to_nd(s: f64) -> f64 { s / TU_S }
pub fn nd_to_s(nd: f64) -> f64 { nd * TU_S }
```
Note: compute `TU_S` once via `(A_EM_KM.powi(3) / (GM_EARTH_KM3S2 + GM_MOON_KM3S2)).sqrt()` in a scratch test and paste the value to full precision (const fn sqrt is not stable); the tolerance in the test guards it.

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p elfo-core`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat: workspace scaffold and physical constants"
```

---

### Task 2: Keplerian elements ↔ Cartesian, rotating ↔ inertial

**Files:**
- Create: `crates/elfo-core/src/elements.rs`; Modify: `lib.rs` (add `pub mod elements;`)

**Interfaces:**
- Consumes: `constants::MU_MOON_ND`
- Produces:
  - `pub struct Coe { pub a: f64, pub e: f64, pub i: f64, pub raan: f64, pub aop: f64, pub ta: f64 }` (nondim a, radians)
  - `pub fn coe_to_rv(coe: &Coe, mu: f64) -> ([f64; 3], [f64; 3])` (inertial Moon-centered)
  - `pub fn rv_to_coe(r: &[f64; 3], v: &[f64; 3], mu: f64) -> Coe`
  - `pub fn inertial_to_rotating(r: &[f64;3], v: &[f64;3], t: f64) -> ([f64;3],[f64;3])`
  - `pub fn rotating_to_inertial(r: &[f64;3], v: &[f64;3], t: f64) -> ([f64;3],[f64;3])`

  Frames coincide at t = 0; rotating frame spins at rate 1 about +z, so `r_in = Rz(t) · r_rot`, `v_in = Rz(t) · (v_rot + ẑ × r_rot)`.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::MU_MOON_ND;
    #[test]
    fn coe_rv_round_trip() {
        let c = Coe { a: 0.02, e: 0.6, i: 1.0, raan: 0.7, aop: 1.6, ta: 2.1 };
        let (r, v) = coe_to_rv(&c, MU_MOON_ND);
        let c2 = rv_to_coe(&r, &v, MU_MOON_ND);
        for (x, y) in [(c.a,c2.a),(c.e,c2.e),(c.i,c2.i),(c.raan,c2.raan),(c.aop,c2.aop),(c.ta,c2.ta)] {
            assert!((x - y).abs() < 1e-10, "{x} vs {y}");
        }
    }
    #[test]
    fn frame_round_trip_and_velocity_offset() {
        let r = [0.02, -0.01, 0.005]; let v = [0.1, 0.3, -0.2]; let t = 1.234;
        let (ri, vi) = rotating_to_inertial(&r, &v, t);
        let (rr, vr) = inertial_to_rotating(&ri, &vi, t);
        for k in 0..3 { assert!((rr[k]-r[k]).abs() < 1e-12 && (vr[k]-v[k]).abs() < 1e-12); }
        // at t=0 positions equal, velocities differ by ẑ×r
        let (r0, v0) = rotating_to_inertial(&r, &v, 0.0);
        assert!((r0[0]-r[0]).abs() < 1e-15);
        assert!((v0[0] - (v[0] - r[1])).abs() < 1e-15); // (ẑ×r)_x = -y
        assert!((v0[1] - (v[1] + r[0])).abs() < 1e-15);
    }
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p elfo-core elements` → FAIL (module missing)

- [ ] **Step 3: Implement** (standard formulas; guard small-e/small-i not required — catalog orbits are eccentric and inclined; still implement the standard `rv_to_coe` with atan2-based angles, wrapping all angles to [0, 2π))

```rust
use std::f64::consts::TAU;

pub struct Coe { pub a: f64, pub e: f64, pub i: f64, pub raan: f64, pub aop: f64, pub ta: f64 }

fn cross(a: &[f64;3], b: &[f64;3]) -> [f64;3] {
    [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]
}
fn dot(a: &[f64;3], b: &[f64;3]) -> f64 { a[0]*b[0]+a[1]*b[1]+a[2]*b[2] }
fn norm(a: &[f64;3]) -> f64 { dot(a, a).sqrt() }
fn wrap(x: f64) -> f64 { let y = x % TAU; if y < 0.0 { y + TAU } else { y } }

pub fn coe_to_rv(c: &Coe, mu: f64) -> ([f64;3],[f64;3]) {
    let p = c.a * (1.0 - c.e * c.e);
    let r = p / (1.0 + c.e * c.ta.cos());
    let (rpf, vpf) = (
        [r * c.ta.cos(), r * c.ta.sin(), 0.0],
        [-(mu/p).sqrt() * c.ta.sin(), (mu/p).sqrt() * (c.e + c.ta.cos()), 0.0],
    );
    let (co, so, ci, si, cw, sw) =
        (c.raan.cos(), c.raan.sin(), c.i.cos(), c.i.sin(), c.aop.cos(), c.aop.sin());
    // R3(-raan) R1(-i) R3(-aop)
    let rot = [
        [co*cw - so*sw*ci, -co*sw - so*cw*ci,  so*si],
        [so*cw + co*sw*ci, -so*sw + co*cw*ci, -co*si],
        [sw*si,             cw*si,             ci   ],
    ];
    let apply = |x: &[f64;3]| [
        rot[0][0]*x[0]+rot[0][1]*x[1]+rot[0][2]*x[2],
        rot[1][0]*x[0]+rot[1][1]*x[1]+rot[1][2]*x[2],
        rot[2][0]*x[0]+rot[2][1]*x[1]+rot[2][2]*x[2],
    ];
    (apply(&rpf), apply(&vpf))
}

pub fn rv_to_coe(r: &[f64;3], v: &[f64;3], mu: f64) -> Coe {
    let rn = norm(r); let vn = norm(v);
    let h = cross(r, v); let hn = norm(&h);
    let n = cross(&[0.0,0.0,1.0], &h); let nn = norm(&n);
    let ev = {
        let c1 = vn*vn - mu/rn; let c2 = dot(r, v);
        [(c1*r[0]-c2*v[0])/mu, (c1*r[1]-c2*v[1])/mu, (c1*r[2]-c2*v[2])/mu]
    };
    let e = norm(&ev);
    let a = 1.0 / (2.0/rn - vn*vn/mu);
    let i = (h[2]/hn).acos();
    let raan = wrap(f64::atan2(n[1], n[0]));
    let aop = {
        let cosw = dot(&n, &ev) / (nn * e);
        let w = cosw.clamp(-1.0,1.0).acos();
        wrap(if ev[2] < 0.0 { TAU - w } else { w })
    };
    let ta = {
        let cosv = dot(&ev, r) / (e * rn);
        let t = cosv.clamp(-1.0,1.0).acos();
        wrap(if dot(r, v) < 0.0 { TAU - t } else { t })
    };
    Coe { a, e, i, raan, aop, ta }
}

pub fn rotating_to_inertial(r: &[f64;3], v: &[f64;3], t: f64) -> ([f64;3],[f64;3]) {
    let (c, s) = (t.cos(), t.sin());
    let rz = |x: &[f64;3]| [c*x[0]-s*x[1], s*x[0]+c*x[1], x[2]];
    let vplus = [v[0]-r[1], v[1]+r[0], v[2]]; // v + ẑ×r
    (rz(r), rz(&vplus))
}
pub fn inertial_to_rotating(r: &[f64;3], v: &[f64;3], t: f64) -> ([f64;3],[f64;3]) {
    let (c, s) = ((-t).cos(), (-t).sin());
    let rz = |x: &[f64;3]| [c*x[0]-s*x[1], s*x[0]+c*x[1], x[2]];
    let rr = rz(r); let vi = rz(v);
    (rr, [vi[0]+rr[1], vi[1]-rr[0], vi[2]]) // v_rot = R⁻¹v_in − ẑ×r_rot
}
```

- [ ] **Step 4: Run to verify pass** — `cargo test -p elfo-core elements` → PASS

- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat: element conversions and frame transforms"`

---

### Task 3: Potential-term calculus (exact gradients/Hessians)

**Files:**
- Create: `crates/elfo-core/src/forces.rs` (first half); Modify: `lib.rs` (add `pub mod forces;`)

**Interfaces:**
- Produces:
  - `pub struct Term { pub c: f64, pub p: [i32; 3], pub n: i32 }` — represents `c · x^p0 y^p1 z^p2 · r^(−n)`
  - `impl Term { pub fn eval(&self, r: &[f64;3]) -> f64; pub fn deriv(&self, axis: usize) -> Vec<Term> }`
  - `pub fn terms_value(terms: &[Term], r: &[f64;3]) -> f64`
  - `pub fn terms_grad(terms: &[Term], r: &[f64;3]) -> [f64;3]`
  - `pub fn terms_hess(terms: &[Term], r: &[f64;3]) -> [[f64;3];3]`
  - `pub fn harmonic_terms(j2: bool, c22: bool, j3: bool) -> Vec<Term>` — the Moon monopole is **not** included here (Task 4 handles it in closed form); this returns only the perturbing-potential terms.

The differentiation rule is exact and recursive: `d/dx_i [c x^p r^(−n)] = c·p_i·x^(p−e_i)·r^(−n) + (−c·n)·x^(p+e_i)·r^(−n−2)`. Gradient = one derivative; Hessian = two. One finite-difference test validates all harmonic gradients and Hessians forever.

- [ ] **Step 1: Write the failing tests**

```rust
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
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p elfo-core term_tests` → FAIL

- [ ] **Step 3: Implement**

```rust
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
```

- [ ] **Step 4: Run to verify pass** — `cargo test -p elfo-core term_tests` → PASS

- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat: exact potential-term calculus and lunar harmonic terms"`

---

### Task 4: ForceModel — acceleration, energy

**Files:**
- Modify: `crates/elfo-core/src/forces.rs` (append)

**Interfaces:**
- Consumes: Task 3's `Term` machinery.
- Produces:
  - `#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)] pub struct ForceModel { pub j2: bool, pub c22: bool, pub j3: bool, pub earth: bool }`
  - `impl ForceModel { pub fn accel(&self, s: &[f64;6]) -> [f64;3]; pub fn energy(&self, s: &[f64;6]) -> f64; pub fn eom(&self, s: &[f64;6]) -> [f64;6]; pub fn omega_eff(&self, r: &[f64;3]) -> f64 }`
  - Physics (nondim, Moon-centered rotating, ω = 1): Earth at `E = (−1,0,0)`; rotation center `b = (−(1−μ),0,0)` if `earth` else origin. `accel = centrifugal(r − b) + coriolis(v) + ∇(μm/r) + terms_grad(harmonics) + earth_direct` where `earth_direct = −(1−μ)(r−E)/|r−E|³` (NO separate indirect term — the barycentric centrifugal center absorbs it). `energy = ½|v|² − Ω_eff`, `Ω_eff = ½((x−bx)² + y²) + μm/r + [(1−μ)/|r−E| if earth] + terms_value(harmonics)`.

- [ ] **Step 1: Write the failing tests** (append to `forces.rs` tests)

```rust
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
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p elfo-core force_tests` → FAIL

- [ ] **Step 3: Implement** (append)

```rust
use serde::{Deserialize, Serialize};

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
```
(Note the Coriolis terms live in `accel`, not `omega_eff` — the FD test uses zero velocity so they vanish there. Performance note: `harmonics()` allocates per call; that is acceptable for phase 1 — do NOT optimize prematurely. If catalog generation is slow later, cache the `Vec<Term>` in a `OnceCell` keyed by flags.)

- [ ] **Step 4: Run to verify pass** — `cargo test -p elfo-core force_tests` → PASS

- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat: force model acceleration and energy integral"`

---

### Task 5: Acceleration Jacobian (gravity gradient)

**Files:**
- Modify: `crates/elfo-core/src/forces.rs` (append)

**Interfaces:**
- Produces: `impl ForceModel { pub fn accel_jacobian(&self, s: &[f64;6]) -> [[f64;6];6] }` — the full 6×6 `A` matrix of the variational equations: `A = [[0, I], [G, C]]` with `G = ∂accel/∂r` (point masses closed-form + `terms_hess` + centrifugal `diag(1,1,0)`), `C = [[0,2,0],[−2,0,0],[0,0,0]]` (Coriolis).

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod jac_tests {
    use super::*;
    #[test]
    fn jacobian_matches_finite_differences() {
        let fm = ForceModel { j2: true, c22: true, j3: true, earth: true };
        let s = [0.018, -0.012, 0.015, 0.05, -0.2, 0.1];
        let a = fm.accel_jacobian(&s);
        let h = 1e-7;
        for col in 0..6 {
            let (mut sp, mut sm) = (s, s); sp[col] += h; sm[col] -= h;
            let (fp, fm_) = (fm.eom(&sp), fm.eom(&sm));
            for row in 0..6 {
                let fd = (fp[row] - fm_[row]) / (2.0 * h);
                assert!((a[row][col] - fd).abs() < (1e-5 * fd.abs()).max(2e-6),
                    "row {row} col {col}: {} vs {}", a[row][col], fd);
            }
        }
    }
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p elfo-core jac_tests` → FAIL

- [ ] **Step 3: Implement** (append inside `impl ForceModel`)

```rust
    /// G(d) for a point mass μ at offset such that accel = −μ d/|d|³:
    /// ∂a/∂r = μ (3 d dᵀ/|d|⁵ − I/|d|³)
    fn point_mass_grad(mu: f64, d: &[f64;3]) -> [[f64;3];3] {
        let n2 = d[0]*d[0]+d[1]*d[1]+d[2]*d[2];
        let n5 = n2 * n2 * n2.sqrt();
        let n3 = n2 * n2.sqrt();
        let mut g = [[0.0;3];3];
        for i in 0..3 { for j in 0..3 {
            g[i][j] = mu * 3.0 * d[i]*d[j] / n5 - if i == j { mu / n3 } else { 0.0 };
        }}
        g
    }

    pub fn accel_jacobian(&self, s: &[f64;6]) -> [[f64;6];6] {
        let r = [s[0], s[1], s[2]];
        let mut g = Self::point_mass_grad(MU_MOON_ND, &r);
        if self.earth {
            let d = [r[0]+1.0, r[1], r[2]];
            let ge = Self::point_mass_grad(MU_EARTH_ND, &d);
            for i in 0..3 { for j in 0..3 { g[i][j] += ge[i][j]; } }
        }
        let hh = terms_hess(&self.harmonics(), &r);
        for i in 0..3 { for j in 0..3 { g[i][j] += hh[i][j]; } }
        g[0][0] += 1.0; g[1][1] += 1.0; // centrifugal diag(1,1,0)
        let mut a = [[0.0;6];6];
        for k in 0..3 { a[k][k+3] = 1.0; }
        for i in 0..3 { for j in 0..3 { a[i+3][j] = g[i][j]; } }
        a[3][4] = 2.0; a[4][3] = -2.0; // Coriolis
        a
    }
```

- [ ] **Step 4: Run to verify pass** — `cargo test -p elfo-core jac_tests` → PASS

- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat: analytic acceleration Jacobian"`

---

### Task 6: DP54 adaptive integrator

**Files:**
- Create: `crates/elfo-core/src/integrator.rs`; Modify: `lib.rs`

**Interfaces:**
- Consumes: `ForceModel::eom`, `elements::*` (tests only)
- Produces:
  - `pub struct Dp54 { pub rtol: f64, pub atol: f64 }` with `impl Default` (1e-12/1e-12)
  - `pub fn propagate(&self, f: &impl Fn(f64, &[f64]) -> Vec<f64>, y0: &[f64], t0: f64, tf: f64, sample_times: &[f64], observer: &mut impl FnMut(f64, &[f64])) -> Vec<f64>` — adaptive steps, clamped so integration lands exactly on each `sample_times` entry (sorted, within [t0,tf]); observer called at each sample time. Works for any state dimension (6 or 42).
  - `pub fn propagate_fixed(&self, f: ..., y0: &[f64], t0: f64, tf: f64, h: f64) -> Vec<f64>` — fixed-step (order test).
  - Butcher tableau: Dormand–Prince 5(4), FSAL, PI-free simple controller: `err = RMS(e_i / (atol + rtol·max(|y_i|,|ŷ_i|)))`, accept if err ≤ 1, `h ← h·clamp(0.9·err^(−1/5), 0.2, 5.0)`.

- [ ] **Step 1: Write the failing tests**

```rust
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
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p elfo-core integrator` → FAIL

- [ ] **Step 3: Implement**

```rust
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
```

- [ ] **Step 4: Run to verify pass** — `cargo test -p elfo-core integrator` → PASS (energy test may need `6.28` → shorter arc if the arbitrary IC impacts the Moon; if it fails with NaN, change y0 to `[0.02, 0.0, 0.01, 0.0, -0.65, 0.25]` which is safely elliptic and non-impacting — verify `|r|` stays above `R_MOON_ND` by printing min radius in a debug run)

- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat: DP54 adaptive integrator with exact sample-time hits"`

---

### Task 7: STM propagation

**Files:**
- Modify: `crates/elfo-core/src/integrator.rs` (append free function)

**Interfaces:**
- Consumes: `Dp54::propagate`, `ForceModel::{eom, accel_jacobian}`
- Produces: `pub fn propagate_stm(integ: &Dp54, fm: &ForceModel, y0: &[f64;6], t0: f64, tf: f64) -> ([f64;6], nalgebra::SMatrix<f64,6,6>)` — integrates the 42-dim system (state + STM columns, Φ(t0)=I), `Φ̇ = A(x)Φ`.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod stm_tests {
    use super::*;
    use crate::forces::ForceModel;
    #[test]
    fn stm_matches_finite_difference_of_flow() {
        let fm = ForceModel { j2: true, c22: true, j3: true, earth: true };
        let y0 = [0.02, 0.003, 0.012, 0.05, -0.6, 0.28];
        let integ = Dp54::default();
        let (_, phi) = propagate_stm(&integ, &fm, &y0, 0.0, 0.4);
        let h = 1e-7;
        let f = |_t: f64, y: &[f64]| fm.eom(&[y[0],y[1],y[2],y[3],y[4],y[5]]).to_vec();
        for col in 0..6 {
            let (mut yp, mut ym) = (y0, y0); yp[col] += h; ym[col] -= h;
            let fp = integ.propagate(&f, &yp, 0.0, 0.4, &[], &mut |_,_|{});
            let fm_ = integ.propagate(&f, &ym, 0.0, 0.4, &[], &mut |_,_|{});
            for row in 0..6 {
                let fd = (fp[row] - fm_[row]) / (2.0 * h);
                assert!((phi[(row, col)] - fd).abs() < (1e-5 * fd.abs()).max(1e-5),
                    "Φ[{row},{col}] {} vs fd {}", phi[(row,col)], fd);
            }
        }
    }
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p elfo-core stm_tests` → FAIL

- [ ] **Step 3: Implement** (append to `integrator.rs`)

```rust
use crate::forces::ForceModel;
use nalgebra::SMatrix;

pub fn propagate_stm(integ: &Dp54, fm: &ForceModel, y0: &[f64;6], t0: f64, tf: f64)
    -> ([f64;6], SMatrix<f64,6,6>) {
    let mut z0 = vec![0.0; 42];
    z0[..6].copy_from_slice(y0);
    for k in 0..6 { z0[6 + k*6 + k] = 1.0; } // Φ = I, column-major blocks
    let f = |_t: f64, z: &[f64]| {
        let s = [z[0],z[1],z[2],z[3],z[4],z[5]];
        let a = fm.accel_jacobian(&s);
        let mut dz = vec![0.0; 42];
        dz[..6].copy_from_slice(&fm.eom(&s));
        for col in 0..6 {
            for row in 0..6 {
                let mut acc = 0.0;
                for m in 0..6 { acc += a[row][m] * z[6 + col*6 + m]; }
                dz[6 + col*6 + row] = acc;
            }
        }
        dz
    };
    let zf = integ.propagate(&f, &z0, t0, tf, &[], &mut |_,_|{});
    let mut yf = [0.0;6]; yf.copy_from_slice(&zf[..6]);
    let mut phi = SMatrix::<f64,6,6>::zeros();
    for col in 0..6 { for row in 0..6 { phi[(row,col)] = zf[6 + col*6 + row]; } }
    (yf, phi)
}
```

- [ ] **Step 4: Run to verify pass** — `cargo test -p elfo-core stm_tests` → PASS

- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat: state transition matrix propagation"`

---

### Task 8: Lagrange points + planar linearization seed

**Files:**
- Create: `crates/elfo-core/src/lagrange.rs`; Modify: `lib.rs`

**Interfaces:**
- Consumes: `ForceModel::{accel, accel_jacobian}`
- Produces:
  - `pub fn l1_x() -> f64` / `pub fn l2_x() -> f64` — Moon-centered x of L1/L2 for the pure CR3BP (`ForceModel { earth: true, harmonics off }`), via bisection on `accel([x,0,0,0,0,0])[0] = 0` over x ∈ (−0.5, −0.05) for L1 and (0.05, 0.5) for L2.
  - `pub fn lyapunov_seed(amplitude: f64) -> ([f64;6], f64)` — planar Lyapunov initial state near L1 and its linear period `2π/ω`, from the collinear-point planar linearization: with `Uxx = G[0][0]`, `Uyy = G[1][1]` (from `accel_jacobian` at L1, rows/cols 3/4 minus Coriolis — i.e., the position-gradient block), solve `s² + (Uxx + Uyy − 4)s + Uxx·Uyy = 0` for the positive root `s = ω²`; eigen-relation `B/A = −(Uxx + ω²)/(2ω)`; seed `[x_L1 + A, 0, 0, 0, B·ω, 0]`.

- [ ] **Step 1: Write the failing tests**

```rust
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
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p elfo-core lagrange` → FAIL

- [ ] **Step 3: Implement**

```rust
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
```

- [ ] **Step 4: Run to verify pass** — `cargo test -p elfo-core lagrange` → PASS

- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat: Lagrange points and planar Lyapunov linear seed"`

---

### Task 9: Multiple-shooting corrector

**Files:**
- Create: `crates/elfo-core/src/shooting.rs`; Modify: `lib.rs`; add `nalgebra` feature needs nothing extra.

**Interfaces:**
- Consumes: `propagate_stm`, `Dp54`, `ForceModel`
- Produces:
  - `pub struct PeriodicOrbit { pub nodes: Vec<[f64;6]>, pub period: f64, pub residual: f64, pub segment_stms: Vec<nalgebra::SMatrix<f64,6,6>> }`
  - `pub enum Constraint { None, Arclength { tangent: nalgebra::DVector<f64>, prev: nalgebra::DVector<f64>, ds: f64 } }`
  - `pub fn correct(fm: &ForceModel, nodes: &[[f64;6]], period: f64, constraint: &Constraint) -> Result<PeriodicOrbit, String>`
  - `pub fn pack(nodes: &[[f64;6]], period: f64) -> nalgebra::DVector<f64>` / `pub fn unpack(u: &nalgebra::DVector<f64>, m: usize) -> (Vec<[f64;6]>, f64)`
  - `pub fn build_system(fm: &ForceModel, u: &DVector<f64>, m: usize, constraint: &Constraint) -> (DVector<f64>, DMatrix<f64>, Vec<SMatrix<f64,6,6>>)` — residual R, Jacobian J, per-segment STMs.
  - `pub fn seed_nodes(fm: &ForceModel, state0: &[f64;6], period: f64, m: usize) -> Vec<[f64;6]>` — propagate `state0` and record states at `i·period/m`.

  Unknowns `U = (X_0..X_{m−1}, T)` (6m+1). Residual rows: continuity `φ_{T/m}(X_i) − X_{i+1}` for i<m−1 (6 each), periodicity `φ_{T/m}(X_{m−1}) − X_0` (6), anchor `X_0.y = 0` (1), optional arclength `tangent·(U − prev) − ds` (1). Jacobian blocks: `∂cont_i/∂X_i = Φ_i`, `∂cont_i/∂X_{i+1} = −I`, `∂/∂T = f(seg_end)/m`; anchor row: 1 at X_0.y column; arclength row = tangentᵀ. Solve `J ΔU = −R` by SVD least-squares (`svd.solve(&(-r), 1e-11)`) — handles the square-but-singular no-arclength case (min-norm) and the overdetermined arclength case (least squares). Damping: accept step if ‖R‖∞ decreases, else halve α up to 6 times. Converged when ‖R‖∞ < 1e-10; max 25 iterations.

- [ ] **Step 1: Write the failing test** (DRO — stable, converges from a crude two-body circular seed)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{constants::*, forces::ForceModel};
    #[test]
    fn dro_converges_from_circular_seed() {
        let fm = ForceModel { j2: false, c22: false, j3: false, earth: true };
        let x0 = 0.04;
        let vc = (MU_MOON_ND / x0).sqrt();
        let seed = [x0, 0.0, 0.0, 0.0, -(vc + x0), 0.0]; // retrograde in rotating frame
        let t0 = std::f64::consts::TAU / (vc / x0 + 1.0);
        // m=4: empirically required — m=3 stalls at residual ~0.021 for this seed
        let nodes = seed_nodes(&fm, &seed, t0, 4);
        let orbit = correct(&fm, &nodes, t0, &Constraint::None).expect("DRO should converge");
        assert!(orbit.residual < 1e-10);
        assert!((orbit.period - t0).abs() < 0.2 * t0);
        // periodicity double-check by direct propagation
        let integ = crate::integrator::Dp54::default();
        let f = |_t: f64, y: &[f64]| fm.eom(&[y[0],y[1],y[2],y[3],y[4],y[5]]).to_vec();
        let yf = integ.propagate(&f, &orbit.nodes[0], 0.0, orbit.period, &[], &mut |_,_|{});
        for k in 0..6 { assert!((yf[k] - orbit.nodes[0][k]).abs() < 1e-8, "k={k}"); }
    }
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p elfo-core shooting` → FAIL

- [ ] **Step 3: Implement**

```rust
use nalgebra::{DMatrix, DVector, SMatrix};
use crate::{forces::ForceModel, integrator::{Dp54, propagate_stm}};

pub struct PeriodicOrbit {
    pub nodes: Vec<[f64;6]>,
    pub period: f64,
    pub residual: f64,
    pub segment_stms: Vec<SMatrix<f64,6,6>>,
}

pub enum Constraint {
    None,
    Arclength { tangent: DVector<f64>, prev: DVector<f64>, ds: f64 },
}

pub fn pack(nodes: &[[f64;6]], period: f64) -> DVector<f64> {
    let m = nodes.len();
    let mut u = DVector::zeros(6*m + 1);
    for (i, n) in nodes.iter().enumerate() { for k in 0..6 { u[6*i+k] = n[k]; } }
    u[6*m] = period;
    u
}
pub fn unpack(u: &DVector<f64>, m: usize) -> (Vec<[f64;6]>, f64) {
    let mut nodes = vec![[0.0;6]; m];
    for i in 0..m { for k in 0..6 { nodes[i][k] = u[6*i+k]; } }
    (nodes, u[6*m])
}

pub fn seed_nodes(fm: &ForceModel, state0: &[f64;6], period: f64, m: usize) -> Vec<[f64;6]> {
    let integ = Dp54::default();
    let f = |_t: f64, y: &[f64]| fm.eom(&[y[0],y[1],y[2],y[3],y[4],y[5]]).to_vec();
    let times: Vec<f64> = (1..m).map(|i| period * i as f64 / m as f64).collect();
    let mut nodes = vec![*state0];
    integ.propagate(&f, state0, 0.0, period, &times, &mut |_, y| {
        let mut n = [0.0;6]; n.copy_from_slice(&y[..6]); nodes.push(n);
    });
    nodes.truncate(m);
    nodes
}

pub fn build_system(fm: &ForceModel, u: &DVector<f64>, m: usize, constraint: &Constraint)
    -> (DVector<f64>, DMatrix<f64>, Vec<SMatrix<f64,6,6>>) {
    let (nodes, period) = unpack(u, m);
    let tau = period / m as f64;
    let integ = Dp54::default();
    let nrows = 6*m + 1 + if matches!(constraint, Constraint::Arclength{..}) { 1 } else { 0 };
    let mut r = DVector::zeros(nrows);
    let mut j = DMatrix::zeros(nrows, 6*m + 1);
    let mut stms = Vec::with_capacity(m);
    for i in 0..m {
        let (endstate, phi) = propagate_stm(&integ, fm, &nodes[i], 0.0, tau);
        let fend = fm.eom(&endstate_arr(&endstate));
        let inext = (i + 1) % m;
        for row in 0..6 {
            r[6*i+row] = endstate[row] - nodes[inext][row];
            for col in 0..6 { j[(6*i+row, 6*i+col)] = phi[(row, col)]; }
            j[(6*i+row, 6*inext+row)] -= 1.0;
            j[(6*i+row, 6*m)] = fend[row] / m as f64;
        }
        stms.push(phi);
    }
    r[6*m] = nodes[0][1]; // anchor: y_0 = 0
    j[(6*m, 1)] = 1.0;
    if let Constraint::Arclength { tangent, prev, ds } = constraint {
        r[6*m+1] = tangent.dot(&(u - prev)) - ds;
        for col in 0..6*m+1 { j[(6*m+1, col)] = tangent[col]; }
    }
    (r, j, stms)
}
fn endstate_arr(y: &[f64;6]) -> [f64;6] { *y }

pub fn correct(fm: &ForceModel, nodes: &[[f64;6]], period: f64, constraint: &Constraint)
    -> Result<PeriodicOrbit, String> {
    let m = nodes.len();
    let mut u = pack(nodes, period);
    let (mut r, mut j, mut stms) = build_system(fm, &u, m, constraint);
    for _iter in 0..25 {
        let rn = r.amax();
        if rn < 1e-10 {
            let (nodes, period) = unpack(&u, m);
            return Ok(PeriodicOrbit { nodes, period, residual: rn, segment_stms: stms });
        }
        let du = j.clone().svd(true, true).solve(&(-&r), 1e-11)
            .map_err(|e| e.to_string())?;
        let mut alpha = 1.0;
        let mut accepted = false;
        for _ in 0..6 {
            let ut = &u + &du * alpha;
            let (rt, jt, st) = build_system(fm, &ut, m, constraint);
            if rt.amax() < rn {
                u = ut; r = rt; j = jt; stms = st; accepted = true; break;
            }
            alpha *= 0.5;
        }
        if !accepted { return Err(format!("stalled at residual {rn}")); }
    }
    Err("max iterations".into())
}
```
Note: the anchor `y_0 = 0` assumes the seed's first node starts at (or near) a y = 0 crossing — all seeds in this plan do (DRO/Lyapunov start on the x-axis; ELFO seeds are built at true anomaly 180° then rotated so node 0 has y ≈ 0, see Task 13). The damping check `rt.amax() < rn` re-propagates; acceptable cost.

- [ ] **Step 4: Run to verify pass** — `cargo test -p elfo-core shooting` → PASS (runtime tens of seconds is normal; if the DRO stalls, loosen seed period ±20% or increase m to 4)

- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat: multiple-shooting periodic-orbit corrector"`

---

### Task 10: CR3BP validation — Lyapunov + 9:2 NRHO-class orbit

**Files:**
- Create: `crates/elfo-core/tests/cr3bp_validation.rs` (integration test)

**Interfaces:**
- Consumes: `lagrange::lyapunov_seed`, `shooting::{correct, seed_nodes, Constraint}`

- [ ] **Step 1: Write the failing tests**

```rust
use elfo_core::{forces::ForceModel, lagrange::lyapunov_seed,
    shooting::{correct, seed_nodes, Constraint}, constants::*};

fn cr3bp() -> ForceModel { ForceModel { j2: false, c22: false, j3: false, earth: true } }

#[test]
fn l1_lyapunov_converges_from_linear_seed() {
    let (seed, t_lin) = lyapunov_seed(1e-3);
    let nodes = seed_nodes(&cr3bp(), &seed, t_lin, 3);
    let orbit = correct(&cr3bp(), &nodes, t_lin, &Constraint::None).expect("converge");
    assert!(orbit.residual < 1e-10);
    assert!((orbit.period - t_lin).abs() < 0.1 * t_lin);
}

#[test]
fn nrho_92_class_orbit_converges_near_published_seed() {
    // Approximate 9:2 L2 southern NRHO (JPL catalog vicinity), barycentric rotating
    // → Moon-centered by subtracting (1−μ). Loose tolerances absorb seed imprecision.
    let seed = [1.02134 - (1.0 - MU), 0.0, -0.18162, 0.0, -0.10176, 0.0];
    let t0 = 1.5092; // ≈ 6.56 days: 9 revs per 2 synodic months
    let nodes = seed_nodes(&cr3bp(), &seed, t0, 6);
    let orbit = correct(&cr3bp(), &nodes, t0, &Constraint::None).expect("converge");
    assert!(orbit.residual < 1e-10);
    assert!(orbit.period > 1.45 && orbit.period < 1.57);
    // perilune radius in the published ballpark (≈ 3,200 km): accept 1,800–8,000 km
    let integ = elfo_core::integrator::Dp54::default();
    let fm = cr3bp();
    let f = |_t: f64, y: &[f64]| fm.eom(&[y[0],y[1],y[2],y[3],y[4],y[5]]).to_vec();
    let mut rmin = f64::MAX;
    let times: Vec<f64> = (0..2000).map(|i| orbit.period * i as f64 / 2000.0).collect();
    integ.propagate(&f, &orbit.nodes[0], 0.0, orbit.period, &times, &mut |_, y| {
        rmin = rmin.min((y[0]*y[0]+y[1]*y[1]+y[2]*y[2]).sqrt());
    });
    let rmin_km = nd_to_km(rmin);
    assert!(rmin_km > 1800.0 && rmin_km < 8000.0, "perilune {rmin_km} km");
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p elfo-core --test cr3bp_validation` → FAIL (file doesn't exist yet → create; then it should actually PASS if Tasks 8–9 are correct)

- [ ] **Step 3: Run to verify pass** — `cargo test -p elfo-core --test cr3bp_validation` → PASS. If the NRHO diverges: the seed digits are approximate by design — retry with m = 8 and t0 ∈ {1.45, 1.51, 1.55}; if it still fails, investigate the corrector (this is a validation gate, not a tunable).

- [ ] **Step 4: Commit** — `git add -A && git commit -m "test: CR3BP validation against Lyapunov and 9:2 NRHO-class orbits"`

---

### Task 11: Stability indices

**Files:**
- Create: `crates/elfo-core/src/stability.rs`; Modify: `lib.rs`

**Interfaces:**
- Consumes: `PeriodicOrbit::segment_stms`
- Produces:
  - `pub fn monodromy(orbit: &PeriodicOrbit) -> nalgebra::SMatrix<f64,6,6>` — product `Φ_{m−1}···Φ_0`
  - `pub fn stability_indices(mono: &SMatrix<f64,6,6>) -> (f64, f64)` — complex eigenvalues via `DMatrix::complex_eigenvalues()`; discard the two closest to 1+0i; greedily pair remaining four by minimizing `|λᵢ·λⱼ − 1|`; return `ν = Re(λ + 1/λ)/2` for each pair, ordered ν₁ ≥ ν₂ by |ν|.

- [ ] **Step 1: Write the failing tests**

```rust
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
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p elfo-core stability` → FAIL

- [ ] **Step 3: Implement**

```rust
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
```

- [ ] **Step 4: Run to verify pass** — `cargo test -p elfo-core stability` → PASS

- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat: monodromy matrix and stability indices"`

---

### Task 12: Pseudo-arclength continuation

**Files:**
- Create: `crates/elfo-core/src/continuation.rs`; Modify: `lib.rs`

**Interfaces:**
- Consumes: `shooting::{build_system, correct, pack, unpack, Constraint, PeriodicOrbit}`
- Produces:
  - `pub fn tangent(fm: &ForceModel, orbit: &PeriodicOrbit) -> nalgebra::DVector<f64>` — right-singular vector of the smallest singular value of the no-arclength Jacobian (square, singular by exactly the family direction once the anchor kills phase), normalized.
  - `pub fn continue_family(fm: &ForceModel, first: PeriodicOrbit, steps: usize, ds0: f64, direction: f64) -> Vec<PeriodicOrbit>` — predictor `U + ds·t̂`, corrector with `Constraint::Arclength`; tangent sign kept consistent (`dot(t̂ₖ, t̂ₖ₋₁) > 0`, else flip); adaptive ds (`≤4` iters → ×1.3 capped at 4·ds0; failure → ÷2, retry, floor ds0/64 then stop); returns members including `first`. Stops early on corrector failure at floor or if any node's radius < R_MOON_ND (impact — record and stop direction).

- [ ] **Step 1: Write the failing test**

```rust
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
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p elfo-core continuation` → FAIL

- [ ] **Step 3: Implement**

```rust
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
```
(Adaptive growth on easy convergence is folded into the Ok arm; the "≤4 iters" refinement requires `correct` to report iteration count — skip that refinement, the simple version above is sufficient.)

- [ ] **Step 4: Run to verify pass** — `cargo test -p elfo-core continuation` → PASS

- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat: pseudo-arclength continuation"`

---

### Task 13: ELFO seeds + first frozen family (physics milestone)

**Files:**
- Create: `crates/elfo-core/src/seeds.rs`; Modify: `lib.rs`

**Interfaces:**
- Consumes: `elements::{Coe, coe_to_rv, inertial_to_rotating}`, `constants`, `ForceModel`
- Produces:
  - `pub fn elfo_seed(n_revs: u32, fm: &ForceModel) -> ([f64;6], f64)` — initial rotating-frame state + closure-period guess `T₀ = 2π` (one frame period; N Kepler revs fit exactly at the resonant semi-major axis `a = (μm/N²)^(1/3)`).
  - Seed elements: `ω = 90°` (apoapsis dwell over the south pole), `Ω = 90°`, `ν = 180°` (start at apoapsis, far from periapsis sensitivity). Eccentricity/inclination by force model:
    - `fm.earth == true` (Kozai-dominated): `e = min(0.711, 0.85·(1 − (R_MOON_ND + km_to_nd(200.0))/a))`, then `i = acos(sqrt(0.6·(1 − e²)))` (the Lidov–Kozai frozen relation `e = sqrt(1 − (5/3)cos²i)` inverted; 0.711 is the i = 57° value).
    - `fm.earth == false` (J2/J3 frozen, near-circular): `i = 57°.to_radians()`, `e = MOON_J3 · R_MOON_ND · i.sin() / (2.0 · MOON_J2 · a)`.
  - After building the rotating-frame state, rotate the whole state about +z so that node 0 has y = 0 exactly (satisfies the corrector anchor): angle `θ = atan2(y, x)` of the position; apply `Rz(−θ)` to position AND velocity (rotating the orbit's node — an equally valid seed, since seeds are approximate anyway).

- [ ] **Step 1: Write the failing test** (the milestone: a full-model N = 25 frozen family member converges)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{constants::*, elements::rv_to_coe, elements::rotating_to_inertial,
        forces::ForceModel, shooting::{correct, seed_nodes, Constraint}};
    #[test]
    fn n25_full_model_elfo_converges() {
        let fm = ForceModel { j2: true, c22: true, j3: true, earth: true };
        let (seed, t0) = elfo_seed(25, &fm);
        let nodes = seed_nodes(&fm, &seed, t0, 25);
        let orbit = correct(&fm, &nodes, t0, &Constraint::None).expect("ELFO must converge");
        assert!(orbit.residual < 1e-10);
        assert!((orbit.period - std::f64::consts::TAU).abs() < 0.3);
        // eccentric, inclined, periapsis above surface
        let (ri, vi) = rotating_to_inertial(
            &[orbit.nodes[0][0], orbit.nodes[0][1], orbit.nodes[0][2]],
            &[orbit.nodes[0][3], orbit.nodes[0][4], orbit.nodes[0][5]], 0.0);
        let coe = rv_to_coe(&ri, &vi, MU_MOON_ND);
        assert!(coe.e > 0.4 && coe.e < 0.85, "e = {}", coe.e);
        assert!(coe.i > 0.6, "i = {} rad", coe.i);
        assert!(coe.a * (1.0 - coe.e) > R_MOON_ND, "periapsis below surface");
    }

    #[test]
    fn classical_frozen_seed_stays_bounded_without_correction() {
        // Spec validation item 4 (ELFO half): the *uncorrected* classical
        // frozen-orbit geometry, propagated for ~3 months in the full model,
        // shows bounded libration — periapsis radius wanders but does not
        // secularly drift or impact. This validates the physics against the
        // classical (Ely-style) frozen condition, independent of the corrector.
        let fm = ForceModel { j2: true, c22: true, j3: true, earth: true };
        let (seed, t0) = elfo_seed(25, &fm);
        let integ = crate::integrator::Dp54::default();
        let f = |_t: f64, y: &[f64]| fm.eom(&[y[0],y[1],y[2],y[3],y[4],y[5]]).to_vec();
        let total = 3.0 * t0; // ~3 sidereal months, ~75 revs
        let times: Vec<f64> = (0..6000).map(|i| total * i as f64 / 6000.0).collect();
        let (mut prev_r, mut falling) = (f64::MAX, false);
        let mut peri_radii: Vec<f64> = Vec::new();
        integ.propagate(&f, &seed, 0.0, total, &times, &mut |_, y| {
            let r = (y[0]*y[0]+y[1]*y[1]+y[2]*y[2]).sqrt();
            assert!(r > R_MOON_ND, "seed trajectory impacted the Moon");
            if falling && r > prev_r { peri_radii.push(prev_r); } // local min = periapsis pass
            falling = r < prev_r;
            prev_r = r;
        });
        assert!(peri_radii.len() >= 60, "expected ~75 periapsis passes, got {}", peri_radii.len());
        let (lo, hi) = peri_radii.iter().fold((f64::MAX, f64::MIN), |(a, b), &r| (a.min(r), b.max(r)));
        assert!((hi - lo) / lo < 0.25, "periapsis radius wanders {:.1}% — not frozen", 100.0 * (hi - lo) / lo);
    }
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p elfo-core seeds` → FAIL (module missing)

- [ ] **Step 3: Implement**

```rust
use crate::constants::*;
use crate::elements::{coe_to_rv, inertial_to_rotating, Coe};
use crate::forces::ForceModel;
use std::f64::consts::{PI, TAU};

pub fn elfo_seed(n_revs: u32, fm: &ForceModel) -> ([f64;6], f64) {
    let n = n_revs as f64;
    let a = (MU_MOON_ND / (n * n)).cbrt();
    let (e, i) = if fm.earth {
        let e_geo = 0.85 * (1.0 - (R_MOON_ND + km_to_nd(200.0)) / a);
        let e = e_geo.min(0.711).max(0.05);
        (e, (0.6 * (1.0 - e * e)).sqrt().acos())
    } else {
        let i = 57f64.to_radians();
        let e = (MOON_J3 * R_MOON_ND * i.sin() / (2.0 * MOON_J2 * a)).min(0.3);
        (e, i)
    };
    let coe = Coe { a, e, i, raan: PI / 2.0, aop: PI / 2.0, ta: PI };
    let (ri, vi) = coe_to_rv(&coe, MU_MOON_ND);
    let (r0, v0) = inertial_to_rotating(&ri, &vi, 0.0);
    // rotate about z so node 0 sits at y = 0 (anchor compatibility)
    let theta = f64::atan2(r0[1], r0[0]);
    let (c, s) = ((-theta).cos(), (-theta).sin());
    let rz = |x: &[f64;3]| [c*x[0]-s*x[1], s*x[0]+c*x[1], x[2]];
    let (rr, vr) = (rz(&r0), rz(&v0));
    ([rr[0], rr[1], rr[2], vr[0], vr[1], vr[2]], TAU)
}
```

- [ ] **Step 4: Run to verify pass** — `cargo test -p elfo-core seeds` → PASS. **This is the highest-risk test in the plan.** Runtime may be minutes (25 segments × STM). If the corrector diverges or stalls, try in order: (a) e scaled by 0.9, (b) i ± 5°, (c) Ω = 0 instead of 90°, (d) m = 50 segments (2 per rev), (e) N = 20 instead of 25. Record whatever combination converges as the new defaults in `elfo_seed` and note the change in the commit message. If nothing converges, STOP and escalate to the human partner with the observed residual history — do not paper over.

- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat: ELFO frozen-orbit seeds; first full-model family member converges"`

---

### Task 14: Catalog config, QA, writers

**Files:**
- Create: `crates/elfo-catalog/Cargo.toml`, `src/main.rs` (stub), `src/config.rs`, `src/qa.rs`, `src/writer.rs`; Modify root `Cargo.toml` members list.

**Interfaces:**
- Consumes: everything in `elfo-core`
- Produces (consumed by Task 15 and the web app):
  - `config::CatalogConfig { pub combos: Vec<ComboCfg>, pub resonances: Vec<u32>, pub members_per_direction: usize, pub ds0: f64 }` with `ComboCfg { pub id: String, pub name: String, pub force_model: ForceModel }` — from TOML.
  - `qa::check_member(m: &MemberOut, prev: Option<&MemberOut>) -> Vec<String>` — flags: `residual > 1e-9`; `r_peri_km < R_MOON_KM + 50`; vs prev: `|Δperiod_nd| > 0.05`, `|Δν₁| > 5.0`.
  - `writer::MemberOut { pub index: usize, pub state0: [f64;6], pub period_s: f64, pub period_nd: f64, pub elements: ElementsOut, pub nu1: f64, pub nu2: f64, pub r_peri_km: f64, pub r_apo_km: f64, pub residual: f64, pub traj: String }`, `ElementsOut { pub a_km: f64, pub e: f64, pub i_deg: f64, pub omega_deg: f64, pub raan_deg: f64 }` — all serde-Serialize with these exact JSON field names.
  - `writer::write_catalog(out_dir: &Path, combos: Vec<ComboOut>, provenance: Provenance) -> anyhow::Result<()>` — writes `catalog.json` (`{ schema_version: 1, generated: {date, git_hash}, constants: {...}, combos: [...] }`), per-member `.f32` (little-endian f32 xyz km triples), per-family `preview.f32` (each member decimated to ≤1000 points, concatenated, plus `preview_counts: Vec<u32>` recorded in the family JSON).
  - `ComboOut { pub id: String, pub name: String, pub terms: ForceModel, pub families: Vec<FamilyOut> }`, `FamilyOut { pub resonance_n: u32, pub members: Vec<MemberOut>, pub preview: String, pub preview_counts: Vec<u32> }`, `Provenance { pub date: String, pub git_hash: String }`.

`elfo-catalog/Cargo.toml` dependencies: `elfo-core = { path = "../elfo-core" }`, `serde`, `serde_json`, `toml = "0.8"`, `rayon = "1"`, `clap = { version = "4", features = ["derive"] }`, `chrono = "0.4"`, `anyhow = "1"`; dev-dependency `tempfile = "3"`.

- [ ] **Step 1: Write the failing tests** (in `qa.rs` and `writer.rs`)

```rust
// qa.rs tests
#[test]
fn qa_flags_planted_violations() {
    let good = mk_member(0, 1e-10, 1900.0, 6.28, 1.2);   // helper builds MemberOut
    let bad  = mk_member(1, 1e-6, 1700.0, 6.40, 9.0);
    assert!(check_member(&good, None).is_empty());
    let flags = check_member(&bad, Some(&good));
    assert!(flags.iter().any(|f| f.contains("residual")));
    assert!(flags.iter().any(|f| f.contains("periapsis")));
    assert!(flags.iter().any(|f| f.contains("nu1")));
}
// writer.rs tests
#[test]
fn f32_round_trip_and_catalog_json_shape() {
    let dir = tempfile::tempdir().unwrap();
    // write a 3-point trajectory, read bytes back, check little-endian f32 km
    let pts = vec![[1000.0f64, -2000.0, 3000.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]];
    write_f32(&dir.path().join("t.f32"), &pts).unwrap();
    let bytes = std::fs::read(dir.path().join("t.f32")).unwrap();
    assert_eq!(bytes.len(), 3 * 3 * 4);
    assert_eq!(f32::from_le_bytes(bytes[0..4].try_into().unwrap()), 1000.0);
    // catalog.json minimal shape
    write_catalog(dir.path(), vec![], Provenance { date: "d".into(), git_hash: "h".into() }).unwrap();
    let v: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.path().join("catalog.json")).unwrap()).unwrap();
    assert_eq!(v["schema_version"], 1);
    assert!(v["combos"].as_array().unwrap().is_empty());
}
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p elfo-catalog` → FAIL

- [ ] **Step 3: Implement** `config.rs` (serde+toml derive), `qa.rs`, `writer.rs` (including `pub fn write_f32(path: &Path, pts: &[[f64;3]]) -> anyhow::Result<()>` and `constants` block in JSON echoing `GM_EARTH_KM3S2, GM_MOON_KM3S2, A_EM_KM, R_MOON_KM, MOON_J2, MOON_C22, MOON_J3` with a `source` string). `main.rs` stub: `fn main() {}`. Include the `mk_member` test helper.

- [ ] **Step 4: Run to verify pass** — `cargo test -p elfo-catalog` → PASS

- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat: catalog config, QA checks, and file writers"`

---

### Task 15: Catalog CLI end-to-end

**Files:**
- Modify: `crates/elfo-catalog/src/main.rs`; Create: `catalog.toml` (real config), `crates/elfo-catalog/tests/end_to_end.rs`

**Interfaces:**
- Produces: `elfo-catalog gen --config <toml> --out <dir>` CLI. For each (combo, N) pair in parallel (rayon `par_iter` over pairs): seed via `elfo_seed`, correct (m = N segments), `continue_family` in both directions (`direction: ±1.0`, `members_per_direction` each), concatenate (reverse the −1 side, dedupe the shared first member, index 0..len), sample each member's trajectory at `100·N` uniform times over its period (positions → km), compute elements at the minimum-radius sample (via `rotating_to_inertial` + `rv_to_coe`), stability indices, r_peri/r_apo from samples, QA flags (log to stderr; drop members failing residual/periapsis, keep drift warnings), write everything. A (combo, N) whose seed fails to converge is recorded as an absent family (combo present, family missing) with a stderr note — **absence is data, not an error**.
- **File layout under the out dir (exact names — the web tests in Task 16 assume them):** `catalog.json` at the root; per family a directory `{combo_id}/n{resonance}/` containing `{member_index}.f32` (0-based) and `preview.f32`; `traj` and `preview` fields in the JSON store these paths relative to the catalog root (e.g. `full/n25/0.f32`).
- `catalog.toml` (committed, the real config):

```toml
members_per_direction = 20
ds0 = 5e-4
resonances = [20, 25, 30, 35, 40, 45, 50, 60, 70]

[[combos]]
id = "full"
name = "J2 + C22 + J3 + Earth"
force_model = { j2 = true, c22 = true, j3 = true, earth = true }

[[combos]]
id = "no-c22"
name = "J2 + J3 + Earth (C22 off)"
force_model = { j2 = true, c22 = false, j3 = true, earth = true }

[[combos]]
id = "no-j3"
name = "J2 + C22 + Earth (J3 off)"
force_model = { j2 = true, c22 = true, j3 = false, earth = true }

[[combos]]
id = "no-earth"
name = "J2 + C22 + J3 (Earth off)"
force_model = { j2 = true, c22 = true, j3 = true, earth = false }
```

- [ ] **Step 1: Write the failing end-to-end test** (`tests/end_to_end.rs`) — tiny config: one combo (`full`), `resonances = [25]`, `members_per_direction = 2`, written to a tempdir TOML; run the generation function directly (expose `pub fn run(config: &Path, out: &Path) -> anyhow::Result<()>` from a `lib.rs` in elfo-catalog so the test avoids spawning a binary); assert: `catalog.json` parses, has 1 combo with 1 family of ≥3 members, every member's `.f32` exists with size `= 100·25·3·4` bytes... (size check: `100·N` samples × 3 × 4 bytes), `preview.f32` exists, all residuals < 1e-9.

- [ ] **Step 2: Run to verify failure** — `cargo test -p elfo-catalog --test end_to_end` → FAIL

- [ ] **Step 3: Implement** `main.rs`/`lib.rs`: clap parsing, `run()` orchestration as specified, provenance via `chrono::Utc::now().to_rfc3339()` and `git rev-parse --short HEAD` (fallback `"unknown"`).

- [ ] **Step 4: Run to verify pass** — `cargo test -p elfo-catalog --test end_to_end` (runtime: minutes — mark `#[ignore]` if it exceeds 10 min and run explicitly with `-- --ignored`) → PASS

- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat: catalog CLI generates families end to end"`

---

### Task 16: Web scaffold, data contract types, data loader

**Files:**
- Create: `web/package.json`, `web/tsconfig.json`, `web/vitest.config.ts`, `web/index.html`, `web/dev-catalog.toml`, `web/src/types.ts`, `web/src/data.ts`, `web/src/testFixtures.ts`, `web/src/data.test.ts`
- Delete (from the Vite template): `web/src/counter.ts`, `web/src/typescript.svg`, `web/public/vite.svg`
- Generate (git-ignored): `web/public/catalog/**`

**Interfaces:**
- Consumes: the `elfo-catalog gen --config <toml> --out <dir>` CLI from Task 15 and the on-disk contract it writes (`catalog.json` + `.f32` files).
- Produces (used by Tasks 17–22):
  - `types.ts`: `SCHEMA_VERSION = 1`; interfaces `Terms { j2: boolean; c22: boolean; j3: boolean; earth: boolean }`, `Elements { a_km: number; e: number; i_deg: number; omega_deg: number; raan_deg: number }`, `Member { index: number; state0: number[]; period_s: number; period_nd: number; elements: Elements; nu1: number; nu2: number; r_peri_km: number; r_apo_km: number; residual: number; traj: string }`, `Family { resonance_n: number; members: Member[]; preview: string; preview_counts: number[] }`, `Combo { id: string; name: string; terms: Terms; families: Family[] }`, `Catalog { schema_version: number; generated: { date: string; git_hash: string }; constants: Record<string, number | string>; combos: Combo[] }`; `assertCatalog(value: unknown): Catalog` (throws on bad shape).
  - `data.ts`: `joinUrl(base: string, path: string): string`, `parseF32(buf: ArrayBuffer): Float32Array`, `splitPreview(data: Float32Array, counts: number[]): Float32Array[]`, `loadCatalog(baseUrl: string): Promise<Catalog>`, `loadF32(url: string): Promise<Float32Array>`, `memberTrajectory(baseUrl: string, member: Member): Promise<Float32Array>`, `familyPreview(baseUrl: string, family: Family): Promise<Float32Array[]>`, `prefetchNeighbors(baseUrl: string, family: Family, index: number): void`, `clearCache(): void`.
  - `testFixtures.ts`: `makeMember(index: number, over?: Partial<Member>): Member`, `makeFamily(n: number, count: number): Family`, `makeCombo(id: string, name: string, terms: Terms, families: Family[]): Combo`, `makeCatalog(): Catalog`.
- The catalog is served by Vite from `web/public/catalog`, so the app's base URL is the literal string `'catalog'`.

- [ ] **Step 1: Scaffold the Vite app**

From the repo root:
```bash
npm create vite@latest web -- --template vanilla-ts
cd web
npm install
npm install three d3-scale
npm install -D vitest @types/three @types/d3-scale
rm -f src/counter.ts src/typescript.svg public/vite.svg
```
`@types/three` and `@types/d3-scale` are typings for the two libraries already in the Tech Stack, not new runtime dependencies. Keep whatever versions npm resolved.

Merge these scripts into `web/package.json` (leave `name`, `version`, `dependencies`, `devDependencies` as npm wrote them):
```json
  "scripts": {
    "dev": "vite",
    "build": "tsc --noEmit && vite build",
    "preview": "vite preview",
    "test": "vitest run",
    "test:watch": "vitest"
  }
```

Replace `web/tsconfig.json` with:
```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "moduleResolution": "bundler",
    "types": ["vite/client"],
    "skipLibCheck": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "noImplicitOverride": true
  },
  "include": ["src", "vitest.config.ts"]
}
```

Create `web/vitest.config.ts`:
```ts
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
  },
});
```

Replace `web/index.html` with (the three containers are the cockpit's grid areas; Tasks 18–21 fill them):
```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>ELFO Family Browser</title>
  </head>
  <body>
    <div id="app">
      <aside id="rail"></aside>
      <main id="stage"></main>
      <section id="plot"></section>
    </div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 2: Write the dev catalog config and generate a small catalog**

Create `web/dev-catalog.toml` (committed — it is a generator config, not generated data):
```toml
# Small catalog for web development: one combo, two resonances, 7 members each.
# Generation takes roughly a minute. The real config is /catalog.toml (Task 15).
members_per_direction = 3
ds0 = 5e-4
resonances = [25, 30]

[[combos]]
id = "full"
name = "J2 + C22 + J3 + Earth"
force_model = { j2 = true, c22 = true, j3 = true, earth = true }
```

Run from the repo root:
```bash
cargo run -p elfo-catalog --release -- gen --config web/dev-catalog.toml --out web/public/catalog
```
Expected: `web/public/catalog/catalog.json` exists, plus `web/public/catalog/full/n25/*.f32` and `full/n30/*.f32` including `preview.f32`. Verify with:
```bash
python3 -c "import json;c=json.load(open('web/public/catalog/catalog.json'));print(c['schema_version'],[ (f['resonance_n'],len(f['members'])) for f in c['combos'][0]['families']])"
```
Expected output shape: `1 [(25, 7), (30, 7)]` (member counts may be smaller if continuation stopped early — anything ≥ 3 is fine).

- [ ] **Step 3: Write the failing tests**

`web/src/testFixtures.ts`:
```ts
import type { Catalog, Combo, Family, Member, Terms } from './types';

export function makeMember(index: number, over: Partial<Member> = {}): Member {
  return {
    index,
    state0: [0.02, 0, 0.01, 0, -0.6, 0.3],
    period_s: 2360591,
    period_nd: 6.2831853,
    elements: { a_km: 6000 + index * 10, e: 0.6, i_deg: 57, omega_deg: 90, raan_deg: 90 },
    nu1: 1.2 + index * 0.1,
    nu2: -0.4,
    r_peri_km: 2400,
    r_apo_km: 9600,
    residual: 1e-11,
    traj: `full/n25/${index}.f32`,
    ...over,
  };
}

export function makeFamily(n: number, count: number): Family {
  return {
    resonance_n: n,
    members: Array.from({ length: count }, (_, i) => makeMember(i, { traj: `full/n${n}/${i}.f32` })),
    preview: `full/n${n}/preview.f32`,
    preview_counts: Array.from({ length: count }, () => 1000),
  };
}

export function makeCombo(id: string, name: string, terms: Terms, families: Family[]): Combo {
  return { id, name, terms, families };
}

export function makeCatalog(): Catalog {
  return {
    schema_version: 1,
    generated: { date: '2026-08-11T00:00:00Z', git_hash: 'abc1234' },
    constants: { R_MOON_KM: 1737.4, source: 'DE440 / GRGM1200' },
    combos: [
      makeCombo('full', 'J2 + C22 + J3 + Earth',
        { j2: true, c22: true, j3: true, earth: true },
        [makeFamily(25, 5), makeFamily(30, 4)]),
      makeCombo('no-c22', 'J2 + J3 + Earth (C22 off)',
        { j2: true, c22: false, j3: true, earth: true },
        [makeFamily(25, 7)]),
      makeCombo('no-earth', 'J2 + C22 + J3 (Earth off)',
        { j2: true, c22: true, j3: true, earth: false },
        [makeFamily(40, 3)]),
    ],
  };
}
```

`web/src/data.test.ts`:
```ts
import { afterEach, describe, expect, it, vi } from 'vitest';
import { assertCatalog } from './types';
import {
  clearCache, familyPreview, joinUrl, loadCatalog, loadF32,
  memberTrajectory, parseF32, splitPreview,
} from './data';
import { makeCatalog, makeMember } from './testFixtures';

function f32Buffer(values: number[]): ArrayBuffer {
  const buf = new ArrayBuffer(values.length * 4);
  const view = new DataView(buf);
  values.forEach((v, i) => view.setFloat32(i * 4, v, true));
  return buf;
}

afterEach(() => {
  clearCache();
  vi.unstubAllGlobals();
});

describe('joinUrl', () => {
  it('joins without doubling or dropping slashes', () => {
    expect(joinUrl('catalog', 'full/n25/0.f32')).toBe('catalog/full/n25/0.f32');
    expect(joinUrl('catalog/', '/full/n25/0.f32')).toBe('catalog/full/n25/0.f32');
    expect(joinUrl('', 'catalog.json')).toBe('catalog.json');
  });
});

describe('parseF32', () => {
  it('decodes little-endian float32 xyz triples regardless of host endianness', () => {
    const out = parseF32(f32Buffer([1000, -2000, 3000, 4, 5, 6]));
    expect(Array.from(out)).toEqual([1000, -2000, 3000, 4, 5, 6]);
    expect(out.length / 3).toBe(2);
  });
});

describe('splitPreview', () => {
  it('slices a concatenated preview by per-member point counts', () => {
    const data = parseF32(f32Buffer(Array.from({ length: 18 }, (_, i) => i)));
    const loops = splitPreview(data, [2, 4]);
    expect(loops).toHaveLength(2);
    expect(Array.from(loops[0])).toEqual([0, 1, 2, 3, 4, 5]);
    expect(loops[1].length).toBe(12);
    expect(loops[1][0]).toBe(6);
  });
});

describe('assertCatalog', () => {
  it('accepts a well-formed catalog and rejects broken ones', () => {
    expect(assertCatalog(makeCatalog()).combos).toHaveLength(3);
    expect(() => assertCatalog(null)).toThrow(/not an object/);
    expect(() => assertCatalog({ ...makeCatalog(), schema_version: 2 })).toThrow(/schema_version/);
    expect(() => assertCatalog({ schema_version: 1 })).toThrow(/combos/);
    const noMembers = makeCatalog();
    noMembers.combos[0].families[0].members = [];
    expect(() => assertCatalog(noMembers)).toThrow(/no members/);
  });
});

describe('loadCatalog', () => {
  it('fetches <base>/catalog.json and validates it', async () => {
    const fetchMock = vi.fn(async () => ({ ok: true, status: 200, json: async () => makeCatalog() }));
    vi.stubGlobal('fetch', fetchMock);
    const cat = await loadCatalog('catalog');
    expect(fetchMock).toHaveBeenCalledWith('catalog/catalog.json');
    expect(cat.combos[0].id).toBe('full');
  });

  it('throws on a non-ok response', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => ({ ok: false, status: 404, json: async () => ({}) })));
    await expect(loadCatalog('catalog')).rejects.toThrow(/404/);
  });
});

describe('loadF32 cache', () => {
  it('fetches each url exactly once', async () => {
    const fetchMock = vi.fn(async () => ({
      ok: true, status: 200, arrayBuffer: async () => f32Buffer([1, 2, 3]),
    }));
    vi.stubGlobal('fetch', fetchMock);
    const a = await loadF32('catalog/full/n25/0.f32');
    const b = await loadF32('catalog/full/n25/0.f32');
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(Array.from(a)).toEqual([1, 2, 3]);
    expect(b).toBe(a);
  });
});

describe('memberTrajectory / familyPreview', () => {
  it('resolves member and preview paths against the catalog base', async () => {
    const seen: string[] = [];
    vi.stubGlobal('fetch', vi.fn(async (url: string) => {
      seen.push(url);
      return { ok: true, status: 200, arrayBuffer: async () => f32Buffer([0, 0, 0, 1, 1, 1]) };
    }));
    await memberTrajectory('catalog', makeMember(3, { traj: 'full/n25/3.f32' }));
    const loops = await familyPreview('catalog', {
      resonance_n: 25, members: [], preview: 'full/n25/preview.f32', preview_counts: [1, 1],
    });
    expect(seen).toEqual(['catalog/full/n25/3.f32', 'catalog/full/n25/preview.f32']);
    expect(loops.map((l) => l.length)).toEqual([3, 3]);
  });
});
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cd web && npm test`
Expected: FAIL — `Failed to resolve import "./types"` / `"./data"` (modules do not exist yet).

- [ ] **Step 5: Implement `web/src/types.ts`**

```ts
export const SCHEMA_VERSION = 1;

export interface Terms { j2: boolean; c22: boolean; j3: boolean; earth: boolean }

export interface Elements {
  a_km: number;
  e: number;
  i_deg: number;
  omega_deg: number;
  raan_deg: number;
}

export interface Member {
  index: number;
  state0: number[];        // nondimensional rotating-frame [x,y,z,vx,vy,vz]
  period_s: number;
  period_nd: number;
  elements: Elements;
  nu1: number;
  nu2: number;
  r_peri_km: number;
  r_apo_km: number;
  residual: number;
  traj: string;            // path relative to the catalog root
}

export interface Family {
  resonance_n: number;
  members: Member[];
  preview: string;         // path relative to the catalog root
  preview_counts: number[]; // points per member inside preview.f32
}

export interface Combo {
  id: string;
  name: string;
  terms: Terms;
  families: Family[];
}

export interface Catalog {
  schema_version: number;
  generated: { date: string; git_hash: string };
  constants: Record<string, number | string>;
  combos: Combo[];
}

/** Runtime shape check at the trust boundary: JSON off the network is `unknown`. */
export function assertCatalog(value: unknown): Catalog {
  if (typeof value !== 'object' || value === null) throw new Error('catalog: not an object');
  const c = value as Catalog;
  if (c.schema_version !== SCHEMA_VERSION) {
    throw new Error(`catalog: schema_version ${String(c.schema_version)} != ${SCHEMA_VERSION}`);
  }
  if (!Array.isArray(c.combos)) throw new Error('catalog: combos must be an array');
  for (const combo of c.combos) {
    if (typeof combo.id !== 'string') throw new Error('catalog: combo missing id');
    if (typeof combo.terms?.j2 !== 'boolean') throw new Error(`catalog: combo ${combo.id} missing terms`);
    if (!Array.isArray(combo.families)) throw new Error(`catalog: combo ${combo.id} missing families`);
    for (const fam of combo.families) {
      if (typeof fam.resonance_n !== 'number') {
        throw new Error(`catalog: combo ${combo.id} family missing resonance_n`);
      }
      if (!Array.isArray(fam.members) || fam.members.length === 0) {
        throw new Error(`catalog: combo ${combo.id} N=${fam.resonance_n} has no members`);
      }
    }
  }
  return c;
}
```

- [ ] **Step 6: Implement `web/src/data.ts`**

```ts
import { assertCatalog } from './types';
import type { Catalog, Family, Member } from './types';

const cache = new Map<string, Promise<Float32Array>>();

export function clearCache(): void {
  cache.clear();
}

export function joinUrl(base: string, path: string): string {
  const b = base.replace(/\/+$/, '');
  const p = path.replace(/^\/+/, '');
  return b === '' ? p : `${b}/${p}`;
}

/** Raw little-endian float32 xyz triples in km, no header (catalog binary contract). */
export function parseF32(buf: ArrayBuffer): Float32Array {
  const n = Math.floor(buf.byteLength / 4);
  const view = new DataView(buf);
  const out = new Float32Array(n);
  for (let i = 0; i < n; i++) out[i] = view.getFloat32(i * 4, true);
  return out;
}

/** preview.f32 is every member's decimated loop concatenated; counts are point counts. */
export function splitPreview(data: Float32Array, counts: number[]): Float32Array[] {
  const out: Float32Array[] = [];
  let off = 0;
  for (const c of counts) {
    out.push(data.subarray(off, off + c * 3));
    off += c * 3;
  }
  return out;
}

export async function loadCatalog(baseUrl: string): Promise<Catalog> {
  const url = joinUrl(baseUrl, 'catalog.json');
  const res = await fetch(url);
  if (!res.ok) throw new Error(`${url}: HTTP ${res.status}`);
  return assertCatalog(await res.json());
}

export function loadF32(url: string): Promise<Float32Array> {
  const hit = cache.get(url);
  if (hit) return hit;
  const pending = fetch(url).then(async (res) => {
    if (!res.ok) throw new Error(`${url}: HTTP ${res.status}`);
    return parseF32(await res.arrayBuffer());
  });
  cache.set(url, pending);
  return pending;
}

export function memberTrajectory(baseUrl: string, member: Member): Promise<Float32Array> {
  return loadF32(joinUrl(baseUrl, member.traj));
}

export async function familyPreview(baseUrl: string, family: Family): Promise<Float32Array[]> {
  const data = await loadF32(joinUrl(baseUrl, family.preview));
  return splitPreview(data, family.preview_counts);
}

/** Fire-and-forget warming of the neighbours the member slider is about to reach. */
export function prefetchNeighbors(baseUrl: string, family: Family, index: number): void {
  for (const i of [index - 1, index + 1]) {
    const m = family.members[i];
    if (m) void memberTrajectory(baseUrl, m).catch(() => undefined);
  }
}
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cd web && npm test`
Expected: PASS — all suites in `src/data.test.ts` green.

- [ ] **Step 8: Commit**

```bash
git add -A && git commit -m "feat: web scaffold, catalog types, and data loader with f32 parsing"
```

---

### Task 17: Store and pure UI helpers

**Files:**
- Create: `web/src/state.ts`, `web/src/state.test.ts`

**Interfaces:**
- Consumes: types only, from Task 16's `types.ts` (`Catalog`, `Combo`, `Family`).
- Produces (used by Tasks 18–22):
  - `GhostPin { comboId: string; familyN: number; memberIndex: number }`
  - `AppState { comboId: string; familyN: number; memberIndex: number; animTime: number; playing: boolean; speed: number; ghost: GhostPin | null }` — `animTime` in seconds into the current member's closure period, `speed` in simulated seconds per wall second.
  - `Listener = (state: AppState, prev: AppState) => void`
  - `Store { get(): AppState; update(partial: Partial<AppState>): void; subscribe(fn: Listener): () => void }`
  - `createStore(initial: AppState): Store`
  - `symlog(v: number): number`
  - `nearestMemberIndex(fromIndex: number, fromLength: number, toLength: number): number`
  - `samplePosition(traj: Float32Array, period: number, t: number): [number, number, number]`
  - `elapsedRevs(t: number, period: number, resonanceN: number): number`
  - `comboById(catalog: Catalog, id: string): Combo | undefined`
  - `familyByN(combo: Combo, n: number): Family | undefined`
- Subscribers receive the previous state so they can skip work on irrelevant changes — the animation writes `animTime` every frame, so every subscriber MUST guard on the fields it cares about.

- [ ] **Step 1: Write the failing tests**

`web/src/state.test.ts`:
```ts
import { describe, expect, it } from 'vitest';
import {
  comboById, createStore, elapsedRevs, familyByN,
  nearestMemberIndex, samplePosition, symlog,
} from './state';
import type { AppState } from './state';
import { makeCatalog } from './testFixtures';

const INITIAL: AppState = {
  comboId: 'full', familyN: 25, memberIndex: 0,
  animTime: 0, playing: false, speed: 21600, ghost: null,
};

describe('symlog', () => {
  it('is identity inside the unit band and log10 outside', () => {
    expect(symlog(0)).toBe(0);
    expect(symlog(0.5)).toBeCloseTo(0.5, 12);
    expect(symlog(1)).toBeCloseTo(1, 12);
    expect(symlog(-1)).toBeCloseTo(-1, 12);
    expect(symlog(10)).toBeCloseTo(2, 12);
    expect(symlog(-1000)).toBeCloseTo(-4, 12);
  });
});

describe('nearestMemberIndex', () => {
  it('maps fractional position along the family and clamps', () => {
    expect(nearestMemberIndex(0, 10, 5)).toBe(0);
    expect(nearestMemberIndex(9, 10, 5)).toBe(4);
    expect(nearestMemberIndex(5, 11, 21)).toBe(10);
    expect(nearestMemberIndex(3, 7, 1)).toBe(0);
    expect(nearestMemberIndex(3, 1, 9)).toBe(0);
  });
});

describe('samplePosition', () => {
  // Four samples around a unit square, uniform over a period of 4; sample 3 wraps to sample 0.
  const square = new Float32Array([1, 0, 0, 0, 1, 0, -1, 0, 0, 0, -1, 0]);

  it('hits stored samples exactly', () => {
    expect(samplePosition(square, 4, 0)).toEqual([1, 0, 0]);
    expect(samplePosition(square, 4, 2)).toEqual([-1, 0, 0]);
  });

  it('interpolates linearly between samples', () => {
    const p = samplePosition(square, 4, 0.5);
    expect(p[0]).toBeCloseTo(0.5, 12);
    expect(p[1]).toBeCloseTo(0.5, 12);
  });

  it('wraps the last segment back to the first sample', () => {
    const p = samplePosition(square, 4, 3.5);
    expect(p[0]).toBeCloseTo(0.5, 12);
    expect(p[1]).toBeCloseTo(-0.5, 12);
  });

  it('wraps t modulo the period in both directions', () => {
    expect(samplePosition(square, 4, 4)).toEqual([1, 0, 0]);
    const back = samplePosition(square, 4, -0.5);
    expect(back[0]).toBeCloseTo(0.5, 12);
    expect(back[1]).toBeCloseTo(-0.5, 12);
    const fwd = samplePosition(square, 4, 8.25);
    expect(fwd[0]).toBeCloseTo(0.75, 12);
    expect(fwd[1]).toBeCloseTo(0.25, 12);
  });
});

describe('elapsedRevs', () => {
  it('counts revs as the resonance number scaled by period fraction', () => {
    expect(elapsedRevs(1180295.5, 2360591, 25)).toBeCloseTo(12.5, 9);
    expect(elapsedRevs(0, 2360591, 25)).toBe(0);
  });
});

describe('catalog lookups', () => {
  it('finds a combo by id and a family by resonance, or reports absence', () => {
    const cat = makeCatalog();
    expect(comboById(cat, 'no-c22')?.name).toBe('J2 + J3 + Earth (C22 off)');
    expect(comboById(cat, 'nope')).toBeUndefined();
    expect(familyByN(comboById(cat, 'full')!, 30)?.members).toHaveLength(4);
    expect(familyByN(comboById(cat, 'full')!, 59)).toBeUndefined();
  });
});

describe('createStore', () => {
  it('merges partial updates, notifies with previous state, and unsubscribes', () => {
    const store = createStore(INITIAL);
    const seen: Array<[number, number]> = [];
    const off = store.subscribe((s, p) => seen.push([s.memberIndex, p.memberIndex]));

    store.update({ memberIndex: 3 });
    expect(store.get().memberIndex).toBe(3);
    expect(store.get().familyN).toBe(25);
    expect(seen).toEqual([[3, 0]]);

    off();
    store.update({ memberIndex: 4 });
    expect(store.get().memberIndex).toBe(4);
    expect(seen).toHaveLength(1);
  });

  it('does not mutate the object it was constructed with', () => {
    const store = createStore(INITIAL);
    store.update({ playing: true });
    expect(INITIAL.playing).toBe(false);
    expect(store.get().playing).toBe(true);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd web && npm test -- state`
Expected: FAIL — `Failed to resolve import "./state"`.

- [ ] **Step 3: Implement `web/src/state.ts`**

```ts
import type { Catalog, Combo, Family } from './types';

export interface GhostPin {
  comboId: string;
  familyN: number;
  memberIndex: number;
}

export interface AppState {
  comboId: string;
  familyN: number;
  memberIndex: number;
  animTime: number;   // seconds into the current member's closure period
  playing: boolean;
  speed: number;      // simulated seconds per wall-clock second
  ghost: GhostPin | null;
}

export type Listener = (state: AppState, prev: AppState) => void;

export interface Store {
  get(): AppState;
  update(partial: Partial<AppState>): void;
  subscribe(fn: Listener): () => void;
}

export function createStore(initial: AppState): Store {
  let state: AppState = { ...initial };
  const listeners = new Set<Listener>();
  return {
    get: () => state,
    update(partial) {
      const prev = state;
      state = { ...state, ...partial };
      for (const fn of [...listeners]) fn(state, prev);
    },
    subscribe(fn) {
      listeners.add(fn);
      return () => {
        listeners.delete(fn);
      };
    },
  };
}

/** Symmetric log: linear inside |v| <= 1, log10 outside. Keeps the ±1 stability band readable. */
export function symlog(v: number): number {
  const a = Math.abs(v);
  return Math.sign(v) * (a <= 1 ? a : 1 + Math.log10(a));
}

/** Same fractional position along a family of a different length. */
export function nearestMemberIndex(fromIndex: number, fromLength: number, toLength: number): number {
  if (toLength <= 1 || fromLength <= 1) return 0;
  const frac = fromIndex / (fromLength - 1);
  return Math.min(toLength - 1, Math.max(0, Math.round(frac * (toLength - 1))));
}

/**
 * Position on a uniformly sampled closed trajectory. `traj` is xyz triples over exactly
 * one period with no repeated endpoint, so the last sample interpolates back to the first.
 */
export function samplePosition(traj: Float32Array, period: number, t: number): [number, number, number] {
  const n = Math.floor(traj.length / 3);
  if (n === 0 || !(period > 0)) return [0, 0, 0];
  if (n === 1) return [traj[0], traj[1], traj[2]];
  const tt = ((t % period) + period) % period;
  const u = (tt / period) * n;
  const base = Math.floor(u);
  const f = u - base;
  const a = (base % n) * 3;
  const b = ((base + 1) % n) * 3;
  return [
    traj[a] + (traj[b] - traj[a]) * f,
    traj[a + 1] + (traj[b + 1] - traj[a + 1]) * f,
    traj[a + 2] + (traj[b + 2] - traj[a + 2]) * f,
  ];
}

/** Revolutions completed: the closure period contains exactly `resonanceN` revs. */
export function elapsedRevs(t: number, period: number, resonanceN: number): number {
  return period > 0 ? (t / period) * resonanceN : 0;
}

export function comboById(catalog: Catalog, id: string): Combo | undefined {
  return catalog.combos.find((c) => c.id === id);
}

export function familyByN(combo: Combo, n: number): Family | undefined {
  return combo.families.find((f) => f.resonance_n === n);
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd web && npm test`
Expected: PASS — `src/state.test.ts` and `src/data.test.ts` both green.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: pub/sub store, symlog, nearest-member, and trajectory sampling"
```

---

### Task 18: 3D stage — Moon, family stack, selected member, ghost, camera presets

**Files:**
- Create: `web/src/scene.ts`, `web/src/scene.test.ts`, `web/src/main.ts` (replace the Vite template's file entirely), `web/src/style.css` (replace the template's file entirely), `web/public/moon.jpg` (downloaded)

**Interfaces:**
- Consumes: `data.ts` (`loadCatalog`, `familyPreview`, `memberTrajectory`), `types.ts`.
- Produces (used by Tasks 19–22):
  - `KM_TO_SCENE = 1 / 1000` (scene units are megametres), `MOON_RADIUS_KM = 1737.4`, `MOON_TEXTURE_LON_OFFSET`
  - `PresetName = 'south-pole' | 'earth-line'`
  - `CameraPose { position: [number, number, number]; up: [number, number, number] }`
  - `presetCamera(name: PresetName, dist: number): CameraPose` (pure)
  - `scenePositions(xyzKm: Float32Array): Float32Array` (pure — km → scene units)
  - `Stage { scene, camera, renderer, controls, setFamilyStack(loops: Float32Array[]): void, setSelected(traj: Float32Array | null): void, setGhost(traj: Float32Array | null): void, setGraticule(visible: boolean): void, setFrameRadiusKm(km: number): void, applyPreset(name: PresetName): void, render(): void, resize(): void }`
  - `createStage(container: HTMLElement): Stage`
- Frame convention: data coordinates are used directly. Earth is toward −x, +z is the orbit normal, and `camera.up` is `(0,0,1)` for every preset except the south-pole view (which uses `(1,0,0)` because looking down −z makes `(0,0,1)` degenerate).

- [ ] **Step 1: Download the Moon texture**

```bash
curl -Lo web/public/moon.jpg https://svs.gsfc.nasa.gov/vis/a000000/a004700/a004720/lroc_color_poles_1k.jpg
```
Expected: a JPEG of roughly 100–600 KB. Check with `file web/public/moon.jpg` → `JPEG image data`. This is NASA/SVS LROC albedo imagery (public domain); commit it so a fresh clone renders correctly. If the download fails, continue anyway — `scene.ts` falls back to flat gray.

- [ ] **Step 2: Write the failing tests**

`web/src/scene.test.ts`:
```ts
import { describe, expect, it } from 'vitest';
import { KM_TO_SCENE, MOON_RADIUS_KM, presetCamera, scenePositions } from './scene';

describe('scenePositions', () => {
  it('converts km to scene megametres without changing the axis order', () => {
    const out = scenePositions(new Float32Array([1000, -2000, 3000, 0, 0, 1737.4]));
    expect(Array.from(out.slice(0, 3))).toEqual([1, -2, 3]);
    expect(out[5]).toBeCloseTo(MOON_RADIUS_KM * KM_TO_SCENE, 6);
    expect(out.length).toBe(6);
  });
});

describe('presetCamera', () => {
  it('south-pole view looks along +z from below with +x as screen-up', () => {
    const pose = presetCamera('south-pole', 12);
    expect(pose.position).toEqual([0, 0, -12]);
    expect(pose.up).toEqual([1, 0, 0]);
  });

  it('earth-line view keeps the orbit normal as screen-up and stays off the x axis', () => {
    const pose = presetCamera('earth-line', 12);
    expect(pose.up).toEqual([0, 0, 1]);
    expect(pose.position[0]).toBe(0);
    expect(pose.position[1]).toBeLessThan(0);
    // the up vector must not be parallel to the view direction
    expect(Math.abs(pose.position[1])).toBeGreaterThan(Math.abs(pose.position[2]));
  });
});
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd web && npm test -- scene`
Expected: FAIL — `Failed to resolve import "./scene"`.

- [ ] **Step 4: Implement `web/src/scene.ts`**

```ts
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';

/** Scene units are megametres: keeps three.js far from float32 precision cliffs. */
export const KM_TO_SCENE = 1 / 1000;
export const MOON_RADIUS_KM = 1737.4;
/** Spin applied to the textured sphere so longitude 0 (sub-Earth point) faces −x. */
export const MOON_TEXTURE_LON_OFFSET = Math.PI;

export type PresetName = 'south-pole' | 'earth-line';

export interface CameraPose {
  position: [number, number, number];
  up: [number, number, number];
}

export function presetCamera(name: PresetName, dist: number): CameraPose {
  if (name === 'south-pole') {
    // Straight up the −z axis at the south pole. camera.up = +z would be degenerate here,
    // so +x (anti-Earth) becomes screen-up and Earth points down the screen.
    return { position: [0, 0, -dist], up: [1, 0, 0] };
  }
  // Earth-Moon line across the screen, orbit normal up, slightly above the xy plane.
  return { position: [0, -dist, 0.25 * dist], up: [0, 0, 1] };
}

export function scenePositions(xyzKm: Float32Array): Float32Array {
  const out = new Float32Array(xyzKm.length);
  for (let i = 0; i < xyzKm.length; i++) out[i] = xyzKm[i] * KM_TO_SCENE;
  return out;
}

export interface Stage {
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  renderer: THREE.WebGLRenderer;
  controls: OrbitControls;
  setFamilyStack(loops: Float32Array[]): void;
  setSelected(traj: Float32Array | null): void;
  setGhost(traj: Float32Array | null): void;
  setGraticule(visible: boolean): void;
  setFrameRadiusKm(km: number): void;
  applyPreset(name: PresetName): void;
  render(): void;
  resize(): void;
}

const STACK_COLOR = 0x63b8ff;
const SELECTED_COLOR = 0xffc24a;
const GHOST_COLOR = 0x777777;

function makeLoop(xyzKm: Float32Array, material: THREE.LineBasicMaterial): THREE.LineLoop {
  const geo = new THREE.BufferGeometry();
  geo.setAttribute('position', new THREE.BufferAttribute(scenePositions(xyzKm), 3));
  return new THREE.LineLoop(geo, material);
}

function clearGroup(group: THREE.Group): void {
  for (const child of [...group.children]) {
    group.remove(child);
    (child as THREE.LineLoop).geometry.dispose();
  }
}

function makeGraticule(radiusKm: number): THREE.LineSegments {
  const r = radiusKm * 1.003 * KM_TO_SCENE;
  const seg = 72;
  const pts: number[] = [];
  const push = (lat: number, lon: number) => {
    pts.push(r * Math.cos(lat) * Math.cos(lon), r * Math.cos(lat) * Math.sin(lon), r * Math.sin(lat));
  };
  for (let latDeg = -60; latDeg <= 60; latDeg += 30) {
    const lat = (latDeg * Math.PI) / 180;
    for (let k = 0; k < seg; k++) {
      push(lat, (2 * Math.PI * k) / seg);
      push(lat, (2 * Math.PI * (k + 1)) / seg);
    }
  }
  for (let lonDeg = 0; lonDeg < 360; lonDeg += 30) {
    const lon = (lonDeg * Math.PI) / 180;
    for (let k = 0; k < seg; k++) {
      push(-Math.PI / 2 + (Math.PI * k) / seg, lon);
      push(-Math.PI / 2 + (Math.PI * (k + 1)) / seg, lon);
    }
  }
  const geo = new THREE.BufferGeometry();
  geo.setAttribute('position', new THREE.Float32BufferAttribute(pts, 3));
  return new THREE.LineSegments(
    geo,
    new THREE.LineBasicMaterial({ color: 0x55dd99, transparent: true, opacity: 0.22 }),
  );
}

export function createStage(container: HTMLElement): Stage {
  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0x05070d);

  const camera = new THREE.PerspectiveCamera(45, 1, 0.02, 5000);
  camera.up.set(0, 0, 1);

  const renderer = new THREE.WebGLRenderer({ antialias: true });
  renderer.setPixelRatio(window.devicePixelRatio);
  container.appendChild(renderer.domElement);

  const controls = new OrbitControls(camera, renderer.domElement);
  controls.enableDamping = true;
  controls.dampingFactor = 0.08;

  scene.add(new THREE.AmbientLight(0xffffff, 0.4));
  const key = new THREE.DirectionalLight(0xffffff, 2.0);
  key.position.set(-6, 3, 2); // lit from the Earth side so the near face reads
  scene.add(key);

  // --- Moon -------------------------------------------------------------
  const moonMat = new THREE.MeshStandardMaterial({ color: 0x9a9a9a, roughness: 1, metalness: 0 });
  const moon = new THREE.Mesh(
    new THREE.SphereGeometry(MOON_RADIUS_KM * KM_TO_SCENE, 96, 64),
    moonMat,
  );
  moon.rotation.x = Math.PI / 2; // three's poles (±y) onto our ±z
  const moonPivot = new THREE.Group();
  moonPivot.rotation.z = MOON_TEXTURE_LON_OFFSET;
  moonPivot.add(moon);
  scene.add(moonPivot);

  new THREE.TextureLoader().load(
    'moon.jpg',
    (tex) => {
      tex.colorSpace = THREE.SRGBColorSpace;
      moonMat.map = tex;
      moonMat.color.set(0xffffff);
      moonMat.needsUpdate = true;
    },
    undefined,
    () => {
      console.warn('scene: moon.jpg failed to load — falling back to flat gray');
    },
  );

  const graticule = makeGraticule(MOON_RADIUS_KM);
  scene.add(graticule);

  // --- South pole marker ------------------------------------------------
  const poleR = MOON_RADIUS_KM * KM_TO_SCENE;
  const poleDot = new THREE.Mesh(
    new THREE.SphereGeometry(0.07, 16, 12),
    new THREE.MeshBasicMaterial({ color: 0xff5566 }),
  );
  poleDot.position.set(0, 0, -poleR);
  scene.add(poleDot);
  const poleGeo = new THREE.BufferGeometry().setFromPoints([
    new THREE.Vector3(0, 0, -poleR),
    new THREE.Vector3(0, 0, -poleR * 1.7),
  ]);
  scene.add(new THREE.Line(poleGeo, new THREE.LineBasicMaterial({ color: 0xff5566 })));

  // --- Earth direction (−x) --------------------------------------------
  scene.add(new THREE.ArrowHelper(
    new THREE.Vector3(-1, 0, 0), new THREE.Vector3(0, 0, 0), 6 * poleR, 0x5aa9ff, 0.7, 0.35,
  ));
  const emGeo = new THREE.BufferGeometry().setFromPoints([
    new THREE.Vector3(0, 0, 0),
    new THREE.Vector3(-80, 0, 0),
  ]);
  scene.add(new THREE.Line(
    emGeo, new THREE.LineBasicMaterial({ color: 0x2a4a7a, transparent: true, opacity: 0.5 }),
  ));

  // --- Orbit layers -----------------------------------------------------
  const stackMat = new THREE.LineBasicMaterial({ color: STACK_COLOR, transparent: true, opacity: 0.15 });
  const selectedMat = new THREE.LineBasicMaterial({ color: SELECTED_COLOR });
  const ghostMat = new THREE.LineBasicMaterial({ color: GHOST_COLOR, transparent: true, opacity: 0.85 });
  const stack = new THREE.Group();
  const selected = new THREE.Group();
  const ghost = new THREE.Group();
  scene.add(stack, selected, ghost);

  let frameDist = 30;
  let preset: PresetName = 'south-pole';

  const stage: Stage = {
    scene,
    camera,
    renderer,
    controls,
    setFamilyStack(loops) {
      clearGroup(stack);
      for (const l of loops) stack.add(makeLoop(l, stackMat));
    },
    setSelected(traj) {
      clearGroup(selected);
      if (traj) selected.add(makeLoop(traj, selectedMat));
    },
    setGhost(traj) {
      clearGroup(ghost);
      if (traj) ghost.add(makeLoop(traj, ghostMat));
    },
    setGraticule(visible) {
      graticule.visible = visible;
    },
    setFrameRadiusKm(km) {
      frameDist = Math.max(4, km * KM_TO_SCENE * 2.6);
    },
    applyPreset(name) {
      preset = name;
      const pose = presetCamera(name, frameDist);
      camera.up.set(pose.up[0], pose.up[1], pose.up[2]);
      camera.position.set(pose.position[0], pose.position[1], pose.position[2]);
      controls.target.set(0, 0, 0);
      controls.update();
    },
    render() {
      controls.update();
      renderer.render(scene, camera);
    },
    resize() {
      const w = container.clientWidth || 1;
      const h = container.clientHeight || 1;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
    },
  };

  stage.resize();
  stage.applyPreset(preset);
  return stage;
}
```

- [ ] **Step 5: Implement the minimal cockpit CSS**

Replace `web/src/style.css` with (Task 20 fills in the rail and plot styling; this is the grid skeleton the stage needs to have a size):
```css
:root { color-scheme: dark; }

html, body {
  margin: 0;
  height: 100%;
  background: #05070d;
  color: #d7dee8;
  font: 13px/1.45 ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
}

#app {
  display: grid;
  grid-template-columns: 320px 1fr;
  grid-template-rows: 1fr 210px;
  grid-template-areas:
    "rail stage"
    "plot plot";
  height: 100vh;
}

#rail  { grid-area: rail;  overflow-y: auto; border-right: 1px solid #1b2331; }
#stage { grid-area: stage; position: relative; min-width: 0; min-height: 0; }
#stage canvas { display: block; }
#plot  { grid-area: plot;  border-top: 1px solid #1b2331; }
```

- [ ] **Step 6: Implement a minimal `web/src/main.ts` boot**

Replace `web/src/main.ts` entirely (Tasks 19 and 21 rewrite this file again as the cockpit grows):
```ts
import './style.css';
import { familyPreview, loadCatalog, memberTrajectory } from './data';
import { createStage } from './scene';

const CATALOG_BASE = 'catalog';

async function boot(): Promise<void> {
  const stage = createStage(document.getElementById('stage') as HTMLElement);
  window.addEventListener('resize', () => stage.resize());

  const catalog = await loadCatalog(CATALOG_BASE);
  const combo = catalog.combos[0];
  const family = combo.families[0];
  const member = family.members[Math.floor(family.members.length / 2)];

  stage.setFamilyStack(await familyPreview(CATALOG_BASE, family));
  stage.setSelected(await memberTrajectory(CATALOG_BASE, member));
  stage.setGraticule(true);
  stage.setFrameRadiusKm(member.r_apo_km);
  stage.applyPreset('south-pole');

  const tick = (): void => {
    stage.render();
    requestAnimationFrame(tick);
  };
  tick();
}

boot().catch((err: unknown) => {
  document.body.insertAdjacentHTML(
    'afterbegin',
    `<pre style="color:#ff6b6b;padding:12px">${String(err)}</pre>`,
  );
});
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cd web && npm test`
Expected: PASS — `scene.test.ts`, `state.test.ts`, `data.test.ts` all green.
Also run `cd web && npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 8: Manual verification of the 3D stage**

Run: `cd web && npm run dev`, open the printed URL, and confirm each item by eye:
1. A textured Moon sphere fills the centre pane; if `moon.jpg` is missing it is flat gray and the console logs "falling back to flat gray" — no crash either way.
2. The default view looks straight at the lunar south pole, with the red pole marker and its spike pointing at the camera.
3. A green lat/lon graticule hugs the sphere at 30° spacing.
4. A blue arrow leaves the Moon along −x with a faint blue Earth-Moon line continuing past it.
5. About seven translucent blue orbit loops (the N=25 family stack) surround the Moon, with one solid amber loop drawn brighter than the rest.
6. Mouse drag orbits the camera, wheel zooms, and the orbits stay visually attached to the Moon (no jitter, no clipping through the near plane).
7. In the browser console run `document.querySelector('canvas')` — a canvas exists and resizing the window keeps it filling the pane without stretching.

The dev catalog has only one combo, so no toggles or rail exist yet — that is expected at this task.

- [ ] **Step 9: Commit**

```bash
git add -A && git commit -m "feat: three.js stage with Moon, family stack, ghost layer, and camera presets"
```

---

### Task 19: Animation — rAF loop, satellite marker, fading trail, speed dial, readout

**Files:**
- Create: `web/src/anim.ts`, `web/src/anim.test.ts`
- Modify: `web/src/main.ts` (replace the file with the version below), `web/src/style.css` (append the animation-control rules)

**Interfaces:**
- Consumes: `state.ts` (`samplePosition`, `Store`), `scene.ts` (`KM_TO_SCENE`), `data.ts`, `types.ts`.
- Produces (used by Tasks 20–22):
  - `SPEED_MIN = 60`, `SPEED_MAX = 864000`, `SPEED_DEFAULT = 21600` (simulated seconds per wall second), `TRAIL_POINTS = 200`, `TRAIL_SPAN_FRAC = 0.08`
  - `speedFromDial(u: number): number` / `dialFromSpeed(v: number): number` — log dial, `u ∈ [0,1]`
  - `advanceTime(t: number, dtWallS: number, speed: number, periodS: number): number`
  - `trailTimes(tNow: number, periodS: number, count?: number, spanFrac?: number): number[]`
  - `readout(tS: number, periodS: number, resonanceN: number): { days: number; revs: number }`
  - `Satellite { group: THREE.Group; setMember(traj: Float32Array | null, periodS: number, resonanceN: number): void; update(animTimeS: number): { positionKm: [number, number, number]; days: number; revs: number } | null }`
  - `createSatellite(): Satellite`
  - `createLoop(onTick: (dtWallS: number) => void): { start(): void; stop(): void }`
  - `AnimControls { setReadout(days: number, revs: number, resonanceN: number): void }`
  - `mountAnimControls(container: HTMLElement, store: Store): AnimControls`
- `mountAnimControls` renders a Play/Pause button, a log speed range input, and a readout line into the container it is given. Task 20's left rail owns that container.

- [ ] **Step 1: Write the failing tests**

`web/src/anim.test.ts`:
```ts
import { describe, expect, it } from 'vitest';
import {
  advanceTime, dialFromSpeed, readout, speedFromDial, trailTimes,
  SPEED_DEFAULT, SPEED_MAX, SPEED_MIN,
} from './anim';

describe('speed dial', () => {
  it('is logarithmic across the full range', () => {
    expect(speedFromDial(0)).toBeCloseTo(SPEED_MIN, 6);
    expect(speedFromDial(1)).toBeCloseTo(SPEED_MAX, 6);
    expect(speedFromDial(0.5)).toBeCloseTo(7200, 6); // sqrt(60 * 864000)
  });

  it('clamps out-of-range dial positions', () => {
    expect(speedFromDial(-1)).toBeCloseTo(SPEED_MIN, 6);
    expect(speedFromDial(3)).toBeCloseTo(SPEED_MAX, 6);
  });

  it('round-trips the default speed', () => {
    expect(speedFromDial(dialFromSpeed(SPEED_DEFAULT))).toBeCloseTo(SPEED_DEFAULT, 6);
    expect(dialFromSpeed(SPEED_MIN)).toBeCloseTo(0, 12);
    expect(dialFromSpeed(SPEED_MAX)).toBeCloseTo(1, 12);
  });
});

describe('advanceTime', () => {
  it('advances by speed * wall time and wraps at the period', () => {
    expect(advanceTime(0, 1, 21600, 2360591)).toBeCloseTo(21600, 6);
    expect(advanceTime(2360591 - 100, 1, 21600, 2360591)).toBeCloseTo(21500, 6);
  });

  it('returns 0 for a degenerate period', () => {
    expect(advanceTime(5, 1, 21600, 0)).toBe(0);
  });
});

describe('trailTimes', () => {
  it('walks backwards from now over a fraction of the period', () => {
    expect(trailTimes(100, 1000, 5, 0.1)).toEqual([100, 75, 50, 25, 0]);
  });
});

describe('readout', () => {
  it('reports elapsed days and revs within the closure period', () => {
    const r = readout(172800, 2360591, 25);
    expect(r.days).toBeCloseTo(2, 9);
    expect(r.revs).toBeCloseTo(1.83, 2);
    expect(readout(2360591, 2360591, 25).revs).toBeCloseTo(25, 9);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd web && npm test -- anim`
Expected: FAIL — `Failed to resolve import "./anim"`.

- [ ] **Step 3: Implement `web/src/anim.ts`**

```ts
import * as THREE from 'three';
import { KM_TO_SCENE } from './scene';
import { samplePosition } from './state';
import type { Store } from './state';

/** Simulated seconds per wall-clock second. */
export const SPEED_MIN = 60;        // 1 minute per second
export const SPEED_MAX = 864_000;   // 10 days per second
export const SPEED_DEFAULT = 21_600; // 6 hours per second
export const TRAIL_POINTS = 200;
export const TRAIL_SPAN_FRAC = 0.08; // trail covers 8% of the closure period

export function speedFromDial(u: number): number {
  const c = Math.min(1, Math.max(0, u));
  return SPEED_MIN * Math.pow(SPEED_MAX / SPEED_MIN, c);
}

export function dialFromSpeed(v: number): number {
  const c = Math.min(SPEED_MAX, Math.max(SPEED_MIN, v));
  return Math.log(c / SPEED_MIN) / Math.log(SPEED_MAX / SPEED_MIN);
}

export function advanceTime(t: number, dtWallS: number, speed: number, periodS: number): number {
  if (!(periodS > 0)) return 0;
  const next = t + dtWallS * speed;
  return ((next % periodS) + periodS) % periodS;
}

export function trailTimes(
  tNow: number,
  periodS: number,
  count: number = TRAIL_POINTS,
  spanFrac: number = TRAIL_SPAN_FRAC,
): number[] {
  const span = periodS * spanFrac;
  const out: number[] = [];
  for (let k = 0; k < count; k++) out.push(tNow - (span * k) / (count - 1));
  return out;
}

export function readout(tS: number, periodS: number, resonanceN: number): { days: number; revs: number } {
  return {
    days: tS / 86_400,
    revs: periodS > 0 ? (tS / periodS) * resonanceN : 0,
  };
}

export interface Satellite {
  group: THREE.Group;
  setMember(traj: Float32Array | null, periodS: number, resonanceN: number): void;
  update(animTimeS: number): { positionKm: [number, number, number]; days: number; revs: number } | null;
}

export function createSatellite(): Satellite {
  const group = new THREE.Group();

  const marker = new THREE.Mesh(
    new THREE.SphereGeometry(0.09, 16, 12),
    new THREE.MeshBasicMaterial({ color: 0xffffff }),
  );
  group.add(marker);

  const positions = new Float32Array(TRAIL_POINTS * 3);
  const colors = new Float32Array(TRAIL_POINTS * 3);
  const trailGeo = new THREE.BufferGeometry();
  trailGeo.setAttribute('position', new THREE.BufferAttribute(positions, 3));
  trailGeo.setAttribute('color', new THREE.BufferAttribute(colors, 3));
  // Per-vertex alpha is not available on lines; ramp the colour toward the
  // background instead, which reads as a fading trail on the dark stage.
  const trail = new THREE.Line(
    trailGeo,
    new THREE.LineBasicMaterial({ vertexColors: true, transparent: true, opacity: 0.9 }),
  );
  group.add(trail);

  let traj: Float32Array | null = null;
  let periodS = 0;
  let resonanceN = 1;

  return {
    group,
    setMember(t, p, n) {
      traj = t;
      periodS = p;
      resonanceN = n;
      group.visible = t !== null;
    },
    update(animTimeS) {
      if (!traj || periodS <= 0) return null;
      const p = samplePosition(traj, periodS, animTimeS);
      marker.position.set(p[0] * KM_TO_SCENE, p[1] * KM_TO_SCENE, p[2] * KM_TO_SCENE);

      const times = trailTimes(animTimeS, periodS);
      for (let k = 0; k < times.length; k++) {
        const q = samplePosition(traj, periodS, times[k]);
        positions[k * 3] = q[0] * KM_TO_SCENE;
        positions[k * 3 + 1] = q[1] * KM_TO_SCENE;
        positions[k * 3 + 2] = q[2] * KM_TO_SCENE;
        const fade = 1 - k / (times.length - 1);
        colors[k * 3] = fade;
        colors[k * 3 + 1] = fade * 0.85;
        colors[k * 3 + 2] = fade * 0.55;
      }
      trailGeo.attributes.position.needsUpdate = true;
      trailGeo.attributes.color.needsUpdate = true;
      trailGeo.computeBoundingSphere();

      const r = readout(animTimeS, periodS, resonanceN);
      return { positionKm: p, days: r.days, revs: r.revs };
    },
  };
}

export function createLoop(onTick: (dtWallS: number) => void): { start(): void; stop(): void } {
  let raf = 0;
  let last = 0;
  const frame = (now: number): void => {
    const dt = last === 0 ? 0 : Math.min(0.1, (now - last) / 1000);
    last = now;
    onTick(dt);
    raf = requestAnimationFrame(frame);
  };
  return {
    start() {
      if (raf === 0) {
        last = 0;
        raf = requestAnimationFrame(frame);
      }
    },
    stop() {
      if (raf !== 0) {
        cancelAnimationFrame(raf);
        raf = 0;
      }
    },
  };
}

export interface AnimControls {
  setReadout(days: number, revs: number, resonanceN: number): void;
}

function formatSpeed(v: number): string {
  if (v >= 86_400) return `${(v / 86_400).toFixed(1)} d/s`;
  if (v >= 3_600) return `${(v / 3_600).toFixed(1)} h/s`;
  return `${(v / 60).toFixed(0)} min/s`;
}

export function mountAnimControls(container: HTMLElement, store: Store): AnimControls {
  container.innerHTML = `
    <div class="anim-controls">
      <button id="play-btn" type="button">Play</button>
      <label class="dial">
        <span>Speed</span>
        <input id="speed-dial" type="range" min="0" max="1" step="0.001" />
        <span id="speed-label" class="muted"></span>
      </label>
      <div id="anim-readout" class="readout-line">t + 0.00 d · rev 0.0</div>
    </div>`;

  const play = container.querySelector('#play-btn') as HTMLButtonElement;
  const dial = container.querySelector('#speed-dial') as HTMLInputElement;
  const speedLabel = container.querySelector('#speed-label') as HTMLSpanElement;
  const out = container.querySelector('#anim-readout') as HTMLDivElement;

  const sync = (): void => {
    const s = store.get();
    play.textContent = s.playing ? 'Pause' : 'Play';
    dial.value = String(dialFromSpeed(s.speed));
    speedLabel.textContent = formatSpeed(s.speed);
  };

  play.addEventListener('click', () => store.update({ playing: !store.get().playing }));
  dial.addEventListener('input', () => store.update({ speed: speedFromDial(Number(dial.value)) }));
  // animTime changes every frame — only re-sync on the fields these controls show.
  store.subscribe((s, p) => {
    if (s.playing !== p.playing || s.speed !== p.speed) sync();
  });
  sync();

  return {
    setReadout(days, revs, resonanceN) {
      out.textContent = `t + ${days.toFixed(2)} d · rev ${revs.toFixed(1)} / ${resonanceN}`;
    },
  };
}
```

- [ ] **Step 4: Append the animation-control styles to `web/src/style.css`**

```css
.anim-controls { display: flex; flex-direction: column; gap: 8px; }
.anim-controls button {
  align-self: flex-start;
  background: #16202e; color: #d7dee8;
  border: 1px solid #2b3a50; border-radius: 4px;
  padding: 4px 14px; cursor: pointer;
}
.anim-controls button:hover { background: #1e2c3f; }
.dial { display: flex; align-items: center; gap: 8px; }
.dial input[type="range"] { flex: 1; }
.readout-line { font-variant-numeric: tabular-nums; color: #9fb0c4; }
.muted { color: #7b8a9d; }
```

- [ ] **Step 5: Wire the animation into `web/src/main.ts`**

Replace `web/src/main.ts` entirely (the animation controls get a temporary host in the rail until Task 20 builds the real one):
```ts
import './style.css';
import { advanceTime, createLoop, createSatellite, mountAnimControls, SPEED_DEFAULT } from './anim';
import { familyPreview, loadCatalog, memberTrajectory } from './data';
import { createStage } from './scene';
import { createStore } from './state';

const CATALOG_BASE = 'catalog';

async function boot(): Promise<void> {
  const stage = createStage(document.getElementById('stage') as HTMLElement);
  window.addEventListener('resize', () => stage.resize());

  const catalog = await loadCatalog(CATALOG_BASE);
  const combo = catalog.combos[0];
  const family = combo.families[0];
  const memberIndex = Math.floor(family.members.length / 2);
  const member = family.members[memberIndex];

  const store = createStore({
    comboId: combo.id,
    familyN: family.resonance_n,
    memberIndex,
    animTime: 0,
    playing: true,
    speed: SPEED_DEFAULT,
    ghost: null,
  });

  const rail = document.getElementById('rail') as HTMLElement;
  rail.innerHTML = '<section class="card"><h2>Animation</h2><div id="anim-slot"></div></section>';
  const animControls = mountAnimControls(document.getElementById('anim-slot') as HTMLElement, store);

  const satellite = createSatellite();
  stage.scene.add(satellite.group);

  const traj = await memberTrajectory(CATALOG_BASE, member);
  stage.setFamilyStack(await familyPreview(CATALOG_BASE, family));
  stage.setSelected(traj);
  stage.setGraticule(true);
  stage.setFrameRadiusKm(member.r_apo_km);
  stage.applyPreset('south-pole');
  satellite.setMember(traj, member.period_s, family.resonance_n);

  createLoop((dtWall) => {
    const s = store.get();
    if (s.playing) {
      store.update({ animTime: advanceTime(s.animTime, dtWall, s.speed, member.period_s) });
    }
    const info = satellite.update(store.get().animTime);
    if (info) animControls.setReadout(info.days, info.revs, family.resonance_n);
    stage.render();
  }).start();
}

boot().catch((err: unknown) => {
  document.body.insertAdjacentHTML(
    'afterbegin',
    `<pre style="color:#ff6b6b;padding:12px">${String(err)}</pre>`,
  );
});
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd web && npm test`
Expected: PASS — `anim.test.ts` plus all earlier suites.
Also run `cd web && npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 7: Manual verification of the animation**

Run: `cd web && npm run dev` and confirm:
1. A white satellite marker travels along the amber selected orbit with a warm trail behind it that fades to black at the tail.
2. The Pause button stops the marker; Play resumes it from where it stopped.
3. Dragging the speed dial to the far left reads `1 min/s` and the marker crawls; far right reads `10.0 d/s` and it blurs around the loop.
4. The readout line counts up as `t + X.XX d · rev Y.Y / 25` and resets to `t + 0.00 d · rev 0.0` when the closure period completes.
5. The marker visibly slows near apoapsis (far from the Moon) and whips through periapsis — the ELFO dwell.

- [ ] **Step 8: Commit**

```bash
git add -A && git commit -m "feat: animation loop with satellite marker, fading trail, and speed dial"
```

---

### Task 20: Left rail, stability plot, cockpit grid

**Files:**
- Create: `web/src/ui/leftRail.ts`, `web/src/ui/leftRail.test.ts`, `web/src/ui/stabilityPlot.ts`, `web/src/ui/stabilityPlot.test.ts`
- Modify: `web/src/style.css` (replace the whole file), `web/src/main.ts` (replace the whole file)

**Interfaces:**
- Consumes: `state.ts` (`Store`, `symlog`, `comboById`, `familyByN`, `nearestMemberIndex`), `anim.ts` (`mountAnimControls`, `AnimControls`), `scene.ts` (`MOON_RADIUS_KM`, `PresetName`), `types.ts`, `d3-scale`'s `scaleLinear`.
- Produces (used by Tasks 21–22):
  - `ui/leftRail.ts`: `TERM_LABELS: Array<[keyof Terms, string]>`, `formatReadout(member: Member, family: Family): Array<{ label: string; value: string }>`, `memberEndpointLabel(member: Member): string`, `LeftRailHooks { availability(): Record<keyof Terms, boolean>; onToggle(term: keyof Terms): void; onPinGhost(): void; onClearGhost(): void; onPreset(name: PresetName): void; onGraticule(visible: boolean): void }`, `LeftRail { anim: AnimControls; setNotice(text: string): void; refresh(): void }`, `mountLeftRail(container: HTMLElement, store: Store, catalog: Catalog, hooks: LeftRailHooks): LeftRail`
  - `ui/stabilityPlot.ts`: `symlogDomain(nus: number[]): [number, number]`, `indexFromX(px: number, plotWidth: number, count: number): number`, `StabilityPlot { setFamily(family: Family): void; refresh(): void }`, `mountStabilityPlot(container: HTMLElement, store: Store): StabilityPlot`
- The rail owns the family buttons and the member slider (they write straight to the store). It does NOT decide which force-model toggles are legal — `hooks.availability()` supplies that, and Task 21 provides the real implementation.
- The plot's click/drag writes `memberIndex` to the store, making it an alternate member slider.

- [ ] **Step 1: Write the failing tests**

`web/src/ui/leftRail.test.ts`:
```ts
import { describe, expect, it } from 'vitest';
import { formatReadout, memberEndpointLabel, TERM_LABELS } from './leftRail';
import { makeFamily, makeMember } from '../testFixtures';

describe('formatReadout', () => {
  it('renders every metadata field the readout card shows', () => {
    const rows = formatReadout(makeMember(0), makeFamily(25, 1));
    const get = (label: string) => rows.find((r) => r.label === label)?.value;
    expect(get('a')).toBe('6000 km');
    expect(get('e')).toBe('0.6000');
    expect(get('i')).toBe('57.00°');
    expect(get('period')).toBe('27.322 d');
    expect(get('revs')).toBe('25');
    expect(get('peri alt')).toBe('663 km');   // 2400 − 1737.4 km
    expect(get('apo alt')).toBe('7863 km');   // 9600 − 1737.4 km
    expect(get('ν₁')).toBe('1.200');
    expect(get('ν₂')).toBe('-0.400');
    expect(get('residual')).toBe('1.0e-11');
    expect(rows).toHaveLength(12);
  });
});

describe('memberEndpointLabel', () => {
  it('annotates a slider endpoint with periapsis altitude and eccentricity', () => {
    expect(memberEndpointLabel(makeMember(0))).toBe('#0 · hp 663 km · e 0.600');
  });
});

describe('TERM_LABELS', () => {
  it('covers all four toggleable force terms in a stable order', () => {
    expect(TERM_LABELS.map(([k]) => k)).toEqual(['j2', 'c22', 'j3', 'earth']);
  });
});
```

`web/src/ui/stabilityPlot.test.ts`:
```ts
import { describe, expect, it } from 'vitest';
import { indexFromX, symlogDomain } from './stabilityPlot';

describe('symlogDomain', () => {
  it('always shows the ±1 stability boundary with headroom', () => {
    expect(symlogDomain([])).toEqual([-1.2, 1.2]);
    expect(symlogDomain([0.2, -0.5])).toEqual([-1.2, 1.2]);
  });

  it('grows symmetrically to fit unstable members', () => {
    const [lo, hi] = symlogDomain([100]);      // symlog(100) = 3
    expect(hi).toBeCloseTo(3.3, 9);
    expect(lo).toBeCloseTo(-3.3, 9);
    expect(symlogDomain([-1000, 0.5])[1]).toBeCloseTo(4.4, 9);
  });
});

describe('indexFromX', () => {
  it('maps plot x to a member index and clamps to the ends', () => {
    expect(indexFromX(0, 300, 11)).toBe(0);
    expect(indexFromX(150, 300, 11)).toBe(5);
    expect(indexFromX(300, 300, 11)).toBe(10);
    expect(indexFromX(-40, 300, 11)).toBe(0);
    expect(indexFromX(1000, 300, 11)).toBe(10);
    expect(indexFromX(37, 300, 1)).toBe(0);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd web && npm test -- ui`
Expected: FAIL — `Failed to resolve import "./leftRail"` / `"./stabilityPlot"`.

- [ ] **Step 3: Implement `web/src/ui/leftRail.ts`**

```ts
import { mountAnimControls } from '../anim';
import type { AnimControls } from '../anim';
import { MOON_RADIUS_KM } from '../scene';
import type { PresetName } from '../scene';
import { comboById, familyByN, nearestMemberIndex } from '../state';
import type { Store } from '../state';
import type { Catalog, Combo, Family, Member, Terms } from '../types';

export const TERM_LABELS: Array<[keyof Terms, string]> = [
  ['j2', 'J₂ — oblateness'],
  ['c22', 'C₂₂ — equatorial ellipticity'],
  ['j3', 'J₃ — pear shape'],
  ['earth', 'Earth third body'],
];

export function formatReadout(member: Member, family: Family): Array<{ label: string; value: string }> {
  const e = member.elements;
  return [
    { label: 'a', value: `${e.a_km.toFixed(0)} km` },
    { label: 'e', value: e.e.toFixed(4) },
    { label: 'i', value: `${e.i_deg.toFixed(2)}°` },
    { label: 'ω', value: `${e.omega_deg.toFixed(2)}°` },
    { label: 'Ω', value: `${e.raan_deg.toFixed(2)}°` },
    { label: 'period', value: `${(member.period_s / 86_400).toFixed(3)} d` },
    { label: 'revs', value: `${family.resonance_n}` },
    { label: 'peri alt', value: `${(member.r_peri_km - MOON_RADIUS_KM).toFixed(0)} km` },
    { label: 'apo alt', value: `${(member.r_apo_km - MOON_RADIUS_KM).toFixed(0)} km` },
    { label: 'ν₁', value: member.nu1.toFixed(3) },
    { label: 'ν₂', value: member.nu2.toFixed(3) },
    { label: 'residual', value: member.residual.toExponential(1) },
  ];
}

export function memberEndpointLabel(member: Member): string {
  const hp = (member.r_peri_km - MOON_RADIUS_KM).toFixed(0);
  return `#${member.index} · hp ${hp} km · e ${member.elements.e.toFixed(3)}`;
}

export interface LeftRailHooks {
  /** Per-term: would flipping this term land on a combo that exists in the catalog? */
  availability(): Record<keyof Terms, boolean>;
  onToggle(term: keyof Terms): void;
  onPinGhost(): void;
  onClearGhost(): void;
  onPreset(name: PresetName): void;
  onGraticule(visible: boolean): void;
}

export interface LeftRail {
  anim: AnimControls;
  setNotice(text: string): void;
  refresh(): void;
}

const TEMPLATE = `
  <section class="card">
    <h2>Force model</h2>
    <div id="toggle-board"></div>
    <p id="combo-name" class="muted"></p>
    <p id="notice" class="notice" hidden></p>
  </section>
  <section class="card">
    <h2>Families</h2>
    <div id="family-list"></div>
  </section>
  <section class="card">
    <h2>Member</h2>
    <input id="member-slider" type="range" min="0" max="0" step="1" />
    <div class="endpoints"><span id="ep-lo"></span><span id="ep-hi"></span></div>
  </section>
  <section class="card">
    <h2>Animation</h2>
    <div id="anim-slot"></div>
    <div class="btn-row">
      <button id="pin-ghost" type="button">Pin ghost</button>
      <button id="clear-ghost" type="button">Clear ghost</button>
    </div>
  </section>
  <section class="card">
    <h2>View</h2>
    <div class="btn-row">
      <button id="preset-pole" type="button">South-pole view</button>
      <button id="preset-earth" type="button">Earth-line view</button>
    </div>
    <label class="toggle-row"><input id="graticule-box" type="checkbox" checked /> lat/lon graticule</label>
  </section>
  <section class="card">
    <h2>Selected member</h2>
    <dl id="readout"></dl>
  </section>`;

export function mountLeftRail(
  container: HTMLElement,
  store: Store,
  catalog: Catalog,
  hooks: LeftRailHooks,
): LeftRail {
  container.innerHTML = TEMPLATE;
  const pick = <T extends Element>(sel: string): T => container.querySelector(sel) as T;

  const anim = mountAnimControls(pick<HTMLElement>('#anim-slot'), store);
  const slider = pick<HTMLInputElement>('#member-slider');
  const notice = pick<HTMLParagraphElement>('#notice');
  const clearGhost = pick<HTMLButtonElement>('#clear-ghost');

  slider.addEventListener('input', () => {
    store.update({ memberIndex: Number(slider.value), animTime: 0 });
  });
  pick<HTMLButtonElement>('#pin-ghost').addEventListener('click', () => hooks.onPinGhost());
  clearGhost.addEventListener('click', () => hooks.onClearGhost());
  pick<HTMLButtonElement>('#preset-pole').addEventListener('click', () => hooks.onPreset('south-pole'));
  pick<HTMLButtonElement>('#preset-earth').addEventListener('click', () => hooks.onPreset('earth-line'));
  const gratBox = pick<HTMLInputElement>('#graticule-box');
  gratBox.addEventListener('change', () => hooks.onGraticule(gratBox.checked));

  const currentCombo = (): Combo => comboById(catalog, store.get().comboId) ?? catalog.combos[0];
  const currentFamily = (): Family => {
    const combo = currentCombo();
    return familyByN(combo, store.get().familyN) ?? combo.families[0];
  };

  function renderToggles(): void {
    const combo = currentCombo();
    const avail = hooks.availability();
    const board = pick<HTMLDivElement>('#toggle-board');
    board.innerHTML = '';
    for (const [term, label] of TERM_LABELS) {
      const row = document.createElement('label');
      row.className = 'toggle-row';
      const box = document.createElement('input');
      box.type = 'checkbox';
      box.checked = combo.terms[term];
      box.disabled = !avail[term];
      if (!avail[term]) {
        row.classList.add('disabled');
        row.title = 'not in catalog';
      }
      box.addEventListener('change', () => hooks.onToggle(term));
      row.append(box, document.createTextNode(` ${label}`));
      board.appendChild(row);
    }
    pick<HTMLElement>('#combo-name').textContent = combo.name;
  }

  function renderFamilies(): void {
    const combo = currentCombo();
    const from = currentFamily();
    const list = pick<HTMLDivElement>('#family-list');
    list.innerHTML = '';
    for (const fam of combo.families) {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = `family-btn${fam.resonance_n === from.resonance_n ? ' active' : ''}`;
      btn.textContent = `N = ${fam.resonance_n} · ${fam.members.length} members`;
      btn.addEventListener('click', () => {
        const idx = nearestMemberIndex(store.get().memberIndex, from.members.length, fam.members.length);
        store.update({ familyN: fam.resonance_n, memberIndex: idx, animTime: 0 });
      });
      list.appendChild(btn);
    }
  }

  function renderMember(): void {
    const family = currentFamily();
    const idx = Math.min(Math.max(0, store.get().memberIndex), family.members.length - 1);
    const member = family.members[idx];

    slider.max = String(family.members.length - 1);
    slider.value = String(idx);
    pick<HTMLElement>('#ep-lo').textContent = memberEndpointLabel(family.members[0]);
    pick<HTMLElement>('#ep-hi').textContent = memberEndpointLabel(family.members[family.members.length - 1]);

    const dl = pick<HTMLElement>('#readout');
    dl.innerHTML = '';
    for (const row of formatReadout(member, family)) {
      const dt = document.createElement('dt');
      dt.textContent = row.label;
      const dd = document.createElement('dd');
      dd.textContent = row.value;
      dl.append(dt, dd);
    }
    clearGhost.disabled = store.get().ghost === null;
  }

  function refresh(): void {
    renderToggles();
    renderFamilies();
    renderMember();
  }

  function setNotice(text: string): void {
    notice.textContent = text;
    notice.hidden = text === '';
  }

  // animTime ticks every frame — only redraw on the fields the rail displays.
  store.subscribe((s, p) => {
    if (s.comboId !== p.comboId || s.familyN !== p.familyN
        || s.memberIndex !== p.memberIndex || s.ghost !== p.ghost) {
      refresh();
    }
  });
  refresh();

  return { anim, setNotice, refresh };
}
```

- [ ] **Step 4: Implement `web/src/ui/stabilityPlot.ts`**

```ts
import { scaleLinear } from 'd3-scale';
import { symlog } from '../state';
import type { Store } from '../state';
import type { Family } from '../types';

const SVG_NS = 'http://www.w3.org/2000/svg';
const MARGIN = { top: 14, right: 18, bottom: 26, left: 48 };

/** Symmetric symlog y-range that always contains the ±1 stability boundary. */
export function symlogDomain(nus: number[]): [number, number] {
  let m = 1.2;
  for (const v of nus) m = Math.max(m, Math.abs(symlog(v)) * 1.1);
  return [-m, m];
}

/** Plot-local x (px, already offset by the left margin) to a member index. */
export function indexFromX(px: number, plotWidth: number, count: number): number {
  if (count <= 1) return 0;
  const f = Math.min(1, Math.max(0, px / plotWidth));
  return Math.round(f * (count - 1));
}

export interface StabilityPlot {
  setFamily(family: Family): void;
  refresh(): void;
}

export function mountStabilityPlot(container: HTMLElement, store: Store): StabilityPlot {
  container.innerHTML = '';
  const svg = document.createElementNS(SVG_NS, 'svg');
  svg.setAttribute('class', 'stability-svg');
  container.appendChild(svg);

  let family: Family | null = null;

  const el = (name: string, attrs: Record<string, string>): SVGElement => {
    const node = document.createElementNS(SVG_NS, name) as SVGElement;
    for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
    return node;
  };
  const text = (attrs: Record<string, string>, content: string): SVGElement => {
    const node = el('text', attrs);
    node.textContent = content;
    return node;
  };

  const innerWidth = (): number =>
    Math.max(10, (container.clientWidth || 800) - MARGIN.left - MARGIN.right);

  function draw(): void {
    svg.innerHTML = '';
    if (!family) return;
    const w = container.clientWidth || 800;
    const h = container.clientHeight || 210;
    const iw = innerWidth();
    const ih = Math.max(10, h - MARGIN.top - MARGIN.bottom);
    svg.setAttribute('width', String(w));
    svg.setAttribute('height', String(h));
    svg.setAttribute('viewBox', `0 0 ${w} ${h}`);

    const members = family.members;
    const n = members.length;
    const domain = symlogDomain(members.flatMap((m) => [m.nu1, m.nu2]));
    const x = scaleLinear().domain([0, Math.max(1, n - 1)]).range([MARGIN.left, MARGIN.left + iw]);
    const y = scaleLinear().domain(domain).range([MARGIN.top + ih, MARGIN.top]);

    svg.appendChild(el('line', {
      x1: String(MARGIN.left), x2: String(MARGIN.left + iw),
      y1: String(y(0)), y2: String(y(0)), class: 'axis',
    }));
    for (const b of [1, -1]) {
      svg.appendChild(el('line', {
        x1: String(MARGIN.left), x2: String(MARGIN.left + iw),
        y1: String(y(b)), y2: String(y(b)), class: 'boundary',
      }));
    }

    const series = (pick: (i: number) => number, cls: string): void => {
      const pts = members.map((_, i) => `${x(i)},${y(symlog(pick(i)))}`).join(' ');
      svg.appendChild(el('polyline', { points: pts, class: cls }));
    };
    series((i) => members[i].nu1, 'nu1');
    series((i) => members[i].nu2, 'nu2');

    const idx = Math.min(Math.max(0, store.get().memberIndex), n - 1);
    svg.appendChild(el('line', {
      x1: String(x(idx)), x2: String(x(idx)),
      y1: String(MARGIN.top), y2: String(MARGIN.top + ih), class: 'cursor',
    }));

    for (const [v, label] of [[1, '+1'], [0, '0'], [-1, '−1']] as Array<[number, string]>) {
      svg.appendChild(text(
        { x: String(MARGIN.left - 8), y: String(y(v) + 4), class: 'tick', 'text-anchor': 'end' },
        label,
      ));
    }
    svg.appendChild(text({ x: String(MARGIN.left), y: String(h - 8), class: 'tick' }, 'member 0'));
    svg.appendChild(text(
      { x: String(MARGIN.left + iw), y: String(h - 8), class: 'tick', 'text-anchor': 'end' },
      `member ${n - 1}`,
    ));
    svg.appendChild(text(
      { x: String(MARGIN.left + 6), y: String(MARGIN.top + 12), class: 'legend' },
      'symlog ν₁ (amber) / ν₂ (teal) — |ν| ≤ 1 is linearly stable',
    ));
  }

  let dragging = false;
  const pickMember = (ev: MouseEvent): void => {
    if (!family) return;
    const rect = svg.getBoundingClientRect();
    const iw = Math.max(10, rect.width - MARGIN.left - MARGIN.right);
    const idx = indexFromX(ev.clientX - rect.left - MARGIN.left, iw, family.members.length);
    if (idx !== store.get().memberIndex) store.update({ memberIndex: idx, animTime: 0 });
  };

  svg.addEventListener('mousedown', (ev) => {
    dragging = true;
    pickMember(ev);
  });
  window.addEventListener('mousemove', (ev) => {
    if (dragging) pickMember(ev);
  });
  window.addEventListener('mouseup', () => {
    dragging = false;
  });
  window.addEventListener('resize', () => draw());
  store.subscribe((s, p) => {
    if (s.memberIndex !== p.memberIndex || s.familyN !== p.familyN || s.comboId !== p.comboId) draw();
  });

  return {
    setFamily(f) {
      family = f;
      draw();
    },
    refresh: draw,
  };
}
```

- [ ] **Step 5: Replace `web/src/style.css` with the full cockpit stylesheet**

```css
:root { color-scheme: dark; }

html, body {
  margin: 0;
  height: 100%;
  background: #05070d;
  color: #d7dee8;
  font: 13px/1.45 ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
}

#app {
  display: grid;
  grid-template-columns: 320px 1fr;
  grid-template-rows: 1fr 210px;
  grid-template-areas:
    "rail stage"
    "plot plot";
  height: 100vh;
}

#rail  { grid-area: rail;  overflow-y: auto; border-right: 1px solid #1b2331; }
#stage { grid-area: stage; position: relative; min-width: 0; min-height: 0; }
#stage canvas { display: block; }
#plot  { grid-area: plot;  border-top: 1px solid #1b2331; position: relative; }

.card { padding: 10px 12px; border-bottom: 1px solid #141b26; }
.card h2 {
  margin: 0 0 8px; font-size: 11px; font-weight: 600;
  letter-spacing: 0.09em; text-transform: uppercase; color: #6f8299;
}
.muted { color: #7b8a9d; margin: 6px 0 0; }

.toggle-row { display: block; padding: 2px 0; cursor: pointer; }
.toggle-row.disabled { color: #4d5a6b; cursor: not-allowed; }

.notice {
  margin: 8px 0 0; padding: 6px 8px;
  background: #2a2313; border-left: 2px solid #c79a2e;
  color: #e0c98a;
}

.family-btn {
  display: block; width: 100%; margin-bottom: 4px; padding: 5px 8px;
  text-align: left; background: #101725; color: #b9c6d6;
  border: 1px solid #1e2938; border-radius: 4px; cursor: pointer;
  font-variant-numeric: tabular-nums;
}
.family-btn:hover  { background: #17203026; border-color: #2b3a50; }
.family-btn.active { background: #1d2b3f; color: #ffc24a; border-color: #3d5271; }

#member-slider { width: 100%; }
.endpoints {
  display: flex; justify-content: space-between; gap: 8px;
  font-size: 11px; color: #7b8a9d; font-variant-numeric: tabular-nums;
}

.btn-row { display: flex; gap: 6px; margin-top: 8px; }
.btn-row button, .anim-controls button {
  background: #16202e; color: #d7dee8;
  border: 1px solid #2b3a50; border-radius: 4px;
  padding: 4px 10px; cursor: pointer;
}
.btn-row button:hover, .anim-controls button:hover { background: #1e2c3f; }
.btn-row button:disabled { color: #4d5a6b; border-color: #1b2331; cursor: not-allowed; }

.anim-controls { display: flex; flex-direction: column; gap: 8px; }
.anim-controls button { align-self: flex-start; padding: 4px 14px; }
.dial { display: flex; align-items: center; gap: 8px; }
.dial input[type="range"] { flex: 1; }
.readout-line { font-variant-numeric: tabular-nums; color: #9fb0c4; }

#readout {
  display: grid; grid-template-columns: 72px 1fr;
  gap: 2px 10px; margin: 0; font-variant-numeric: tabular-nums;
}
#readout dt { color: #7b8a9d; }
#readout dd { margin: 0; color: #d7dee8; }

.stability-svg { display: block; width: 100%; height: 100%; cursor: crosshair; }
.stability-svg .axis     { stroke: #2b3a50; stroke-width: 1; }
.stability-svg .boundary { stroke: #c74a4a; stroke-width: 1; stroke-dasharray: 4 4; }
.stability-svg .cursor   { stroke: #ffc24a; stroke-width: 1.5; }
.stability-svg .nu1      { fill: none; stroke: #ffc24a; stroke-width: 1.5; }
.stability-svg .nu2      { fill: none; stroke: #4fd1c5; stroke-width: 1.5; }
.stability-svg .tick     { fill: #7b8a9d; font-size: 10px; }
.stability-svg .legend   { fill: #5d6d80; font-size: 10px; }
```

- [ ] **Step 6: Replace `web/src/main.ts` to mount the cockpit**

```ts
import './style.css';
import { advanceTime, createLoop, createSatellite, SPEED_DEFAULT } from './anim';
import { familyPreview, loadCatalog, memberTrajectory, prefetchNeighbors } from './data';
import { createStage } from './scene';
import { comboById, createStore, familyByN } from './state';
import { mountLeftRail } from './ui/leftRail';
import { mountStabilityPlot } from './ui/stabilityPlot';
import type { Family, Terms } from './types';

const CATALOG_BASE = 'catalog';

async function boot(): Promise<void> {
  const catalog = await loadCatalog(CATALOG_BASE);
  const combo0 = catalog.combos[0];
  const family0 = combo0.families[0];
  const store = createStore({
    comboId: combo0.id,
    familyN: family0.resonance_n,
    memberIndex: Math.floor(family0.members.length / 2),
    animTime: 0,
    playing: true,
    speed: SPEED_DEFAULT,
    ghost: null,
  });

  const stage = createStage(document.getElementById('stage') as HTMLElement);
  const satellite = createSatellite();
  stage.scene.add(satellite.group);
  const plot = mountStabilityPlot(document.getElementById('plot') as HTMLElement, store);

  const rail = mountLeftRail(document.getElementById('rail') as HTMLElement, store, catalog, {
    // Task 21 replaces this with termAvailability(catalog, currentCombo().terms).
    availability: (): Record<keyof Terms, boolean> =>
      ({ j2: false, c22: false, j3: false, earth: false }),
    onToggle: () => undefined,
    onPinGhost: () => {
      const s = store.get();
      store.update({ ghost: { comboId: s.comboId, familyN: s.familyN, memberIndex: s.memberIndex } });
    },
    onClearGhost: () => store.update({ ghost: null }),
    onPreset: (name) => stage.applyPreset(name),
    onGraticule: (visible) => stage.setGraticule(visible),
  });

  const currentFamily = (): Family => {
    const combo = comboById(catalog, store.get().comboId) ?? catalog.combos[0];
    return familyByN(combo, store.get().familyN) ?? combo.families[0];
  };

  let generation = 0;
  async function refreshAll(): Promise<void> {
    const gen = ++generation;
    const family = currentFamily();
    const idx = Math.min(Math.max(0, store.get().memberIndex), family.members.length - 1);
    const member = family.members[idx];
    plot.setFamily(family);
    stage.setFrameRadiusKm(member.r_apo_km);
    const [loops, traj] = await Promise.all([
      familyPreview(CATALOG_BASE, family),
      memberTrajectory(CATALOG_BASE, member),
    ]);
    if (gen !== generation) return;
    stage.setFamilyStack(loops);
    stage.setSelected(traj);
    satellite.setMember(traj, member.period_s, family.resonance_n);
    prefetchNeighbors(CATALOG_BASE, family, idx);
  }

  store.subscribe((s, p) => {
    if (s.comboId !== p.comboId || s.familyN !== p.familyN || s.memberIndex !== p.memberIndex) {
      void refreshAll();
    }
  });
  await refreshAll();
  stage.applyPreset('south-pole');

  window.addEventListener('resize', () => stage.resize());
  createLoop((dtWall) => {
    const s = store.get();
    const family = currentFamily();
    const member = family.members[Math.min(Math.max(0, s.memberIndex), family.members.length - 1)];
    if (s.playing) {
      store.update({ animTime: advanceTime(s.animTime, dtWall, s.speed, member.period_s) });
    }
    const info = satellite.update(store.get().animTime);
    if (info) rail.anim.setReadout(info.days, info.revs, family.resonance_n);
    stage.render();
  }).start();
}

boot().catch((err: unknown) => {
  document.body.insertAdjacentHTML(
    'afterbegin',
    `<pre style="color:#ff6b6b;padding:12px">${String(err)}</pre>`,
  );
});
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cd web && npm test`
Expected: PASS — `ui/leftRail.test.ts`, `ui/stabilityPlot.test.ts`, and all earlier suites.
Also run `cd web && npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 8: Manual verification of the cockpit layout**

Run: `cd web && npm run dev` and confirm:
1. Three panes: a 320 px left rail, the 3D stage, and a full-width stability strip along the bottom — no page scrollbars.
2. The rail shows Force model (four checkboxes, all disabled and greyed — this task's `availability` stub returns all-false; Task 21 supplies the real computation), Families, Member, Animation, View, and Selected member cards.
3. Clicking `N = 30 · 7 members` swaps the family; the 3D stack changes and the slider range updates.
4. Dragging the member slider highlights a different amber orbit and every value in the readout card changes.
5. The bottom strip draws two polylines (amber ν₁, teal ν₂) with dashed red lines at ±1 and an amber vertical cursor that tracks the slider.
6. Clicking and dragging on the bottom strip moves the slider and the highlighted orbit — the plot works as an alternate slider.
7. "South-pole view" and "Earth-line view" snap the camera; unchecking "lat/lon graticule" hides the green grid.
8. "Clear ghost" starts greyed out; clicking "Pin ghost" enables it, and clicking it greys it out again. (The gray ghost orbit itself is rendered in Task 21 — only the button state is wired here.)

- [ ] **Step 9: Commit**

```bash
git add -A && git commit -m "feat: cockpit left rail, stability plot, and grid layout"
```

---

### Task 21: Sensitivity wiring — combo matching, family/member carry-over, ghost rendering

**Files:**
- Modify: `web/src/state.ts` (append the combo-matching helpers), `web/src/main.ts` (replace the whole file)
- Create: `web/src/combos.test.ts`

**Interfaces:**
- Consumes: everything from Tasks 16–20.
- Produces:
  - `state.ts` additions: `flipTerm(terms: Terms, term: keyof Terms): Terms`, `findCombo(catalog: Catalog, terms: Terms): Combo | undefined`, `termAvailability(catalog: Catalog, terms: Terms): Record<keyof Terms, boolean>`, `nearestResonance(combo: Combo, n: number): number | null`
  - `main.ts`: the complete cockpit wiring. No new exports.
- Behaviour locked by this task:
  - A checkbox is enabled iff `findCombo(catalog, flipTerm(currentTerms, term))` exists; otherwise it renders disabled with `title="not in catalog"`.
  - On a legal flip: keep the same `resonance_n` if the target combo has it (member carried over by `nearestMemberIndex`); otherwise select `nearestResonance` and show the notice `No frozen N=<from> family in <combo name> — showing N=<to>`.
  - The ghost pin stores `{comboId, familyN, memberIndex}` and renders that member's full-resolution trajectory in gray, independent of the current selection.

- [ ] **Step 1: Write the failing tests**

`web/src/combos.test.ts`:
```ts
import { describe, expect, it } from 'vitest';
import { comboById, findCombo, flipTerm, nearestResonance, termAvailability } from './state';
import { makeCatalog } from './testFixtures';
import type { Terms } from './types';

const FULL: Terms = { j2: true, c22: true, j3: true, earth: true };

describe('flipTerm', () => {
  it('returns a new object with exactly one term inverted', () => {
    const flipped = flipTerm(FULL, 'c22');
    expect(flipped).toEqual({ j2: true, c22: false, j3: true, earth: true });
    expect(FULL.c22).toBe(true);
  });
});

describe('findCombo', () => {
  it('matches on the full four-term signature', () => {
    expect(findCombo(makeCatalog(), FULL)?.id).toBe('full');
    expect(findCombo(makeCatalog(), flipTerm(FULL, 'c22'))?.id).toBe('no-c22');
    expect(findCombo(makeCatalog(), flipTerm(FULL, 'earth'))?.id).toBe('no-earth');
  });

  it('returns undefined for a toggle state outside the curated set', () => {
    expect(findCombo(makeCatalog(), flipTerm(FULL, 'j2'))).toBeUndefined();
    expect(findCombo(makeCatalog(), { j2: false, c22: false, j3: false, earth: false })).toBeUndefined();
  });
});

describe('termAvailability', () => {
  it('marks only the flips that land on a catalogued combo', () => {
    expect(termAvailability(makeCatalog(), FULL)).toEqual({
      j2: false, c22: true, j3: false, earth: true,
    });
  });

  it('is computed from the target combo, so it changes as you move around', () => {
    const noEarth = findCombo(makeCatalog(), flipTerm(FULL, 'earth'))!;
    // flipping earth back on returns to "full"; nothing else is catalogued
    expect(termAvailability(makeCatalog(), noEarth.terms)).toEqual({
      j2: false, c22: false, j3: false, earth: true,
    });
  });
});

describe('nearestResonance', () => {
  it('picks the closest available resonance in the target combo', () => {
    const cat = makeCatalog();
    expect(nearestResonance(comboById(cat, 'full')!, 40)).toBe(30);
    expect(nearestResonance(comboById(cat, 'full')!, 25)).toBe(25);
    expect(nearestResonance(comboById(cat, 'no-earth')!, 25)).toBe(40);
  });

  it('returns null for a combo with no families', () => {
    expect(nearestResonance({ id: 'x', name: 'x', terms: FULL, families: [] }, 25)).toBeNull();
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd web && npm test -- combos`
Expected: FAIL — `flipTerm`/`findCombo`/`termAvailability`/`nearestResonance` are not exported from `./state`.

- [ ] **Step 3: Append the combo-matching helpers to `web/src/state.ts`**

Task 17 already put `import type { Catalog, Combo, Family } from './types';` at the top of `state.ts`. Extend it to:
```ts
import type { Catalog, Combo, Family, Terms } from './types';
```

Append to the bottom of `state.ts`:
```ts
/** A new toggle state with exactly one force term inverted. */
export function flipTerm(terms: Terms, term: keyof Terms): Terms {
  return { ...terms, [term]: !terms[term] };
}

/** The catalogued combo whose four active terms match exactly, if any. */
export function findCombo(catalog: Catalog, terms: Terms): Combo | undefined {
  return catalog.combos.find(
    (c) => c.terms.j2 === terms.j2 && c.terms.c22 === terms.c22
      && c.terms.j3 === terms.j3 && c.terms.earth === terms.earth,
  );
}

/**
 * Per term: is the combo you would land on by flipping it present in the catalog?
 * Terms that fail this render as disabled checkboxes titled "not in catalog".
 */
export function termAvailability(catalog: Catalog, terms: Terms): Record<keyof Terms, boolean> {
  const keys: Array<keyof Terms> = ['j2', 'c22', 'j3', 'earth'];
  const out = {} as Record<keyof Terms, boolean>;
  for (const k of keys) out[k] = findCombo(catalog, flipTerm(terms, k)) !== undefined;
  return out;
}

/** Closest resonance actually present in a combo, or null when it has no families. */
export function nearestResonance(combo: Combo, n: number): number | null {
  if (combo.families.length === 0) return null;
  let best = combo.families[0];
  for (const f of combo.families) {
    if (Math.abs(f.resonance_n - n) < Math.abs(best.resonance_n - n)) best = f;
  }
  return best.resonance_n;
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd web && npm test -- combos`
Expected: PASS — all five describe blocks green.

- [ ] **Step 5: Replace `web/src/main.ts` with the complete wiring**

```ts
import './style.css';
import { advanceTime, createLoop, createSatellite, SPEED_DEFAULT } from './anim';
import { familyPreview, loadCatalog, memberTrajectory, prefetchNeighbors } from './data';
import { createStage } from './scene';
import {
  comboById, createStore, familyByN, findCombo, flipTerm,
  nearestMemberIndex, nearestResonance, termAvailability,
} from './state';
import { mountLeftRail } from './ui/leftRail';
import { mountStabilityPlot } from './ui/stabilityPlot';
import type { Combo, Family, Terms } from './types';

const CATALOG_BASE = 'catalog';

async function boot(): Promise<void> {
  const catalog = await loadCatalog(CATALOG_BASE);
  const combo0 = catalog.combos[0];
  const family0 = combo0.families[0];

  const store = createStore({
    comboId: combo0.id,
    familyN: family0.resonance_n,
    memberIndex: Math.floor(family0.members.length / 2),
    animTime: 0,
    playing: true,
    speed: SPEED_DEFAULT,
    ghost: null,
  });

  const stage = createStage(document.getElementById('stage') as HTMLElement);
  const satellite = createSatellite();
  stage.scene.add(satellite.group);
  const plot = mountStabilityPlot(document.getElementById('plot') as HTMLElement, store);

  const currentCombo = (): Combo => comboById(catalog, store.get().comboId) ?? catalog.combos[0];
  const currentFamily = (): Family => {
    const combo = currentCombo();
    return familyByN(combo, store.get().familyN) ?? combo.families[0];
  };

  const rail = mountLeftRail(document.getElementById('rail') as HTMLElement, store, catalog, {
    availability: () => termAvailability(catalog, currentCombo().terms),
    onToggle: (term) => toggleTerm(term),
    onPinGhost: () => {
      const s = store.get();
      store.update({ ghost: { comboId: s.comboId, familyN: s.familyN, memberIndex: s.memberIndex } });
    },
    onClearGhost: () => store.update({ ghost: null }),
    onPreset: (name) => stage.applyPreset(name),
    onGraticule: (visible) => stage.setGraticule(visible),
  });

  /** Toggle one force term: swap combos, carry the family and member across. */
  function toggleTerm(term: keyof Terms): void {
    const from = currentCombo();
    const target = findCombo(catalog, flipTerm(from.terms, term));
    if (!target) {
      rail.refresh(); // defensive: the checkbox should have been disabled
      return;
    }
    const fromFamily = currentFamily();
    const wantN = fromFamily.resonance_n;

    let toFamily = familyByN(target, wantN);
    if (toFamily) {
      rail.setNotice('');
    } else {
      const n = nearestResonance(target, wantN);
      if (n === null) {
        rail.setNotice(`${target.name} has no frozen families in the catalog`);
        rail.refresh();
        return;
      }
      toFamily = familyByN(target, n) as Family;
      rail.setNotice(`No frozen N=${wantN} family in ${target.name} — showing N=${n}`);
    }

    const memberIndex = nearestMemberIndex(
      store.get().memberIndex, fromFamily.members.length, toFamily.members.length,
    );
    store.update({
      comboId: target.id,
      familyN: toFamily.resonance_n,
      memberIndex,
      animTime: 0,
    });
  }

  let generation = 0;
  async function refreshAll(): Promise<void> {
    const gen = ++generation;
    const family = currentFamily();
    const idx = Math.min(Math.max(0, store.get().memberIndex), family.members.length - 1);
    const member = family.members[idx];

    plot.setFamily(family);
    stage.setFrameRadiusKm(member.r_apo_km);
    const [loops, traj] = await Promise.all([
      familyPreview(CATALOG_BASE, family),
      memberTrajectory(CATALOG_BASE, member),
    ]);
    if (gen !== generation) return; // a newer selection won the race
    stage.setFamilyStack(loops);
    stage.setSelected(traj);
    satellite.setMember(traj, member.period_s, family.resonance_n);
    prefetchNeighbors(CATALOG_BASE, family, idx);
  }

  let ghostKey = '';
  async function refreshGhost(): Promise<void> {
    const g = store.get().ghost;
    const key = g ? `${g.comboId}/${g.familyN}/${g.memberIndex}` : '';
    if (key === ghostKey) return;
    ghostKey = key;
    if (!g) {
      stage.setGhost(null);
      return;
    }
    const combo = comboById(catalog, g.comboId);
    const family = combo ? familyByN(combo, g.familyN) : undefined;
    const member = family?.members[g.memberIndex];
    stage.setGhost(member ? await memberTrajectory(CATALOG_BASE, member) : null);
  }

  store.subscribe((s, p) => {
    if (s.comboId !== p.comboId || s.familyN !== p.familyN || s.memberIndex !== p.memberIndex) {
      void refreshAll();
    }
    if (s.ghost !== p.ghost) void refreshGhost();
  });

  await refreshAll();
  stage.applyPreset('south-pole');
  window.addEventListener('resize', () => stage.resize());

  createLoop((dtWall) => {
    const s = store.get();
    const family = currentFamily();
    const member = family.members[Math.min(Math.max(0, s.memberIndex), family.members.length - 1)];
    if (s.playing) {
      store.update({ animTime: advanceTime(s.animTime, dtWall, s.speed, member.period_s) });
    }
    const info = satellite.update(store.get().animTime);
    if (info) rail.anim.setReadout(info.days, info.revs, family.resonance_n);
    stage.render();
  }).start();
}

boot().catch((err: unknown) => {
  document.body.insertAdjacentHTML(
    'afterbegin',
    `<pre style="color:#ff6b6b;padding:12px">${String(err)}</pre>`,
  );
});
```

- [ ] **Step 6: Run the full test suite and typecheck**

Run: `cd web && npm test && npx tsc --noEmit`
Expected: PASS with no type errors.

- [ ] **Step 7: Manual verification against the dev catalog**

Run: `cd web && npm run dev`. The dev catalog holds one combo (`full`), so every flip leaves the curated set — that is exactly the disabled-flip path:
1. All four checkboxes render checked, greyed, and non-clickable.
2. Hovering any of the four shows the tooltip "not in catalog".
3. "Pin ghost" now draws a gray copy of the current orbit; moving the slider leaves the gray copy where it was pinned while the amber orbit moves.
4. "Clear ghost" removes the gray orbit.

Combo-swapping, family carry-over, and the missing-family notice need a multi-combo catalog — they are verified in Task 22 against the full catalog.

- [ ] **Step 8: Commit**

```bash
git add -A && git commit -m "feat: force-term sensitivity wiring with combo swap, family carry-over, and ghost pin"
```

---

### Task 22: Full catalog run, README, and end-to-end manual verification

**Files:**
- Create: `README.md`
- Generate (git-ignored): `web/public/catalog/**` — the real catalog from `catalog.toml`

**Interfaces:**
- Consumes: the `elfo-catalog gen` CLI (Task 15), `catalog.toml` (Task 15), and the whole web app (Tasks 16–21).
- Produces: no code. This task's deliverable is a working, verified application plus its quickstart documentation.

- [ ] **Step 1: Generate the full catalog**

From the repo root:
```bash
cargo run -p elfo-catalog --release -- gen --config catalog.toml --out web/public/catalog
```
**This takes minutes to tens of minutes** (4 combos × 9 resonances × ~41 members, each member an STM-propagated multiple-shooting solve; rayon uses all cores). Watch stderr for the per-(combo, N) progress and for "family absent" notes — a combo missing a resonance is a physics result, not a failure.

Sanity-check the output:
```bash
du -sh web/public/catalog
python3 - <<'PY'
import json
c = json.load(open('web/public/catalog/catalog.json'))
print('schema', c['schema_version'], 'generated', c['generated'])
for combo in c['combos']:
    fams = [(f['resonance_n'], len(f['members'])) for f in combo['families']]
    print(f"{combo['id']:10s} {combo['terms']}  {fams}")
PY
```
Expected: `schema 1`; four combos (`full`, `no-c22`, `no-j3`, `no-earth`); each with several families; total on-disk size in the tens-to-hundreds of MB. If `no-earth` has noticeably fewer or different families than `full`, that is the sensitivity story the app exists to show — record what you saw in the commit message.

- [ ] **Step 2: Confirm the generated data is not committed**

Run: `git status --porcelain web/public/catalog | head`
Expected: **no output** — `.gitignore` (Task 1) lists `web/public/catalog/`. `web/public/moon.jpg` IS tracked (public-domain NASA/SVS imagery, so a fresh clone renders correctly); confirm with `git ls-files web/public`.

- [ ] **Step 3: Write `README.md`**

```markdown
# frozen — ELFO Family Browser

An interactive browser for **elliptical lunar frozen orbits (ELFOs)**: families of
orbits that are periodic in the Earth-Moon rotating frame, computed in the full
force model and explored in a 3D cockpit. Toggling individual gravitational terms
swaps the family in place, answering "which terms hold this orbit together?"

## What's here

| Path | What it does |
|---|---|
| `crates/elfo-core` | Dynamics, DP54 integrator with STM, multiple-shooting corrector, pseudo-arclength continuation, monodromy stability indices |
| `crates/elfo-catalog` | CLI that sweeps force-model combos × resonances and writes the catalog |
| `catalog.toml` | The real generation config (4 combos, 9 resonances) |
| `web/` | Vite + TypeScript + three.js cockpit (no framework, no WASM) |
| `web/dev-catalog.toml` | Small config for a fast development catalog |

Physics: Moon-centered Earth-Moon rotating frame, ω = 1 nondimensional, Earth fixed
at −x. Force terms J2, C22, J3 (closed form, static in this frame) and the Earth as
a point-mass third body, each individually toggleable. Frozen orbits are periodic
orbits found by differential correction and continued into families; each family is
labeled by its resonance N (revolutions per closure).

## Quickstart

Prerequisites: a Rust toolchain (stable) and Node 20+.

```bash
# 1. Generate a catalog. The dev config is small and takes about a minute:
cargo run -p elfo-catalog --release -- gen --config web/dev-catalog.toml --out web/public/catalog

#    ...or generate the real one (minutes to tens of minutes):
cargo run -p elfo-catalog --release -- gen --config catalog.toml --out web/public/catalog

# 2. Run the cockpit.
cd web
npm install
npm run dev
```

Generated catalog data is git-ignored; only the generator configs are committed.

## Using the cockpit

- **Force model** — check/uncheck a term to swap to that force-model combo. Toggles
  whose flip would leave the curated combo set are disabled ("not in catalog"). The
  same resonance is kept when the target combo has it; otherwise the nearest
  resonance is selected and a notice explains the substitution. **A family being
  absent is a result, not an error.**
- **Families / Member** — pick a resonance, then scrub the member slider. All members
  of the family are drawn as translucent blue loops; the selected one is solid amber.
- **Pin ghost** — freezes the current orbit as a gray copy so you can toggle a term
  and see the deformation by direct comparison.
- **Animation** — play/pause, log speed dial from 1 min/s to 10 d/s, satellite marker
  with a fading trail, and a readout in elapsed days and revolutions.
- **Stability strip** — ν₁ and ν₂ per member on a symlog axis with the |ν| = 1
  boundary marked. Click or drag it as an alternate member slider.
- **View** — south-pole and Earth-line camera presets, lat/lon graticule toggle.

## Tests

```bash
cargo test --workspace     # physics: Kepler, energy conservation, Jacobians, CR3BP validation
cd web && npm test         # pure web logic: binary parsing, interpolation, combo matching
npx tsc --noEmit           # strict typecheck
```

three.js rendering is verified by eye, not by test.

## Known idealizations

The Moon is modeled as rotating at a constant rate with its spin axis perpendicular
to the Earth-Moon orbit plane, so its surface is exactly static in the rotating
frame. The real Moon librates ~±8° in longitude and ~±6.7° in latitude relative to
that frame, so real ground tracks smear by several degrees around these exact closed
tracks. The Sun, Earth-Moon eccentricity, and real ephemeris each nudge members from
periodic to quasi-periodic; all are phase 2.

## Design

Full design and roadmap: `docs/superpowers/specs/2026-08-11-elfo-family-browser-design.md`.
Implementation plan: `docs/superpowers/plans/2026-08-11-elfo-family-browser.md`.
```

- [ ] **Step 4: Run the full test suite one more time**

```bash
cargo test --workspace
cd web && npm test && npx tsc --noEmit && npm run build
```
Expected: all Rust tests pass, all vitest suites pass, no type errors, and `vite build` writes `web/dist` without warnings about missing imports.

- [ ] **Step 5: Manual verification checklist (full catalog)**

Run `cd web && npm run dev` against the **full** catalog and tick every line. Any failure is a bug to fix before the final commit, not a note to file.

1. The app boots straight into a lit, textured Moon with the family stack and a solid amber selected orbit — no console errors, no red error banner.
2. The Force model card shows four checkboxes matching the loaded combo's terms; starting from `full`, `C22`, `J3`, and `Earth` are enabled while `J2` is disabled with the tooltip "not in catalog" (the curated set in `catalog.toml` never turns J2 off).
3. The Families card lists every resonance in the current combo with its member count, and the active one is highlighted.
4. The family stack is visible as many translucent loops, and dragging the member slider moves the amber highlight through the stack one member at a time.
5. Every field in the Selected member card (a, e, i, ω, Ω, period, revs, peri alt, apo alt, ν₁, ν₂, residual) changes as the slider moves, and `revs` equals the selected family's N.
6. The stability plot's amber cursor follows the slider exactly; clicking elsewhere on the plot jumps the selection there and dragging scrubs it continuously.
7. Members whose plot curve crosses the dashed ±1 boundary read |ν| > 1 in the readout card, and members inside the band read |ν| ≤ 1 — the plot and the card agree.
8. With the animation playing, the satellite visibly dwells near apoapsis over the south pole and whips through periapsis; switching to "South-pole view" makes the dwell unmistakable.
9. The readout counts elapsed days and revolutions, reaching `rev N.0 / N` exactly as the trail closes on itself, then resetting.
10. Pin ghost, then uncheck **C22**: the app swaps to the `no-c22` combo, keeps the same N, selects the corresponding member, and the amber orbit visibly differs from the gray ghost still drawn beside it.
11. Re-check C22, then uncheck **Earth**: the `no-earth` combo loads; its orbits are markedly less eccentric (near-circular J2/J3-frozen geometry) or the notice reports that the N you were on has no family there and names the substitute.
12. When a family is missing from the target combo, the notice reads exactly `No frozen N=<from> family in <combo name> — showing N=<to>` and the Families card highlights the substituted resonance.
13. Toggling a term never leaves the stability plot, the readout card, and the 3D stage disagreeing about which member is selected.
14. "Clear ghost" removes the gray orbit and greys its own button out.
15. Scrubbing the slider quickly across a whole family stays smooth (neighbour prefetch) and never leaves a stale orbit rendered after you stop.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "docs: README quickstart and project overview; full catalog verified end to end"
```






