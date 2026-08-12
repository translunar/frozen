# Literature review: dual-resonance (sidereal + synodic) ELFO families

Prepared for the ELFO Family Browser catalog. Date: 2026-08-12.

**Epistemic labeling used throughout:**
- `[VERIFIED]` — a number or claim read directly out of a cited source.
- `[INFERRED]` — my own arithmetic or reasoning, derived from verified inputs.
- `[UNVERIFIED]` — appears in a search snippet / secondary aggregation but I could not open the primary source.
- `[REFUTED]` — the source contradicts the hypothesis.

---

## 0. Executive verdict

**The half-integer (M:2) hypothesis is not supported for the JAXA and ESA constellation
families as published.** Those families are sized to **exact submultiples of the mean solar
day** (12 h / 24 h / 30 h), for ground-segment and user-scheduling convenience — not to a
commensurability with either lunar month. `[VERIFIED for ESA; INFERRED for NASA/JAXA]`

Two important corrections to the working premise:

1. **JAXA's LNSS ELFO is not a 12-hour orbit.** Its published semi-major axis
   (a = 6541.4 km) implies **T = 13.1875 h**, and JAXA's own conference material describes
   the baseline as "about 13 hours". The "12-hour ELFO (LNSS)" label used in the Iiyama &
   Gao ephemeris paper is a coarse bin label, not the design period. `[VERIFIED + INFERRED]`
2. **ESA has a 24-hour NAV family and a 12-hour COM family** (not "12- and 24-hour NAV
   families"), plus a separate ~10 h Lunar Pathfinder relay orbit. `[VERIFIED]`

**However**, the underlying intuition is *productive*: a genuine half-integer / M:2 family
does exist in the recent literature — the **ERGO** ("Elliptical Repeat Ground-track Orbit")
constellation of Zhang et al. (2025), whose published `a = 4996.6 km` sits within **0.024%**
of an exact **149:2 sidereal** commensurability (74.5 revolutions per sidereal month; closure
after **two** sidereal months). It is *simultaneously* within **0.005%** of **161:2 synodic**.
That is exactly the dual half-integer structure hypothesized — just found in a different
family than expected. `[INFERRED from VERIFIED elements]`

---

## 1. Reference constants and conventions

| Quantity | Value | Note |
|---|---|---|
| Lunar sidereal month `T_sid` | 27.321661 d = 655.719864 h | `[VERIFIED]` — value explicitly used by Iiyama & Gao for almanac Fourier terms |
| Lunar synodic month `T_syn` | 29.530589 d = 708.734136 h | standard |
| Draconic month `T_dra` | 27.212221 d | standard |
| Anomalistic month `T_ano` | 27.554550 d | standard |
| `T_syn / T_sid` | **1.080848964** | `[INFERRED]` |
| Moon GM | 4902.800118 km³/s² | GRGM/DE440 |
| Moon radius | 1737.4 km | |

**Which clock is which** (relevant to how the catalog should label things) `[INFERRED, standard dynamics]`:

- The **Earth–Moon rotating frame** (CR3BP synodic frame) rotates at the **sidereal** rate.
  Closure of a periodic orbit in that frame ⇒ commensurability with `T_sid`.
- The **Moon's body-fixed frame** also rotates at the sidereal rate (synchronous rotation),
  so **ground-track repeat ⇒ sidereal** commensurability too. These are the same clock.
- The **Sun's apparent motion relative to the Earth–Moon line** has period `T_syn`.
  Solar-perturbation periodicity, eclipse/illumination geometry, and "Sun-synchronous"
  conditions ⇒ **synodic** commensurability.
- **Node-related** repeat conditions properly use the **draconic** month; **apse-related**
  conditions use the **anomalistic** month. A rigorous catalog needs all four denominators
  available, because real ELFOs precess.

Keplerian period ↔ semi-major axis (used for every derived number below) `[INFERRED]`:

| T | a |
|---|---|
| 6.000 h | 3869.58 km |
| 8.8016 h | 4995.79 km |
| 10.000 h | 5439.55 km |
| 12.000 h | 6142.58 km |
| 13.1875 h | 6541.40 km |
| 24.000 h | 9750.73 km |
| 30.000 h | 11314.72 km |

---

## 2. Per-family parameters

### 2.1 JAXA — Lunar Navigation Satellite System (LNSS), operational ELFO

| Element | Value | Status |
|---|---|---|
| a | **6541.4 km** | `[VERIFIED]` Zhang et al. 2025, Table 1 |
| e | **0.6000** | `[VERIFIED]` |
| i | **56.200°** | `[VERIFIED]` (see conflict note) |
| ω | **90°** (apolune over south pole) | `[VERIFIED]` |
| Ω | 0° and 180° (two planes) | `[VERIFIED]` |
| ν | 0°, 90°, 180°, 270° per plane (4 sats × 2 planes = 8) | `[VERIFIED]` |
| **Period** | **13.1875 h** | `[INFERRED]` from a; corroborated by ION abstract "orbital period was about 13 hours" `[VERIFIED]` |
| Constellation | 8 navigation satellites in 2 ELFO planes (8–10 in some sources) | `[VERIFIED]` |
| Resonance claim in source | **none** | `[VERIFIED — no resonance/repeat statement in the source]` |

**Conflict note** `[UNVERIFIED]`: a search-index snippet reports "LNSS (ELFO) with 12-hour
period: a = 6541.40 km, e = 0.600, i = 62.940°". The semi-major axis matches Zhang et al.
but the inclination and the "12-hour" label do not; a = 6541.40 km cannot be a 12-hour orbit
(12 h ⇒ 6142.58 km). Treat i as uncertain in the range **56°–63°** and the period as
**13.19 h**, not 12 h.

**Demonstration satellite** (a different orbit, launching 2029) `[VERIFIED]`, from the LANS
Interoperability Demonstration paper, Table 3:
a = 3870.00 km, e = 0 (circular), i = 104.428°, ω = 90.0°, Ω = 53.563°, ν = −5.0°,
epoch 2027-01-01 00:00:00.000 TDB, ICRF. ⇒ **T = 6.0010 h**, near-circular polar `[INFERRED]`.
This is the "6-hour polar orbit" in the Iiyama & Gao study.

**Resonance arithmetic** `[INFERRED]` for T = 13.1875 h:
- revs / sidereal month = **49.7230** → nearest low-order: 199:4 = 49.75 (err 0.054%),
  149:3 = 49.667 (0.113%), 348:7 = 49.714 (0.012%). **No half-integer.**
- revs / synodic month = **53.7430** → 215:4 = 53.75 (err 0.013%), 161:3 (0.142%).
- Interesting near-miss: the exact **double half-integer** point 99:2 sidereal (49.5) /
  107:2 synodic (53.5) lies at **T = 13.2469 h, a = 6561.03 km** — only **19.6 km (0.30%)
  above JAXA's published a**. See §5, candidate D-4.

### 2.2 ESA — Moonlight / LCNS navigation satellites (24-hour ELFO)

Two independently published parameter sets exist; both are 24 h.

**Set A** — Moonlight navigation service design paper `[VERIFIED]`:

| Element | Value |
|---|---|
| a | **9750.7 km** — "set to ensure a period of 24 hours" (direct quote) |
| e | **0.7** |
| i | **63.2°** |
| ω | **90°** — "fixed to 90° such that the aposelene lies above the South Pole" |
| Ω / ν | Sat 1: 0°/0°; Sat 2: 120°/164°; Sat 3: 240°/196° |
| Rationale | RAAN/TA optimized to improve lunar-surface GDOP |

**Set B** — LANS Interoperability Demonstration paper, Table 3, "ESA Moonlight LCNS Nav #1"
`[VERIFIED]`: a = 9748.14 km, e = 0.70, i = 48.04°, ω = 123.60°, Ω = 89.49°, ν = 90.0°,
ICRF, epoch 2027-01-01 00:00:00.000 TDB. (Marked "notional" in the source.) ⇒ T = 23.9904 h.

A third snippet `[UNVERIFIED]` gives i = 50.638°, ω = 94.344° for a = 9748.14 km.
The i/ω spread (48°–63° / 90°–124°) is almost certainly osculating-vs-mean elements at
different epochs, not different families. **a ≈ 9748–9751 km and e = 0.70 are stable
across all sources.**

**ESA constellation composition** `[VERIFIED]`: LCNS = 5 satellites — **4 navigation in
24-hour ELFOs** plus **1 communication satellite in a 12-hour orbit, a ≈ 6000 km**.
IOC 2029 (1 NAV + 1 COM), FOC 2030 (+3 NAV). Target: ≥15 h/day of PVT at the south pole.

**Resonance claim in source**: **none.** The only stated design drivers are (i) exactly
24 h period, (ii) ω = 90° for south-polar apolune, (iii) frozen e/i/ω, (iv) GDOP optimization
of RAAN/TA. `[VERIFIED]`

**Resonance arithmetic** `[INFERRED]` for T = 24.000 h:
- revs / sidereal month = **27.3217** → 82:3 = 27.3333 (err 0.042%); 109:4 = 27.25 (0.263%);
  55:2 = 27.5 (0.65%). **Half-integer sidereal is a poor fit.**
- revs / synodic month = **29.5306** → **59:2 = 29.5 (err 0.104%)**. This *is* a half-integer,
  but see §4 — it is a triviality, not a design choice.

### 2.3 ESA / SSTL — Lunar Pathfinder (relay precursor)

`[VERIFIED]` from the ESA Moonlight LP & LCNS briefing:
- Perilune altitude **673.4 km**, apolune altitude **7331.8 km**, i = **46.8°**,
  stated orbital period **10 h**.

`[INFERRED]` **The source is internally inconsistent.** With R_moon = 1737.4 km those
altitudes give a = 5740.0 km, e = 0.5800 ⇒ **T = 10.84 h**, not 10 h. A literal 10.00 h
orbit requires a = 5439.6 km. Do not treat "10 h" and the altitudes as mutually consistent.
- If a = 5740.0 km: revs/sidereal month = 60.4916 → **121:2 = 60.5, err 0.014%** — a
  tantalizing half-integer, but resting on an inconsistent source. `[UNVERIFIED]`
- If T = 10.000 h exactly: revs/sidereal month = 65.5720 → 459:7 (0.001%), 328:5 (0.043%).
  No low-order match.

Neither eoPortal nor the SSTL user manual states e, ω, or any repeat/resonance condition.

### 2.4 NASA — LCRNS (30-hour ELFO), for completeness

`[VERIFIED]` NASA LCRNS Reference Constellation 3.1 (Aug 2025), epoch 2027-03-01 00:00:00 UTC:
a = **11315.94 km**, e = **0.692**, i = **59.373°**, Ω = **321.019°**, ω = **92.494°**, ν = 0°.
Rationale quoted: ELFO "selected to provide longer dwell times with apolune above the south
pole"; uses "Earth's perturbations to provide stability in the orbit Argument of Perilune
(AOP) and Eccentricity." **No resonance statement.**

`[VERIFIED]` LANS Interop paper Table 3 gives three *notional* LCRNS nodes at
a = 11999.2626 / 12027.7960 / 11993.3508 km, e = 0.655 / 0.641 / 0.721,
i = 32.22° / 31.33° / 79.07°, ω = 75.96° / 76.14° / 68.18° — i.e. ~32.8 h orbits, not 30 h.
The official 3.1 reference constellation supersedes these.

`[INFERRED]` T = 30.0048 h ⇒ revs/sidereal = 21.8538 (nearest 131:6, 0.094%);
revs/synodic = 23.6207 (nearest 118:5, 0.087%). No low-order commensurability.

### 2.5 Stanford LNCSS — the actual, literal 12-hour ELFO

`[VERIFIED]` Bhamidipati, Mina, Sanchez & Gao (2023):
a = **6143 km**, e = **0.6**, i = **51.7°**, ω = **90°**, perigee alt 720 km,
apogee alt 8090 km, **T = 12 h exactly**. Cases A/B/C = 8/12/16 satellites,
RAAN ∈ {0°, 180°}, mean anomaly ∈ {0°, 90°, 180°, 270°}.

This is the most-cited literal 12-h ELFO and is very likely the source of the "12-hour ELFO"
bin label. **Crucially**, the paper describes the design as a *flower constellation*, "a
special family of J2-frozen repeat ground track constellations wherein the orbital parameters
are selected such that the nodal period of the orbit matches the nodal period of the primary
body" — but it **does not give the commensurability integers** and **never mentions the
synodic month or any solar resonance**. `[VERIFIED]` The only period statement made is
"one orbital period is 12 h … equivalent to 30 full revolutions … over 15 simulation days"
— i.e. the reference clock is the **Earth day**.

Baweja (2026) reuses essentially the same orbit: a = 6142 km, e = 0.6, i = 57.7°, ω = 90°.
`[VERIFIED]`

### 2.6 ERGO — the family that *does* carry a half-integer structure

`[VERIFIED]` Zhang et al. (2025), Scientific Reports 15:35809, Table 1, "Elliptical Repeat
Ground-track Orbit" LNSS variant:
a = **4996.6 km**, e = **0.5384**, i = **52.207°**, ω = **90°**,
Ω = 30° and 120° (two planes), ν = 45°, 135°, 225°, 315° per plane (8 satellites).

`[VERIFIED]` The paper compares ELFO (8 sat, 69% P(NVS≥4), 52% P(PDOP<10)) with
ERGO (8 sat, 64%, 60%), and hybrid ELFO+NRHO (9 sat, 96%, 73%). It states ERGO needs
"no orbital maintenance … for at least a decade" but **does not publish the repeat integers**.

`[INFERRED]` **Reverse-engineering the repeat condition** — this is the key finding:

```
a = 4996.6 km  ⇒  T = 8.80372 h
revs / sidereal month = 74.4820
    nearest:  149:2 = 74.5000   err 0.024%    ← HALF-INTEGER, closes after 2 sidereal months
              223:3 = 74.3333   err 0.199%
               74:1 = 74.0000   err 0.647%
revs / synodic  month = 80.5038
    nearest:  161:2 = 80.5000   err 0.005%    ← HALF-INTEGER, closes after 2 synodic months
```

Exact 149:2 sidereal ⇒ T = 8.80158 h ⇒ a = **4995.79 km**, i.e. **0.81 km (0.016%)** from
the published value. Given that (a) the published element is presumably a *mean* element,
and (b) a true repeat condition uses the **draconic** period plus nodal/apsidal drift rather
than the Keplerian period, an 0.02% offset is exactly the size one expects. **I assess with
high confidence that ERGO is a 149:2 sidereal repeat orbit** — an odd rev count over two
sidereal months, i.e. precisely the M:2 / half-integer structure hypothesized.

That it is *also* within 0.005% of 161:2 synodic is a **dual half-integer resonance**.
Whether that was designed or is a by-product I cannot determine from the source
(`149/161 = 0.92547` vs `T_sid/T_syn = 0.92520`, agreement 0.029% — the two conditions are
nearly, but not exactly, the same condition).

---

## 3. The dual-resonance concept in the literature

**What I could NOT find:** no paper explicitly designs an Earth–Moon ELFO to be
*simultaneously* resonant with the sidereal and synodic months. Searches on
"resonant elliptical lunar frozen orbit", "2:1 synodic resonance lunar orbit",
"semi-synodic", "lunar navigation constellation frozen orbit resonance" returned nothing
that imposes both conditions. `[VERIFIED — negative result]`

**What exists, and is adjacent:**

1. **Sun-synchronous ELFOs** — Wang/Zhang et al., *"Design of sun-synchronous and repeating
   tracking condition elliptical lunar frozen orbits"* (Acta Aeronautica et Astronautica
   Sinica, 2023). Uses von Zeipel canonical transformation to average first- and second-order
   Earth third-body plus lunar J2 terms, then imposes (i) frozen conditions, (ii) a
   **Sun-synchronous** condition, and (iii) a **repeating-tracking** condition. This is the
   closest published thing to a genuine dual condition: Sun-synchrony is a *synodic-clock*
   constraint and track repeat is a *sidereal-clock* constraint. `[VERIFIED that these three
   conditions are imposed; the specific commensurability integers are behind a paywall —
   UNVERIFIED]` **This is the single highest-value paper to obtain in full.**

2. **Flower constellations / repeat-ground-track lunar orbits** — Bhamidipati et al. (2023)
   use the framework (`ν = N_P / N_D`) but do not publish integers for the lunar case.
   Russell & Lara previously found long-lifetime lunar RGT orbits in the Earth–Moon RTBP
   with a high-resolution lunar field. `[VERIFIED, general]`

3. **The torus / frequency view, and an explicit counter-argument** — Park, Howell & Brack,
   *"Elliptical Lunar Frozen Orbit Constellations: Torus-Based Design and Analysis"*
   (arXiv:2608.10417, Aug 2026), and Park, Howell & Stewart (AAS 2025). Each ELFO is
   characterized by **three** frequency–angle pairs: `(ν_S, θ_S)` orbital revolution (~1 day),
   `(ν_M, θ_M)` nodal precession in the rotating frame (~1 month), `(ν_L, θ_L)` libration
   about the frozen equilibrium (months to years). Reference config: a = 14200 km, ω = 90°,
   with (e, i) = (0.5707, 50.5°) without lunar-asymmetry terms and (0.6507, 46.5°) with
   them; `T_M ≈ 25.2 d` in the first case. **The paper states these frequencies are generically
   *incommensurate*, so ELFOs live on 2-D quasi-periodic tori rather than as resonant
   periodic orbits.** `[VERIFIED]` Their design space is `(θ_S, θ_M) ∈ [0,2π)²` with time
   evolution along slope `dθ_M/dθ_S = ν_M/ν_S`. They *do* note that "the parity of the
   resonance ratio" is exploited to initialize admissible apse configurations `[UNVERIFIED —
   from a secondary summary]`, which is the closest hook in that literature to an
   even/odd (M:1 vs M:2) distinction.

4. **Baseline frozen-orbit condition** used across the LANS/ELFO literature `[VERIFIED]`:
   `ω = π/2` and `e² + (5/3)cos²i = 1`, i.e. `e = sqrt(1 − (5/3)cos²i)`; the circular limit
   gives the critical inclination `i = 39.23°`. Trade spaces run a ∈ [4000, 16000] km,
   i ∈ [40°, 65°] for ELFO. Note that none of the agency orbits sit exactly on this curve
   (e.g. e = 0.7 ⇒ i = 56.4°, vs published 48°–63°), because lunar J2/C22 and higher-order
   terms shift the equilibrium — see Matsumoto (AIAA 2024-1450), which JAXA cites as the
   basis for its ELFO selection.

5. **Spectral evidence for the sidereal clock** — Iiyama & Gao `[VERIFIED]`: "A strong
   periodic component near **half the lunar sidereal period (≈13.7 days)** persists in the
   inclination spectrum, again indicative of third-body coupling", and the same ≈13.66 d
   component appears in e, i, Ω, ω. Their almanac model is built on a single Fourier term at
   `T_sid = 27.321661 days`. **The dominant perturbation clock in these orbits is
   sidereal/2, not synodic.** This is the strongest published argument for making the
   sidereal month the catalog's primary denominator — and, notably, `T_sid/2` naturally
   produces half-integer rev counts.

---

## 4. Verdict on the half-integer question, with arithmetic

### 4.1 The literal-period arithmetic

For a **literal 12.000 h** orbit (a = 6142.58 km) `[INFERRED]`:

```
revs / sidereal month = 655.719864 / 12 = 54.6433
   109:2 = 54.5000   err 0.259%   ← the hypothesized M:2
   164:3 = 54.6667   err 0.043%
   437:8 = 54.6250   err 0.034%
    55:1 = 55.0000   err 0.653%
revs / synodic  month = 708.734136 / 12 = 59.0612
    59:1 = 59.0000   err 0.104%
```

For a **literal 24.000 h** orbit (a = 9750.73 km) `[INFERRED]`:

```
revs / sidereal month = 27.3217
    82:3 = 27.3333   err 0.042%
   109:4 = 27.2500   err 0.263%
    55:2 = 27.5000   err 0.652%   ← the hypothesized M:2
revs / synodic  month = 29.5306
    59:2 = 29.5000   err 0.104%
```

**Is 109:2 a closure?** No. At 12.000 h the orbit performs 109.2866 revolutions in two
sidereal months. The residual, 0.2866 rev, is **103° of mean anomaly** — the orbit is nowhere
near the same point in the rotating frame. To close at 109:2 you would need T = 12.0316 h
(a = 6153.34 km), which is 11 km off the 12-h value and inconsistent with every published
element set. **`[REFUTED]` The 12-h family is not an M:2 sidereal resonance.**

Same for 24 h at 55:2 (0.65% ⇒ 0.18 rev = 64° residual after two sidereal months).
**`[REFUTED]`**

### 4.2 What the 59:2 synodic "half-integer" actually is

24 h ↔ 59:2 synodic is real to 0.104% — but it is **arithmetically trivial** and carries no
design content `[INFERRED]`: a 24-hour orbit performs exactly one revolution per mean solar
day, so "revs per synodic month" ≡ "days per synodic month" = 29.5306. The statement
"59 revolutions in two synodic months" is nothing more than the Babylonian
**59 days ≈ 2 synodic months** near-commensurability. Likewise the 12-h orbit's 59:1 synodic
is the same relation at half-days. Residual after two synodic months: 0.061 rev = **22°** —
better than the sidereal case, but still not a closure, and not designed.

### 4.3 What actually sets the semi-major axes

`[VERIFIED]` ESA states outright that a = 9750.7 km was **"set to ensure a period of 24
hours."** `[INFERRED]` My Keplerian solve for T = 24.000 h gives **a = 9750.73 km** — agreement
to 0.03 km. Similarly T = 30.000 h ⇒ 11314.72 km vs NASA's published 11315.94 km (0.01%),
and T = 6.000 h ⇒ 3869.58 km vs JAXA's demo 3870.00 km (0.01%).

**Conclusion: the agency families are locked to the mean solar day (1/2, 1, 1¼ day), not to
any lunar month.** The driver is ground-segment scheduling, contact planning, and the
"hours per day of PVT availability" service metric ESA quotes. JAXA's 13.19 h ELFO is the
one exception — it is *not* an Earth-day submultiple, and appears to come from a
frozen-orbit + coverage optimization (Matsumoto's J2-aware ELFO design), with no published
resonance rationale.

### 4.4 Where the half-integer structure *is* real

`[INFERRED]` **ERGO, a = 4996.6 km, T = 8.8037 h: 149:2 sidereal (0.024%) and 161:2 synodic
(0.005%).** This is a genuine odd-rev-count-per-two-months family, and it comes from a
paper that explicitly names the family "Elliptical **Repeat Ground-track** Orbit". The
half-integer arises naturally: a repeat-ground-track orbit whose track pattern only closes
after two lunar rotations gives twice the track density of an M:1 orbit for the same
altitude — a standard RGT design move.

**Recommendation: the tool must support M:2 (and higher-q) closures. The hypothesis is
right about the *phenomenon*; it is attached to the wrong families.**

---

## 5. Candidate dual-resonance rev counts for the catalog

The dual condition is `x_sid = p/q` and `x_syn = r/s` for the *same* period, which requires
`(r/s)/(p/q) ≈ T_syn/T_sid = 1.080848964`. Continued-fraction convergents `[INFERRED]`:

| Convergent | Value | Error | Meaning |
|---|---|---|---|
| 13/12 | 1.0833333 | 0.2299% | 13 sidereal ≈ 12 synodic months (~1 yr) |
| 27/25 | 1.0800000 | 0.0785% | |
| **40/37** | 1.0810811 | **0.0215%** | 40 sidereal ≈ 37 synodic months (~3 yr) — the useful low-order one |
| 107/99 | 1.0808081 | 0.0038% | |
| **254/235** | 1.0808511 | **0.0002%** | **the Metonic relation** (19 yr) — exact for practical purposes but useless as a design denominator |

Note this means **the 13.5-sidereal : 12.5-synodic guess in the brief is not the right
near-commensurability**; the relevant small-integer relation is **40:37**, and its
half-integer refinement **149:161** (which is what ERGO sits on).

### Candidate table (denominators ≤ 2, periods 6–32 h) `[INFERRED]`

Sorted by period. `err` is the synodic-side residual once the sidereal side is exact.

| # | T (h) | a (km) | sidereal p:q | revs/sid | synodic r:s | revs/syn | err | Notes |
|---|---|---|---|---|---|---|---|---|
| D-1 | 6.6234 | 4133.19 | 99:1 | 99.0 | 107:1 | 107.0 | 0.0038% | best pure-integer dual; the 107/99 convergent |
| D-2 | 7.0889 | 4324.62 | **185:2** | 92.5 | 100:1 | 100.0 | 0.0215% | half-int sidereal, int synodic |
| D-3 | 7.5806 | 4522.35 | **173:2** | 86.5 | **187:2** | 93.5 | 0.0070% | **double half-integer**, excellent |
| D-4 | 8.1456 | 4744.37 | **161:2** | 80.5 | 87:1 | 87.0 | 0.0096% | |
| **D-5** | **8.8016** | **4995.79** | **149:2** | **74.5** | **161:2** | **80.5** | 0.0289% | **= published ERGO (a = 4996.6 km)** |
| D-6 | 8.8611 | 5018.27 | 74:1 | 74.0 | 80:1 | 80.0 | 0.0215% | the 40/37 convergent ×2 |
| D-7 | 9.6429 | 5309.29 | 68:1 | 68.0 | **147:2** | 73.5 | 0.0031% | |
| D-8 | 10.5761 | 5646.52 | 62:1 | 62.0 | 67:1 | 67.0 | 0.0189% | |
| D-9 | 10.6621 | 5677.08 | **123:2** | 61.5 | **133:2** | 66.5 | 0.0418% | double half-integer |
| **D-10** | **11.8148** | **6079.20** | **111:2** | **55.5** | **60:1** | **60.0** | 0.0215% | **the true "12-hour-band" half-integer** — 63 km below the 12.000 h value |
| D-11 | 11.7093 | 6042.96 | 56:1 | 56.0 | **121:2** | 60.5 | 0.0455% | alternative in the 12-h band |
| **D-12** | **13.2469** | **6561.03** | **99:2** | **49.5** | **107:2** | **53.5** | **0.0038%** | **double half-integer, 0.30% from JAXA's published a = 6541.4 km** |
| D-13 | 15.0740 | 7151.26 | **87:2** | 43.5 | 47:1 | 47.0 | 0.0360% | |
| D-14 | 17.7222 | 7966.01 | 37:1 | 37.0 | 40:1 | 40.0 | 0.0215% | the 40/37 convergent itself |
| D-15 | 21.1523 | 8963.29 | 31:1 | 31.0 | **67:2** | 33.5 | 0.0189% | **best dual near the 24-h band** |
| D-16 | 26.2288 | 10345.43 | 25:1 | 25.0 | 27:1 | 27.0 | 0.0785% | weak |
| D-17 | 35.4443 | 12645.25 | **37:2** | 18.5 | 20:1 | 20.0 | 0.0215% | just outside the 30-h band |

**Notable structural facts** `[INFERRED]`:
- There is **no good dual resonance anywhere near 24 h**. The nearest respectable one is
  D-15 at 21.15 h (a = 8963 km), 12% below ESA's a. So a dual-resonant "24-hour-class" ESA
  variant does not exist without moving the period substantially.
- The **12-h band does have one**: D-10 at 11.8148 h (a = 6079.20 km) is 55.5 revs per
  sidereal month = **111 revs per two sidereal months** — a true M:2 half-integer — and
  simultaneously exactly 60 revs per synodic month. It is only **63 km (1.0%)** below the
  literal 12-h semi-major axis. **This is the orbit the hypothesis was reaching for.**
- The **13-h band's** D-12 (99:2 sidereal, 107:2 synodic; a = 6561.03 km) is the single
  cleanest double-half-integer in the whole ELFO altitude range (0.0038%) and is within
  0.30% of JAXA's actual a. Worth adding as a "JAXA-adjacent, resonance-tuned" catalog entry.

### Higher-denominator families worth supporting
`[INFERRED]` Restricting to q ≤ 2 hides good structure. Examples at q = 3–4:
- 169:2 sidereal (84.5) with 274:3 synodic (91.333), T = 7.7600 h, a = 4593.43 km, err 0.0017%
- 103:2 sidereal (51.5) with 167:3 synodic (55.667), T = 12.7324 h, a = 6390.04 km, err 0.0053%
- 235:2 sidereal (117.5) with 127:1 synodic, T = 5.5806 h, a = 3687.09 km, err 0.0002% (best overall)

---

## 6. Implications for the catalog

`[INFERRED — design recommendations]`

1. **Do not label the ESA/NASA/JAXA constellation orbits with a lunar-month resonance.**
   Their defining property is a period commensurate with the **mean solar day**. Add a
   distinct label class, e.g. `earth_day: 1:2` (12 h), `1:1` (24 h), `5:4` (30 h),
   `1:4` (6 h). Marking these as "N ≈ 54.64 revs/sidereal" would be misleading.
   JAXA's 13.19 h orbit gets **no** day label — flag it as `unlocked`.

2. **Support rational resonance labels `p:q`, not integers.** Minimum useful denominator
   set is `q ∈ {1, 2, 3, 4}`; `q` up to 8 is cheap and captures 437:8-type entries. The
   half-integer case (`q = 2`) is *load-bearing* — the one published repeat-ground-track
   lunar constellation family (ERGO) is 149:2.

3. **Carry multiple reference clocks per family, not one.** At minimum:
   - `sidereal` (27.321661 d) — rotating-frame / ground-track closure. **Primary.**
   - `synodic` (29.530589 d) — solar perturbation, illumination, Sun-synchrony. **Secondary.**
   - `draconic` (27.212221 d) — the correct clock for node-referenced repeat conditions.
   - `anomalistic` (27.554550 d) — apse-referenced conditions.
   - `solar_day` (24 h) — because that is what the real missions actually use.
   Store a resonance record as a tuple, e.g.
   `{sidereal: 149:2, synodic: 161:2, draconic: null, solar_day: null}`.

4. **Report a residual, not just a label.** Every "resonance" here is approximate. Store,
   for each label, the fractional period error and, more usefully, the **phase residual per
   closure** in degrees of mean anomaly (`360° × q × |x − p/q|`). Suppress a label when the
   residual exceeds a threshold (~10–20°). Under such a rule, 12 h → 109:2 sidereal (103°
   residual) is correctly rejected, while ERGO → 149:2 (17° residual, and much less once the
   draconic period is used) is correctly kept.

5. **Add a `dual_resonance` flag and a derived score.** Families satisfying both a sidereal
   and a synodic condition below threshold are the interesting ones for long-term stability
   (the solar term and the ground-track pattern re-phase together). Candidates D-1 … D-17
   above are the seed list; D-3, D-5, D-10, D-12 are the standouts.

6. **Be careful about which period you use to compute `N`.** All the numbers here are
   Keplerian two-body periods from `a`. Real ELFOs at these altitudes have substantial nodal
   and apsidal drift under Earth's third-body pull, so the *nodal* (draconic) period differs
   from the Keplerian one at the 0.1–1% level — the same order as the resonance residuals
   being tested. **The catalog should compute `N` from the numerically-continued
   rotating-frame closure period of the actual periodic orbit, not from `a`.** The
   Keplerian-`a` route in this document is a screening tool only.

7. **Consider representing ELFOs as tori, not just periodic orbits.** Park, Howell & Brack
   show the natural object has three frequencies `(ν_S, ν_M, ν_L)`, generically incommensurate.
   A "family labeled by resonance N" is really a *slice* through that torus family where
   `ν_M/ν_S` is rational. Exposing `ν_M/ν_S` (and `ν_L`) as continuous quantities alongside
   the discrete resonance label would let the browser show *how far* a given member is from
   each nearby resonance — which is exactly the residual in point 4.

8. **Open item for follow-up.** Obtain the full text of the Acta Aeronautica et Astronautica
   Sinica paper on Sun-synchronous + repeating-track ELFOs (§3 item 1). It is the only
   located work that imposes a synodic-clock condition and a sidereal-clock condition
   simultaneously, and its constraint equations would give the catalog a principled
   definition of "dual resonance" rather than the numerological screening used here.

---

## 7. Confidence summary

| Claim | Status |
|---|---|
| ESA LCNS NAV = 24-h ELFO, a ≈ 9750.7 km, e = 0.7, ω = 90° | VERIFIED |
| ESA a chosen "to ensure a period of 24 hours" | VERIFIED (direct quote) |
| ESA LCNS COM satellite = 12-h, a ≈ 6000 km | VERIFIED |
| ESA LCNS inclination | VERIFIED but source-dependent (48.04° / 50.638° / 63.2°) |
| JAXA LNSS ELFO a = 6541.4 km, e = 0.6, ω = 90°, 8 sats / 2 planes | VERIFIED |
| JAXA LNSS period ≈ 13.19 h, **not** 12 h | INFERRED from a; corroborated VERIFIED ("about 13 hours") |
| JAXA LNSS inclination | UNCERTAIN, 56.2° or 62.94° |
| NASA LCRNS 30-h: a = 11315.94, e = 0.692, i = 59.373°, ω = 92.494° | VERIFIED |
| Stanford/Gao literal 12-h ELFO: a = 6143, e = 0.6, i = 51.7°, ω = 90° | VERIFIED |
| Lunar Pathfinder: peri 673.4 km, apo 7331.8 km, i = 46.8°, "10 h" | VERIFIED but internally inconsistent |
| **No agency source claims any lunar-month resonance for these orbits** | VERIFIED (negative) |
| 12-h family is an M:2 sidereal resonance (109:2) | **REFUTED** (0.26%, 103° residual) |
| 24-h family is an M:2 sidereal resonance (55:2) | **REFUTED** (0.65%) |
| 24-h ↔ 59:2 synodic (0.104%) | true but trivial (= 59 days ≈ 2 synodic months) |
| ERGO a = 4996.6 km is a 149:2 sidereal repeat | INFERRED, high confidence (0.024%) |
| ERGO is also 161:2 synodic | INFERRED (0.005%) |
| ELFO frequencies are generically incommensurate (torus, not periodic orbit) | VERIFIED |
| Dominant perturbation spectral line is at T_sid/2 ≈ 13.7 d | VERIFIED |
| Dual-resonance candidate table D-1 … D-17 | INFERRED (my arithmetic) |

---

## Sources

**Agency / mission reference orbits**
- NASA, *Lunar Communications Relay and Navigation Systems (LCRNS) Reference Constellation, rev 3.1*, Aug 2025 — https://www.nasa.gov/wp-content/uploads/2025/08/nasa-lunar-communications-relay-and-navigation-systems-lcrns-reference-constellation-3-1.pdf
- Swinden, R., et al. (NASA/ESA/JAXA), *Lunar Augmented Navigation Service (LANS) Interoperability Demonstration*, NTRS 20250009447 — https://ntrs.nasa.gov/api/citations/20250009447/downloads/LANS_Demo_ION_Paper_v1_3.pdf
- NASA, *LANS Interop Demo*, SpaceOps 2025, ID 429, NTRS 20250003163 — https://ntrs.nasa.gov/api/citations/20250003163/downloads/LANS%20Interop%20Demo-SpOps2025%20ID429%20v4Final.pdf
- ESA, *Lunar Communication & Navigation Network — Lunar Pathfinder Orbit & LCNS* (briefing slides) — https://spacefinland.fi/documents/60305973/0/ESA+-+Moonlight_LP_and_LCNS_Slides+(1).pdf/d5399da8-26d6-016b-2cda-703c8ac70f16?t=1695736353872
- ESA, *Moonlight programme* — https://www.esa.int/Applications/Connectivity_and_Secure_Communications/ESA_s_Moonlight_programme_Pioneering_the_path_for_lunar_exploration
- ESA, *ESA launches Moonlight* (press release) — https://www.esa.int/Newsroom/Press_Releases/ESA_launches_Moonlight_to_establish_lunar_communications_and_navigation_infrastructure
- ESA IDEAS, *LCNS Phase 0 — Assessment of ELFO / Lunar Navigation Satellite System Update*, 2020 (image-only PDF, not text-extractable) — https://ideas.esa.int/core/apps/IMT/UploadedFiles/00/f_462e68a907c2320bfa139943786d59be/Lunar_Navigation_Satellite_System_Update_AD_AG_MS_09_06_2020.pdf
- Murata, M. (JAXA), *Japan Lunar Navigation Satellite System (LNSS) and Its Contribution*, ESA BSGN, 2024 (image-only PDF) — https://bsgn.esa.int/wp-content/uploads/2024/02/06-Masaya_Murata_JAXA_presentation.pdf
- JAXA, *Lunar Navigation Satellite System (LNSS) and Its Demonstration Mission*, ICG-16 WG-B — https://www.unoosa.org/documents/pdf/icg/2022/ICG16/WG-B/ICG16_WG-B_03.pdf
- eoPortal, *Lunar Pathfinder* — https://www.eoportal.org/satellite-missions/lunar-pathfinder and https://www.eoportal.org/ftp/satellite-missions/l/LunarPath_200921/LunarPath.html
- SSTL, *Lunar Pathfinder User Manual* — https://irp-cdn.multiscreensite.com/19e31c60/files/uploaded/LunarPathfinder-UserManual-WebSite-v002-2.pdf
- SSTL, *Lunar Mission Services* — https://www.sstl.co.uk/what-we-do/lunar-mission-services

**Orbit design / analysis papers**
- Iiyama, K. & Gao, G., *Ephemeris and Almanac Design for Lunar Navigation Satellites*, arXiv:2510.25161 (Oct 2025) — https://arxiv.org/abs/2510.25161 · https://arxiv.org/pdf/2510.25161
- Bhamidipati, S., Mina, T., Sanchez, A. & Gao, G., *Satellite Constellation Design for a Lunar Navigation and Communication System*, NAVIGATION 70(4), navi.613, 2023 — https://navi.ion.org/content/70/4/navi.613 · https://doi.org/10.33012/navi.613
- *Trade-off Analysis for Lunar Augmented Navigation Service (LANS) Constellation Design*, arXiv:2510.16030 — https://arxiv.org/html/2510.16030v1
- Park, B., Howell, K. C. & Brack, D., *Elliptical Lunar Frozen Orbit Constellations: Torus-Based Design and Analysis*, arXiv:2608.10417 (Aug 2026) — https://arxiv.org/abs/2608.10417
- Park, B., Howell, K. C. & Stewart, S., *Elliptical Lunar Frozen Orbit Constellation Design within a Model of Evolving Fidelity*, AAS 2025 — https://engineering.purdue.edu/people/kathleen.howell.1/Publications/Conferences/2025_AAS_ParHowSte.pdf
- Zhang, Y., et al., *Analysis of LNSS satellite occlusion in the southern polar region of the Moon based on DEM*, Scientific Reports 15:35809 (2025) — https://www.nature.com/articles/s41598-025-19786-x · open-access mirror https://pmc.ncbi.nlm.nih.gov/articles/PMC12521500/
- Ely, T. A., *Stable Constellations of Frozen Elliptical Inclined Lunar Orbits*, J. Astronaut. Sci. 53(3):301, 2005 — https://link.springer.com/article/10.1007/BF03546355 · https://ui.adsabs.harvard.edu/abs/2005JAnSc..53..301E/abstract
- Ely, T. A. & Lieb, E., *Constellations of Elliptical Inclined Lunar Orbits Providing Polar and Global Coverage* — https://www.researchgate.net/publication/268016378_Coverage_and_Control_of_Constellations_of_Elliptical_Inclined_Frozen_Lunar_Orbits
- Matsumoto, Y., *Design of Elliptical Lunar Frozen Orbit Considering Lunar J2 Perturbations*, AIAA 2024-1450 (AIAA SciTech 2024) — https://arc.aiaa.org/doi/abs/10.2514/6.2024-1450
- *Design of sun-synchronous and repeating tracking condition elliptical lunar frozen orbits*, Acta Aeronautica et Astronautica Sinica (2023), doi 10.7527/S1000-6893.2023.29926 — https://www.sciopen.com/article/10.7527/S1000-6893.2023.29926
- Giordano, P., et al. (ESA), *Moonlight navigation service — how to land on peaks of eternal light* (source of the LCNS ELFO Keplerian table) — https://www.researchgate.net/publication/358266257_Moonlight_navigation_service_-_how_to_land_on_peaks_of_eternal_light · table: https://www.researchgate.net/figure/ELFO-Keplerian-parameters-of-LCNS-satellites_tbl1_358266257
- Murata, M., Kawano, I. & Kogure, S., *Lunar Navigation Satellite System and Positioning Accuracy Evaluation*, ION ITM 2022, pp. 582–586, doi 10.33012/2022.18220 — https://www.ion.org/publications/pdf.cfm?articleID=18220
- ION GNSS+ abstract, *Japan's LNSS Demonstration Mission: Advancing Lunar PNT* — https://www.ion.org/gnss/abstracts.cfm?paperID=15867
- Baweja, C., *The Cost of Lunar South-Polar Geometry, and Surface Beacons as the Efficient Fix: A Dilution-of-Precision Analysis*, arXiv:2607.06212 (Jul 2026) — https://arxiv.org/abs/2607.06212
- *Trajectory design and optimization for elliptical lunar frozen orbit mission*, Acta Astronautica (2025) — https://www.sciencedirect.com/science/article/abs/pii/S0094576525005673
- *A Novel Navigation Message for Future LCNS Satellites*, Eng. Proc. 88:52 (2025) — https://doi.org/10.3390/engproc2025088052
- *Positioning of a Lunar Lander Using a Dedicated LCNS Assuming Realistic ODTS Performances*, Eng. Proc. 88:74 (2025) — https://doi.org/10.3390/engproc2025088074
- Rawat, Kumar, Rosengren & Ross, *Cislunar Mean-Motion Resonances: Definitions, Widths, and …* (2026) — https://ross.aoe.vt.edu/papers/rawat-kumar-rosengren-ross-2026.pdf
- *Lunar Satellite Constellations in Frozen Low Orbits*, Aerospace 11(11):918 (2024) — https://www.mdpi.com/2226-4310/11/11/918
- Russell, R. P. & Lara, M., *Long-Lifetime Lunar Repeat Ground Track Orbits* (referenced) — see also *Lunar Frozen Orbits*, AIAA 2006-6749 — https://arc.aiaa.org/doi/10.2514/6.2006-6749
