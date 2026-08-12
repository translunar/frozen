# Rational M:k resonances — implementation notes

Branch `dual-resonance`. Extends the catalog generator from integer `N:1` resonances
(N revs per node-regression period) to rational `M:k` (M revs per k node-regression
periods). `k = 2` with odd M gives the half-integer families the
[dual-resonance lit review](litreview-dual-resonance-elfos.md) identifies as the
physically interesting ones — 149:2 is the published ERGO repeat-ground-track orbit.

## What changed

| File | Change |
|---|---|
| `crates/elfo-core/src/seeds.rs` | `measure_closure` / `closure_period_and_a` take `(revs, closures)`; new `elfo_seed_resonant{,_checked}`; `elfo_seed{,_checked}` are `closures = 1` wrappers |
| `crates/elfo-catalog/src/config.rs` | `Resonance { revs, closures }` parsed from `"M"` / `"M:k"`; `members_per_direction_override` table |
| `crates/elfo-catalog/src/generate.rs` | `Resonance` threaded through; segment cap; `n{M}_{k}` directories; M:k restricted to the `full` combo |
| `crates/elfo-catalog/src/writer.rs` | `FamilyOut.closures` (serde default 1); output structs gained `Deserialize` so that default is real |
| `catalog.toml` | resonances as strings, plus `"149:2"`, `"111:2"`, `"99:2"`, `"173:2"` at 10 members/direction |

The corrector, continuation and stability code is untouched: a periodic orbit is a
periodic orbit, and none of that machinery ever knew what the period meant.

## The seed generalisation

The `N:1` solver iterated two conditions on `a`: the rotating-frame node azimuth
sweeps −2π at time `T`, and the N-th apoapsis passage lands on that same `T`. Three
places carry the resonance, and all three scale linearly in `k`:

- **closure target** −2π → −2πk (cumulative, on the already-unwrapped azimuth series,
  so no extra bookkeeping was needed);
- **search window** `1.15·2π` → `1.15·2π·k`, with the sample count still `400·1.15·M`
  so the *per-rev* sampling density is unchanged;
- **the `t_min` apoapsis guard** `0.5·2π/M` → `0.5·2π·k/M`, i.e. still half a rev,
  because a rev is now `≈ 2πk/M` long. This guard is the one that stops `sin(π) =
  +1.2e-16` from booking a spurious apoapsis at `t = 0` and shifting every index.

The Kepler starting guess becomes `a = (μ (k/M)²)^(1/3)` (mean motion `M/k`), and the
apoapsis index stays `M − 1` — the M-th passage.

`k` therefore never appears anywhere except as a factor on a target and a window.
That is the whole physical content of the change: the closure condition was already
"the node has come back", and "come back twice" is the same equation with a bigger
right-hand side.

### Verification

`seeds::tests::half_integer_seed_closes_after_two_node_periods` checks 53:2
(26.5 revs/node period) two ways, both on the seed alone:

1. **Period.** `T(53:2)` must sit within 2 % of twice the 26/27 single-closure
   interpolant. (For scale: the gap between the true closure period and the naive
   `T = 2π` guess this whole module exists to fix is ~4 %, so 2 % is a real bound,
   not a rubber stamp.)
2. **Rev count.** Exactly 53 periapsis passages in `[0, T)`. This is the M:k version
   of the existing off-by-one regression guard, and it catches an additional failure
   mode specific to this change: a solver that still targets a single −2π node sweep
   would return the 26- or 27-rev family at half the period, which every other
   assertion would happily accept.

The corrector is deliberately *not* run in the default suite for M:2 — 222+ segments
is minutes per solve. It is exercised by an `#[ignore]`d smoke test (below).

A second test, `configured_dual_resonance_seeds_land_on_the_screened_altitudes`,
cross-checks all four configured entries against the lit review's D-table, which
screens candidates by two-body period alone. The two calculations share no code — one
solves an exact resonant mean motion, the other measures a perturbed node sweep — so
the agreement is evidence, not tautology:

| entry | solved a | D-table a | offset | solved T |
|---|---|---|---|---|
| 173:2 | 4478.23 km | 4522.35 km | −0.98 % | 12.3963 |
| 149:2 | 4943.71 km | 4995.79 km | −1.04 % | 12.3826 |
| 111:2 | 6000.58 km | 6079.20 km | −1.29 % | 12.3318 |
| 99:2  | 6467.13 km | 6561.03 km | −1.43 % | 12.3039 |

The offset is one-sided and its size is predicted: the node regression closes the
orbit early (T = 12.383 vs 4π = 12.566 at 149:2), and `(T/4π)^(2/3) − 1 = −0.98 %`.
That is precisely the effect §6 item 6 of the lit review warns the Keplerian screen
cannot see, and it grows with altitude as the third-body torque does. The test asserts
the offset is negative and under 2 % — a dropped `k` anywhere would show as
`2^(2/3) = 1.59×`, not 1 %.

## Config surface

Resonances are now strings, never TOML integers:

```toml
resonances = ["18", "20", ..., "149:2", "111:2", "99:2", "173:2"]

[members_per_direction_override]
"149:2" = 10
```

One spelling per concept. `"25"` and `"25:1"` parse equal (so an override table that
spells out `:1` still matches), `Display` renders `k = 1` without the `:1`, and
anything else — `""`, `"0"`, `"25:0"`, `"1:2:3"`, `"25.5"`, a bare integer — is a hard
load error rather than a silent zero.

**Non-reduced fractions are accepted and not normalised.** `"50:2"` is `25:1` flown
twice: a wasteful thing to ask for, but a real periodic orbit, and silently renaming a
requested family (and its output directory) would be worse than honouring it. Even M
at k = 2 is exactly this case, which is why the four configured entries are all odd.

## Generation

- **Directories.** `n{M}` for k = 1 — byte-identical to the old layout, so existing
  catalogs and the current web build are unaffected — and `n{M}_{k}` otherwise
  (`full/n149_2/0.f32`).
- **Segments.** `m = 2M`, retrying at `4M`, capped at `MAX_SEGMENTS = 400` with a
  stderr warning. The Jacobian SVD is `(6m+1)³`, so an uncapped `4M` retry on 173:2
  would be 692 segments — a ~64× cost over the first attempt for what is usually a
  lost cause anyway. The cap never bites on a first attempt at any configured
  resonance (max `2M = 346`). When the cap collapses `2M` and `4M` onto the same `m`,
  the retry is skipped rather than re-run identically.
- **Combo scope.** M:k with k > 1 runs for the `full` combo only (`generates()`,
  unit-tested). Every skip prints a `skipped: …` line, so an absent `no-c22/n149_2` is
  a recorded decision, not a silent hole. Sensitivity variants of the heavy families
  are a separate, separately budgeted job.
- **Members.** 10 per direction for the four M:2 entries vs. the global 40.
- **Sampling.** Unchanged at `100·M` samples over the member's own period — which is
  now the full k-closure period, so it is still 100 samples/rev.
- **Failure reporting.** `first_member` now reports the corrector's own message
  (`stalled at residual …`) and the segment counts tried, instead of the previous
  fixed "stalled at m=2N and m=4N".

## Corrector convergence: measured, and it does not converge

**The M:2 families will probably be absent from the generated catalog.** This was
measured, not guessed, and the cause is not the M:k change.

`seeds::tests::dual_resonance_corrector_smoke_111_2` (release, `#[ignore]`d, ~2 min):

```bash
cargo test --release -p elfo-core -- --ignored --nocapture dual_resonance
```

```text
seed: T = 12.331841 nd, solved in 1.4s
seed defect at m=222: |R| = 8.438e-3
  control 25:1 at m=50:  seed |R| = 1.013e-3 -> converged 5.50e-12 in 1.9s
  control 56:1 at m=112: seed |R| = 4.378e-3 -> stalled at residual 4.905e-4 in 55.7s
m=222: stalled at residual 4.037e-3 after 46.4s
```

The 111:2 corrector halves the seed defect (8.4e-3 → 4.0e-3) and then the line search
fails: seven halvings from α = 1 buy nothing. That looks damning for M:k until you
read the second control.

**56:1 is a classical `k = 1` family at essentially the same altitude** — 111:2 is
55.5 revs per node period, so 56:1 brackets it, `a ≈ 6043 km` against `6079 km` — at
the same 2 nodes/rev density, seeded by the *unchanged* code path. It stalls too.
And the shipped catalog agrees: the `full` combo has families at
N = 18, 20, 22, 25, 27, 30, 35, 40, 60 and is **missing 45, 50, 55 and 70**. The
corrector's basin already fails across most of the high-N, low-`a` end of the sweep;
whether a given N lands inside it is not even monotonic (60 works, 55 and 56 do not).

The four configured M:2 entries sit at 43.5–86.5 revs per node period, i.e. squarely
inside that pre-existing dead band. So:

- **The M:k seed machinery is not implicated.** It produces a state and a period that
  genuinely close after M revs and k node sweeps; that is what the 53:2 test asserts,
  and the 111:2 seed's defect (8.4e-3) is within a factor of 2 of the same-altitude
  `k = 1` seed's (4.4e-3) — consistent with a period twice as long, and nowhere near
  the order-of-magnitude gap that a mis-posed closure condition would open.
- **Fixing this is a corrector problem, not a resonance one**: a better line search
  (the current one only halves, and only 7 times), trust-region or Levenberg damping,
  a seed refined past the current `1e-11` closure-ratio tolerance, or a continuation
  *in altitude* from a converged low-N member instead of a cold start. All of that is
  out of scope here and would improve the classical families at the same time.

The failure mode is benign and honest: `first_member` returns `None`, prints
`absent: combo full n=111:2: corrector stalled at m=[222, 400]: stalled at residual …`,
and the family is simply missing. **Plan on zero to four new families, read the
stderr log, and do not read an absence as a bug in this change** — the same log line
already appears for N = 45, 50, 55, 70 today.

One thing the generation run will cost regardless: each M:2 family burns a ~45 s
attempt at `m = 2M` plus a ~4-minute attempt at the 400-segment cap before recording
its absence. Four families × (full combo only) ≈ 20 minutes of wasted-but-honest work.

---

# Addendum: the seed cache and the hard-family campaign

Everything above stands as written except its conclusion. The section immediately
preceding this one says the M:2 families "will probably be absent" and that the
`k = 1` families at N = 45, 50, 55, 70 are absent for the same reason — a corrector
basin failure at the high-N, low-`a` end of the sweep. That diagnosis was right. The
prescribed cures (line search, trust region, Levenberg damping) were **all wrong**,
and measuring that is the most useful thing in this addendum.

## The seed cache (`seeds/`)

| File | Change |
|---|---|
| `crates/elfo-catalog/src/seedcache.rs` | **new** — `SeedRecord` / `AbsentFile` / `SeedCache`, the on-disk format and its validation |
| `crates/elfo-catalog/src/generate.rs` | `GenOptions`; `run_with`; warm start ahead of the analytic seed; absence skip; cache writeback after the parallel sweep |
| `crates/elfo-catalog/src/main.rs` | `--seeds`, `--write-seeds`, `--retry-absent` |
| `crates/elfo-catalog/examples/campaign.rs` | **new** — the hard-family driver: Kepler rescale, ladder, Levenberg–Marquardt, e-fraction reseed, verification |
| `crates/elfo-catalog/tests/end_to_end.rs` | cold/warm round trip, absence skip + retraction, stale-seed fallback |
| `seeds/` | **new, committed** — 41 first-member seeds + 2 absence files |

Conquering a hard family costs a search, and the result of the search is six numbers.
`seeds/` is a committed cache of those numbers, one JSON per (combo, resonance):

```text
seeds/{combo_id}/n{M}.json        # k = 1
seeds/{combo_id}/n{M}_{k}.json    # k > 1
seeds/{combo_id}/absent.json      # confirmed non-converging, with a note
```

The file names are `Resonance::dir()` verbatim, so the cache mirrors the catalog's
own output layout. A record holds `state0`, `period_nd`, the residual it was accepted
at, and the git hash that produced it — not the node set, which `seed_nodes`
reconstructs from `state0` and the period by propagation.

**A cached seed is a warm start, never an answer.** `generate.rs` still runs the
shipped corrector on it (`m = 2M`, retrying at `4M` — see below), and still requires
convergence. So a stale seed can slow a run or make a family absent; it cannot put an
unverified orbit in the catalog. Records are refused on a mismatch between the
record's own `combo_id`/`revs`/`closures` and the path it was filed under, on an
unrecognised `schema_version`, and on a state that is geometrically impossible (this
last one catches a state written in km instead of nondimensional units, which is the
one corruption that would drive the variational integrator into a step-size collapse
rather than merely failing to converge).

`absent.json` records families confirmed not to converge, so a run does not spend
~5 minutes per family re-confirming it. `--retry-absent` or `ELFO_RETRY_ABSENT=1`
ignores the list, which is what you set the day you change the corrector, the seed
solver, or the force model. Converging a listed family retracts its entry
automatically, so a conquest cannot be undone by a stale skip.

`--write-seeds` updates the cache from a run. It writes only records whose *numbers*
changed, not every record whose `generated_by` moved, so a regeneration produces a
diff of the seeds that actually moved.

The retry at `4M` on the warm path is load-bearing rather than symmetric-for-its-own-
sake: N = 70 was *found* at `4M`, and since the warm start re-propagates the node set
from `state0` rather than storing it, it needs the resolution that found the orbit in
order to close it again. Without that retry N = 70 would be banked and then silently
unusable.

## The campaign: what actually failed, and why

`crates/elfo-catalog/examples/campaign.rs` is the driver. Four techniques were tried
against N = 45 (full model) before any of them was applied at scale.

### (a) Kepler rescale from a converged neighbour — *worse than the analytic seed*

Take the converged N = 40 first member, scale it to N = 45's semi-major axis
(`r × s`, `v_inertial × s^(-1/2)`, `s = a₄₅/a₄₀ = 0.9262`), re-anchor on `y = 0`, and
correct. Measured:

| seed | defect \|R\|∞ at m = 90 | outcome |
|---|---|---|
| analytic (`E_FRACTION = 0.64`) | 2.700e-3 | stalled 1.81e-4 |
| rescaled from converged 40 | 1.124e-2 | stalled 3.16e-3 |
| rescaled 40 → 41 (one ladder rung) | 9.307e-3 | stalled 3.98e-3 |

The rescale makes the seed **4× worse**, and a finer ladder does not help because
every rung starts from the same kind of state. The reason is that the rescale is a
similarity transformation of the *two-body* problem: it preserves `e` and `i`
exactly, which is precisely wrong here, because the frozen geometry that closes at
N = 45 has a different `e` from the one that closes at N = 40. Technique (a) moves
the seed to the right altitude by moving it *off* the frozen curve.

**Note on the brief's period rule.** The instruction was to scale the period by
`s^(3/2)`. That is the right law for an orbital period and the wrong object here: the
period being corrected is the *node-regression closure* period (≈ 2π per closure at
every one of these altitudes), which moves by well under 1 % across the whole sweep,
while `s^(3/2)` would move it by 11 %. The campaign takes the state from the rescale
and the period from the analytic solver, which measures it directly.

### (b) Stepwise resonance ladder — *inherits (a)'s defect*

`40 → 42 → 45` and `40 → 41 → 42 → 43 → 44 → 45` both die on the first rung (3.98e-3
at 41), for the reason above. The ladder is implemented and works mechanically; it
has nothing to bridge.

### (c) Levenberg–Marquardt damping — *real progress, no convergence*

Implemented in the campaign example on top of elfo-core's public `build_system`,
deliberately outside the shipped `correct()`. It damps with `λ·diag(JᵀJ)` (Marquardt,
not `λ·I`: the state columns carry STM entries up to 1e4 while the period column is
`f_end/m`, six orders smaller, so a scalar λ freezes the period), accepts on the
2-norm it actually minimises, and hands off to the shipped corrector once under 1e-5.

It is far stronger than the shipped line search — from a 2.7e-3 seed it reaches
**1.3e-6**, where `correct()` gives up at 1.8e-4 — and it then crawls: ~1 % per
iteration, λ pinned at 1e-7, `T` walking steadily and never arriving. 300 iterations
do not close it.

**More segments does not fix it either**: m = 135 stalls at 4.3e-4, m = 180 at
1.5e-4, m = 270 at 1.6e-3 — not even monotone.

**And it is not integration accuracy.** Rebuilding with `Dp54` at `rtol = atol =
1e-14` instead of `1e-12` doubled the runtime and reproduced the residual sequence
*to five significant figures* (2.7003e-3, 7.2579e-6, 4.6345e-6, 3.6572e-6). Whatever
the obstruction is, it is not noise.

### (d) The seed's eccentricity — **this is the whole thing**

`seeds.rs` picks `e` from a periapsis-altitude budget, `E_FRACTION = 0.64`, and then
takes `i` off the Lidov–Kozai frozen relation. That constant was chosen by a measured
sweep **at N = 25**. Sliding it moves the seed *along* the frozen curve — every value
is an equally legitimate frozen geometry — and the corrector's basin turns out to sit
somewhere else entirely at high N:

```text
full N=45, cold, m=90, shipped corrector:
  E_FRACTION 0.64 (shipped):  seed defect 2.700e-3 -> stalled 1.81e-4
  E_FRACTION 0.68:            seed defect 3.136e-3 -> converged 9.76e-12 in 3.7 s
```

The winning seed has a **larger** initial defect than the losing one. This is a basin
*shape* result, not a seed-quality result, and no amount of better stepping from
inside the wrong region was ever going to find it. That is why (a), (b) and (c) all
failed and why a one-line change to a seed constant succeeded in under four seconds.

Sweeping the fraction on a 0.02 grid maps the basin, and it narrows monotonically
with N:

| combo | N | e-fractions that converge (grid 0.60–0.80) |
|---|---|---|
| full | 45 | 0.66 0.68 0.70 0.72 0.74 |
| full | 50 | 0.68 0.70 0.72 0.74 0.76 |
| full | 55 | 0.70 0.72 |
| full | 70 | *none at m = 2M* |
| no-c22 | 45 | 0.66 0.68 0.70 0.72 0.74 |
| no-c22 | 50 | 0.66 0.68 0.70 0.72 0.74 0.76 |
| no-c22 | 55 | 0.70 0.72 0.74 |
| no-j3 | 45 | 0.66 0.68 0.70 0.72 |
| no-j3 | 50 | 0.68 0.70 0.72 0.76 |
| no-j3 | 55 | 0.70 0.74 |

The window's centre drifts up with N (0.70 at 45, 0.72 at 50, 0.71 at 55) and its
width collapses from 5 grid points to 2. N = 70 closes the `m = 2M` window entirely —
and reopens at `m = 4M = 280`, where 0.74/0.76/0.78 all converge. So the two knobs
compose: the seed decides *which* basin you are in, the segment count decides how
wide it is.

`E_FRACTION` itself is deliberately **not** changed. It is the right value for the
families it was tuned on, and moving it would relocate the first member of every
already-working family and rewrite the entire catalog to rescue four. The per-family
value lives in the cache instead, which is exactly what the cache is for.

## Scoreboard

Banked seed = `seeds/{combo}/n{M}.json`. Every entry below was verified by counting
periapsis passages in `[0, T)` (the `seeds.rs` regression pattern — this catalog's
historic bug is resonance mislabelling, and the Kepler rescale in particular can
converge to a neighbouring family) and by computing both stability indices.

| combo | N | result | residual | technique | revs | ν₁ / ν₂ | a (km) | e | i (°) |
|---|---|---|---|---|---|---|---|---|---|
| full | 45 | converged | 7.52e-12 | e-frac 0.66, m = 2M | 45 ✓ | 1.0000 / 0.9643 | 6880.1 | 0.4757 | 47.77 |
| full | 50 | converged | 1.05e-11 | e-frac 0.68, m = 2M | 50 ✓ | 1.0000 / 0.9705 | 6421.5 | 0.4739 | 47.92 |
| full | 55 | converged | 2.28e-14 | e-frac 0.70, m = 2M | 55 ✓ | 1.0000 / 0.9754 | 6032.6 | 0.4702 | 48.07 |
| full | 70 | converged | 1.46e-11 | e-frac 0.76, **m = 4M** | 70 ✓ | 1.0000 / 0.9834 | 5146.5 | 0.4674 | 49.11 |
| no-c22 | 45 | converged | 6.49e-11 | e-frac 0.66, m = 2M | 45 ✓ | 1.0000 / 0.9644 | 6880.1 | 0.4753 | 47.77 |
| no-c22 | 50 | converged | 4.91e-14 | e-frac 0.66, m = 2M | 50 ✓ | 1.0000 / 0.9707 | 6421.7 | 0.4726 | 47.89 |
| no-c22 | 55 | converged | 7.63e-13 | e-frac 0.70, m = 2M | 55 ✓ | 1.0000 / 0.9751 | 6032.2 | 0.4721 | 48.14 |
| no-c22 | 70 | converged | 3.96e-11 | e-frac 0.74, **m = 4M** | 70 ✓ | 1.0000 / 0.9834 | 5146.6 | 0.4674 | 49.11 |
| no-j3 | 45 | converged | 4.67e-12 | e-frac 0.66, m = 2M | 45 ✓ | 1.0000 / 0.9649 | 6880.7 | 0.4729 | 47.67 |
| no-j3 | 50 | converged | 9.69e-14 | e-frac 0.68, m = 2M | 50 ✓ | 1.0000 / 0.9709 | 6421.9 | 0.4720 | 47.84 |
| no-j3 | 55 | converged | 3.70e-12 | e-frac 0.70, m = 2M | 55 ✓ | 1.0000 / 0.9754 | 6032.5 | 0.4704 | 48.05 |
| no-j3 | 70 | converged | 1.01e-11 | e-frac 0.74, **m = 4M** | 70 ✓ | 1.0000 / 0.9834 | 5146.7 | 0.4673 | 49.10 |

`no-earth` is out of scope: it has no converged families to neighbour, and its seed
lives on the J2/J3 near-circular branch, where the Lidov–Kozai `e`-sweep is not a
motion along the frozen curve but a different orbit entirely (the campaign refuses
`--e-fraction` without the Earth term rather than silently producing one).

### The M:2 families

Same technique, same sweep, `full` combo only (M:k with k > 1 is `full`-only by
`generates()`). Two of the four are now real families rather than a table entry:

| family | revs/node period | result | residual | technique | revs | ν₁ / ν₂ | a (km) | e | i (°) |
|---|---|---|---|---|---|---|---|---|---|
| 99:2 | 49.5 | converged | 8.02e-11 | e-frac 0.70, m = 2M = 198 | 99 ✓ | 1.0000 / 0.8766 | 6462.4 | 0.4827 | 48.17 |
| 111:2 | 55.5 | converged | 1.28e-14 | e-frac **0.710**, m = 400 | 111 ✓ | 1.0000 / 0.9037 | 5996.6 | 0.4710 | 48.13 |
| 149:2 | 74.5 | **failed** | best 2.45e-3 | 16 attempts: e-frac 0.66–0.78 × {2M = 298, 400}, plus m = 596 | — | — | — | — | — |
| 173:2 | 86.5 | **failed** | best 8.57e-3 | 14 attempts: e-frac 0.66–0.78 × {2M = 346, 400} | — | — | — | — | — |

**99:2** is the "cleanest double half-integer" of the lit review's D-table (D-12), and
**111:2** is the true half-integer of the 12-hour band (D-10). Both were verified the
same way as the `k = 1` conquests, and the rev count is the assertion that matters
most here: an `M:2` solve that silently targeted a single node sweep would return the
`(M±1)/2 : 1` family at half the period, and every other check would accept it. 99
and 111 periapsis passages in `[0, T)`, with `T ≈ 12.29` and `12.32` — two closure
periods, not one — is what rules that out.

The 111:2 window is **0.710 and nothing else**: 0.705 and 0.715 both stall (1.1e-4 and
2.4e-4 at m = 400). The basin has narrowed from five grid points at N = 45 to under
0.005 in the seed's eccentricity fraction. Finding it needed a 0.005 grid, and there
is no reason to think a 0.001 grid would not find more.

**149:2 and 173:2 are unresolved, and the obvious explanation is wrong.** The
tempting story is resolution. Every family conquered here suggests high
revs-per-node-period needs ~4 shooting nodes per rev — N = 70 failed at 2 nodes/rev
for all nine seed values and converged at 4 — and `MAX_SEGMENTS = 400` lines the four
`M:2` entries up exactly in the order they fell:

| family | revs | 4 nodes/rev would need | m = 400 gives | outcome |
|---|---|---|---|---|
| 99:2 | 99 | 396 | 4.04/rev | converged |
| 111:2 | 111 | 444 | 3.60/rev | converged |
| 149:2 | 149 | 596 | 2.68/rev | failed |
| 173:2 | 173 | 692 | 2.31/rev | failed |

**That hypothesis was tested and it is false.** Running 149:2 at m = 596 — a full 4
nodes/rev, above the cap, 330–545 s per attempt — makes it *worse*, not better:

```text
149:2, e-frac 0.70: m=298 -> 5.26e-3 | m=400 -> 8.60e-3 | m=596 -> 1.28e-2
149:2, e-frac 0.71:                                     | m=596 -> 1.04e-2
```

Which is the same non-monotone-in-`m` behaviour already measured on N = 45 (m = 135
→ 4.3e-4, m = 180 → 1.5e-4, m = 270 → 1.6e-3). Segment count is not a knob that
turns one way.

So the honest state of 149:2 and 173:2 is: **not diagnosed**. What is known is that
their best residuals (2.4e-3, 8.6e-3) are two orders of magnitude worse than 111:2's
near-miss at the same cap (4.8e-5), so they are not sitting just outside a seed
window the way 111:2 was — 111:2 was found by refining the grid from 0.02 to 0.005,
and there is no evidence that doing the same here would find anything. Raising
`MAX_SEGMENTS` is specifically **not** the recommended next step, because the one
experiment that would have justified it says no.

## Working on this next

In rough order of expected value per hour:

1. **Find out what the ~1e-3 obstruction actually is.** It is now the one thing
   standing between this catalog and its remaining families, it is not integration
   accuracy (measured), not segment count (measured), and not the line search
   (Levenberg–Marquardt gets three orders further and still stops). It behaves like a
   genuine nonzero local minimum of ‖R‖, which would be worth confirming directly —
   e.g. by checking whether the Jacobian's smallest singular values collapse there,
   or whether the residual concentrates on a fixed set of segments.
2. **Make the seed's eccentricity fraction a solved quantity rather than a constant.**
   Every conquest here is the same one-dimensional search, run by hand. `seeds.rs`
   could sweep the fraction itself when the first correction fails — the sweep is
   embarrassingly parallel and each probe is one corrector run — which would turn the
   campaign into a generator feature and make `absent.json` mean something much
   stronger than it does today.
3. **`no-earth` has no families at all** and never has. Its stalls are all at 1e-6-ish
   (see `seeds/no-earth/absent.json`), which is the *same* signature as the damped
   corrector's plateau on N = 45 — suggesting a shared, still-undiagnosed obstruction
   rather than thirteen independent basin failures.

## Not done here

- **`no-earth` is still empty.** Unchanged by this work, and now *recorded* rather
  than merely observed: `seeds/no-earth/absent.json` carries all thirteen stalls with
  the corrector's own message, so a future run skips them in milliseconds instead of
  re-deriving the same thirteen failures.
- **The web app is not updated.** `catalog.json` gains a `closures` field per family;
  `types.ts` validates only `resonance_n`, so an unmodified web build loads the new
  catalog and displays an M:2 family as if it were `N = M` — wrong-looking (149 revs
  labelled where 74.5 revs/month is meant) but not broken. Labels, the family strip
  ordering, and the "revs" readout all need a `k`-aware pass.
- **The richer resonance record** the lit review recommends (§6: synodic/draconic
  clocks, per-label phase residuals, a `dual_resonance` flag) is not implemented.
  `FamilyOut` carries the sidereal `M:k` and nothing else.
- **Only k ≤ 2 is configured**, though nothing in the code is k = 2 specific; the
  q = 3–4 candidates in the lit review would work by adding strings to the list.
GENERATOR FOLLOW-UP (post-campaign): near-degenerate resonant families (nu1~1) make pseudo-arclength tangent unstable -> member sequence zigzags in hp/e/E (data: N=22 35 folds, N=60 38 folds; N=25 clean). Fix: natural-parameter continuation in e for these families (fix e per step), or tangent projection maximizing d(hp) progress + near-duplicate thinning. Display-side hp-sort is the interim fix (web round dispatched 2026-08-12).
