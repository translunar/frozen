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

## Not done here

- **The catalog is not regenerated.** `web/public/catalog/` is untouched.
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
