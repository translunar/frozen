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
| `catalog.toml` | The real generation config (4 combos, 13 resonances) |
| `web/` | Vite + TypeScript + three.js cockpit (no framework, no WASM) |
| `web/dev-catalog.toml` | Small config for a fast development catalog |

Physics: Moon-centered Earth-Moon rotating frame, ω = 1 nondimensional, Earth fixed
at −x. Force terms J2, C22, J3 (closed form, static in this frame) and the Earth as
a point-mass third body, each individually toggleable. Frozen orbits are periodic
orbits found by differential correction and continued into families; each family is
labeled by its resonance N (revolutions per closure). Continuation runs to ±40
members per seed at a step size ds0 = 2e-2, so families span wide periapsis-altitude
ranges rather than a narrow neighborhood around the seed solution.

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

- **Force model** — check/uncheck a term to swap to that force-model combo. From the
  `full` combo, C22, J3, and Earth can each be flipped off (the curated set in
  `catalog.toml` includes `no-c22`, `no-j3`, and `no-earth`); J2 is disabled with
  the tooltip "not in catalog" since the curated set never turns J2 off. Toggling
  Earth off is the headline sensitivity result: in the generated catalog, `no-earth`
  currently yields **zero** frozen families at any of the swept resonances — the
  app reports this via its "no family in this combo" notice rather than an error.
  **A family being absent is a result, not an error.** Read this honestly, though:
  absence here means *no family converged from our seeds*, not proof that none
  exists — and `no-earth` is not a like-for-like comparison in the first place,
  since its seed is a qualitatively different near-circular J2/J3 frozen geometry
  (the eccentric Lidov-Kozai-style seed used for the other three combos requires
  Earth's third-body term to be physically meaningful). Treat this as: the
  catalog's eccentric, Earth-driven families have no `no-earth` counterpart, not
  as evidence that frozen orbits cannot exist without Earth.
- **Families / Member** — pick a resonance, then scrub the member slider. All members
  of the family are drawn as translucent blue loops; the selected one is solid amber.
  Across `full`, `no-c22`, and `no-j3`, families exist at resonances N = 20, 25, 30,
  35, 40, 60 (~81 members each, spanning a wide periapsis-altitude range); N = 45, 50,
  70 are absent from all three — the continuation's corrector stalls there rather than
  finding a periodic solution, a solver/physics result recorded as an absence, not a bug.
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
