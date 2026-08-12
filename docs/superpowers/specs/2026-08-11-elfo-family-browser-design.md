# ELFO Family Browser — Design

**Date:** 2026-08-11
**Status:** Approved pending user review

## Purpose

A tool for building mission-design intuition about elliptical lunar frozen orbits
(ELFOs): browse precomputed frozen-orbit families in the Earth-Moon rotating frame,
animate members, see their (automatically repeating) lunar ground tracks, and see
how the families change when individual gravitational terms (J2, C22, J3, Earth
third body) are removed — the "which terms hold this orbit together" question.

Primary user: Juno, for personal mission-design intuition. Presentability to
customers/collaborators is a stretch goal, served by the web-app form factor
(shareable link), not by extra features.

Modeled on an NRHO family visualizer: precomputed families, a slider through
members, **only frozen orbits shown** — not an initial-condition tuning sandbox.

## Key dynamical decisions

- **Full numerical propagation now; averaged theory later.** Doubly-averaged
  models eliminate the fast angle, so they cannot produce trajectories, ground
  tracks, or resonance behavior; they are deferred to a future phase as a
  map-making layer. Phase 1 needs no averaged machinery (except one page of
  classical frozen-orbit algebra used as corrector *seeds*).
- **Frozen orbits are computed as periodic orbits of the rotating frame.** With
  J2, C22, J3 static in the Earth-Moon rotating frame (synchronous rotation; C22
  locked to the Earth-Moon line) and the Earth as a fixed third body, the system
  is autonomous. Frozen orbits = periodic orbits, found by differential
  correction + continuation, exactly as NRHO families are.
- **Repeating ground tracks are automatic.** The Moon's surface is fixed in the
  rotating frame (librations idealized away), so every rotating-frame-periodic
  orbit has a closed, repeating ground track with repeat period equal to its
  closure period. Families are labeled by resonance N (revs per closure). The
  synodic month becomes dynamically meaningful in phase 2: with the Sun on, only
  members commensurate with the synodic forcing stay strictly periodic.
- **Known idealizations** (documented in-app where relevant): real lunar
  librations (~±8° optical, longitude) smear real ground tracks; Sun, Earth-Moon
  eccentricity, and real ephemeris each nudge members from periodic to
  quasi-periodic. Watching which members degrade gracefully is a phase-2 feature,
  not a phase-1 bug.

## Architecture

Cargo workspace + static web app. No server logic; deployable to any static host.
No WASM in phase 1 — the browser only reads precomputed data.

```
frozen/
├── crates/
│   ├── elfo-core/     # dynamics, integrator, STM, multiple shooting, continuation
│   └── elfo-catalog/  # CLI: TOML config → families → data files (rayon-parallel)
└── web/               # Vite + TypeScript + three.js cockpit (no framework, no WASM)
```

Rust was chosen for the core so that ANISE (already in the user's toolchain) can
later slot in as a feature-gated ephemeris provider behind a small trait; phase 1
needs no ephemeris. WASM bindings for live in-browser propagation are phase 2;
the workspace structure anticipates both without building either.

## Force model (phase 1)

Moon-centered, Earth-Moon rotating frame; nondimensional internally, km & km/s at
API boundaries. Each term behind a boolean in a `ForceModel` config:

| Term | Model | Notes |
|---|---|---|
| Moon point mass | always on | |
| J2, C22, J3 | closed-form accelerations | individually toggleable; static in this frame. Closed-form (not generic degree-N) because generic normalized-harmonic code is a bug farm and three terms are a page of verifiable math |
| Earth third body | point mass, fixed at −x (circular) | toggleable; with it on, the system is the perturbed CR3BP |
| Sun third body | point mass, circular, sweeps at synodic rate | implemented in phase 1 for later use, but **off during family generation** (keeps the system autonomous); "Sun-on" propagation experiments are phase 2 |

Constants (GM values, R_moon, GRGM-derived J2/C22/J3, Earth-Moon distance) live
in one module with sources cited in comments.

Out of scope for phase 1: SRP, ER3BP (elliptic Earth-Moon), degree-N GRAIL field,
ephemeris, live propagation, averaged theory, quasi-periodic (non-resonant)
families, constellation/coverage/DOP features.

## Catalog generation (`elfo-core` + `elfo-catalog`)

- **Integrator:** adaptive embedded Runge-Kutta (DOP853-class) with dense output;
  optionally propagates the 6×6 STM (42 equations) using analytic Jacobians.
- **Corrector: multiple shooting** — roughly one segment per rev (orbits close
  after N ≈ 20–80 revs; single shooting is hopelessly ill-conditioned at that
  length). Free period; the autonomous time-shift nullspace is killed by
  **anchoring one node** to a section (e.g., periapsis on a chosen side). The
  anchor is a labeling device only — it does not constrain the orbit, its
  resonance, or its ground track. If a family drifts off its section, re-anchor;
  if shooting convergence disappoints, **collocation is the named escalation
  path**.
- **Continuation: pseudo-arclength**, step size adapted to corrector difficulty.
- **Seeding:** classical doubly-averaged frozen-orbit conditions (ω = ±90°,
  Lidov-Kozai + J2 e–i relation) → osculating elements → rotating-frame state →
  corrector. Seeds only; no averaged model is built.
- **Stability:** monodromy-matrix eigenvalues → two stability indices
  ν = ½(λ + 1/λ) per member (|ν| ≤ 1 linearly stable; |ν| > 1 growth by |λ|/rev).
  Index crossings of ±1 mark bifurcations — future family hunting grounds.
- **Catalog contents:** N-rev frozen families (each member closes after N revs
  in the rotating frame; see "Repeating ground tracks are automatic") for N spanning
  ELFO-relevant altitudes (target N ≈ 20–80, a ≈ 4,000–13,000 km; exact range
  tuned to where the corrector converges and orbits stay ELFO-like), for a
  **curated set of force-model combos** (e.g., full, no-C22, no-J3, no-Earth,
  Moon-only — not all 2^4), so every UI toggle state maps to a real file.
- **CLI:** TOML config listing combos + resonances; runs are embarrassingly
  parallel (rayon); one JSON family file + binary trajectory files per family.

## Data contract (catalog → web)

Small JSON for browsable metadata; raw binary for lazily fetched trajectories.
Every file carries `schema_version`.

**Conventions:** Moon-centered rotating frame; +x Earth→Moon (Earth at −x), +z
along Earth-Moon orbital angular momentum. Mean sub-Earth point at lon 0° on the
−x axis ⇒ ground track = `lat = asin(z/r)`, `lon = atan2(y,x) − 180°`. Units km,
km/s, s.

- **`catalog.json`** (~tens of KB): generation provenance (date, git hash,
  constants + sources); combo list (id, name, active terms); per combo, families
  with per-member metadata: initial state, period, resonance N, osculating
  elements at periapsis (a, e, i, ω, Ω; frame convention documented), stability
  indices, periapsis/apoapsis altitudes, periodicity residual, trajectory path.
- **`{combo}/{family}/{member}.f32`** (~72 KB): little-endian Float32 xyz triples,
  uniform time steps over one closure period (~100 pts/rev), resampled from dense
  output. Uniform sampling ⇒ no time array; count + period reconstruct timing.
- **`{combo}/{family}/preview.f32`** (~150–200 KB): all members decimated to
  ~1,000 points each, concatenated — powers the whole-family-at-once 3D stack.
- **Not stored:** ground tracks (client-side two-liner), velocities (schema
  versions up if a future feature needs them).

Scale estimate: ~5 combos × ~6 families × ~50 members ≈ 1 MB JSON + ~100 MB
binary on disk; a session touches a few MB.

## Web app (the cockpit)

Vite + TypeScript + three.js. **No UI framework**: vanilla TS with a tiny pub/sub
store (`{combo, family, member, animTime, playing}`). Plots are hand-rolled
SVG/canvas with d3-scale.

Layout (approved):

```
+--------+---------------------------+
| combo  |                           |
| toggles|        3D rotating        |
|--------|        frame view         |
| family |     (Moon, family stack,  |
| member |      animated member)     |
| slider |                           |
+--------+------------+--------------+
| stability vs member | ground track |
+---------------------+--------------+
```

- **3D stage:** true-scale textured Moon (LRO albedo), lat/lon grid toggle,
  south-pole marker, Earth-direction indicator at −x. Family stack: all members
  as translucent curves (from preview file), selected member solid. Animation:
  satellite marker + fading trail; play/pause; log speed dial (~minutes/s to
  days/s); readout in elapsed days + revs. Camera: orbit controls + presets
  (south-pole view, Earth-line view).
- **Left rail:** force-model toggle board (combos outside the curated set render
  disabled with tooltip); family selector (resonances + member counts); member
  slider (endpoint annotations: periapsis altitude, e); animation controls;
  selected-member readout card (a, e, i, ω, period, revs, ν₁, ν₂, altitudes).
- **Sensitivity interaction:** toggling a term swaps catalogs and auto-selects
  the nearest member (same resonance, closest family parameter). **Ghost pin:**
  freeze current orbit as a grey ghost, then toggle — deformation is visible by
  comparison. A family absent from a combo is reported plainly ("no frozen N=59
  family without Earth") — absence is a sensitivity result.
- **Bottom strip:** stability indices vs member (log above |ν| = 1, boundary
  marked, cursor at current member, clickable/scrubbable as an alternate
  slider); ground track on equirectangular map drawn progressively during
  animation with full track ghosted, plus a south-pole azimuthal view button.
- **Data loading:** boot from `catalog.json`; fetch member trajectories on
  selection with neighbor prefetch for smooth scrubbing.

## Validation & testing

Physics tested as physics (TDD throughout; superpowers workflow enforces):

1. Integrator vs analytic Kepler (many revs); step-halving confirms order.
2. **Jacobi-like integral conserved** with harmonics on or off (system is
   autonomous with static harmonics) — the most sensitive whole-system test.
3. Analytic Jacobians vs finite differences for every force term.
4. Literature reproduction: CR3BP L1/L2 positions; a published periodic orbit
   (e.g., JPL-catalog 9:2 NRHO state) closes to tolerance with harmonics off; a
   published ELFO geometry (Ely-style) stays frozen (bounded e–ω libration) in
   the full model.
5. Monodromy structure: reciprocal eigenvalue pairs, two unit eigenvalues,
   det = 1.

Catalog QA at generation time (fail → not written): periodicity residual under
tolerance (and stored), periapsis above surface, smooth progression of
period/elements/indices along family (discontinuity ⇒ suspected branch jump ⇒
flagged).

Web: pure logic (binary parsing, transforms, ground-track math, interpolation,
nearest-member matching) extracted and unit-tested; three.js rendering verified
by eye.

## Roadmap (post-phase-1 candidates, not committed)

- **Phase 2:** WASM live propagation ("watch it fall apart"; Sun-on experiments —
  synodic commensurability becomes physical); generic degree-N harmonics with
  truncation; averaged-theory map layer (e–ω portraits, continuous families);
  SRP; ER3BP.
- **Phase 3:** constellation layer — K members + phasing → south-pole coverage
  cones → GDOP maps (surface and orbiting users). The catalog data model is
  designed so this layer drops in without schema upheaval.
- **Whenever earned:** ANISE ephemeris behind the third-body trait.

Constellation-design considerations informing the roadmap (from brainstorming):
station-keeping Δv as design currency and shared-family plane matching;
surface+Earth simultaneous visibility with terrain-driven elevation masks;
GDOP before clocks, but clock stability and time-transfer cadence set the
ranging floor; broadcast-ephemeris representability of non-Keplerian orbits;
LunaNet/AFS/Coordinated Lunar Time interoperability; eclipse depth and
magnetotail passages; deployment, degradation, disposal.
