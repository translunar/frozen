import type { Catalog, Combo, Family, Terms } from './types';

const GM_EARTH_KM3_S2 = 398_600.435507;
const GM_MOON_KM3_S2 = 4_902.800118;
/** Mass parameter mu = GM_moon / (GM_earth + GM_moon) — standard CR3BP convention. */
const MU = GM_MOON_KM3_S2 / (GM_EARTH_KM3_S2 + GM_MOON_KM3_S2);
const MOON_RADIUS_ND = 1_737.4 / 384_400;
const J2 = 2.0323e-4;
const C22 = 2.2426e-5;
const J3 = 8.46e-6;

/**
 * Jacobi-like energy of a nondim rotating-frame state, in the same force model as one
 * catalog combo: E = |v|^2/2 - Omega_eff, where Omega_eff is the centrifugal + gravity +
 * (gated) harmonic potential. State is Moon-centered ([x,y,z,vx,vy,vz]); Earth sits at
 * (-1,0,0) in these coordinates when its term is active. `bx` re-centers the centrifugal
 * term on the Earth-Moon barycenter when Earth's gravity is part of the model — otherwise
 * the frame simply rotates about the Moon itself.
 */
export function energyNd(state0: number[], terms: Terms): number {
  const [x, y, z, vx, vy, vz] = state0;
  const muM = MU;
  const muE = 1 - MU;
  const bx = terms.earth ? -(1 - MU) : 0;
  const R = MOON_RADIUS_ND;
  const k2 = muM * J2 * R ** 2;
  const k22 = muM * C22 * R ** 2;
  const k3 = muM * J3 * R ** 3;

  const r = Math.sqrt(x * x + y * y + z * z);
  let omega = 0.5 * ((x - bx) ** 2 + y * y) + muM / r;

  if (terms.earth) {
    const dxE = x + 1;
    const rE = Math.sqrt(dxE * dxE + y * y + z * z);
    omega += muE / rE;
  }
  if (terms.j2) {
    omega += (-1.5 * k2 * z ** 2) / r ** 5 + (0.5 * k2) / r ** 3;
  }
  if (terms.c22) {
    omega += (3 * k22 * (x * x - y * y)) / r ** 5;
  }
  if (terms.j3) {
    omega += (-2.5 * k3 * z ** 3) / r ** 7 + (1.5 * k3 * z) / r ** 5;
  }

  const v2 = vx * vx + vy * vy + vz * vz;
  return 0.5 * v2 - omega;
}

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
  // Metric-strip selections, promoted out of ui/stabilityPlot.ts's closure-local state so a
  // redraw (or an HMR/page reload rehydrating from sessionStorage — see hydrateUIState) can't
  // silently reset them: any component can read the authoritative current selection off the
  // store instead of trusting a variable that only one closure happens to own. Loosely typed
  // as `string` here (not MetricKey/XAxisKey) to avoid a ui/ -> state.ts dependency inversion;
  // consumers validate against their own known option list and fall back to their own default.
  metric: string;
  xAxis: string;
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

const SIDEREAL_MONTH_S = 86_400 * 27.321661;

/** Winding angle of the resonant mode: acos(nu), clamped to a valid domain, in degrees. */
export function windingAngleDeg(nu: number): number {
  const clamped = Math.min(1, Math.max(-1, nu));
  return Math.acos(clamped) * (180 / Math.PI);
}

/**
 * Libration period implied by a mode's winding angle, in sidereal months: a full libration
 * cycle takes (360/theta) closure periods. Infinity when theta = 0 (nu clamps to 1 — the mode
 * doesn't wind at all, so there's no finite libration period).
 */
export function librationPeriodMonths(nu: number, periodS: number): number {
  const thetaDeg = windingAngleDeg(nu);
  if (thetaDeg === 0) return Infinity;
  return ((360 / thetaDeg) * periodS) / SIDEREAL_MONTH_S;
}

/** Distance from the |nu| = 1 marginal-stability boundary, floored so it plots on a log axis. */
export function stabilityMargin(nu: number): number {
  return Math.max(1e-6, 1 - Math.abs(nu));
}

/** Same fractional position along a family of a different length. */
export function nearestMemberIndex(fromIndex: number, fromLength: number, toLength: number): number {
  if (toLength <= 1 || fromLength <= 1) return 0;
  const frac = fromIndex / (fromLength - 1);
  return Math.min(toLength - 1, Math.max(0, Math.round(frac * (toLength - 1))));
}

/**
 * A family's member INDICES (into `family.members`), reordered so periapsis altitude reads
 * ascending. The catalog stores members in continuation-walk order, which near a
 * near-degenerate step can zigzag — hp/e/energy all locally backtrack along the stored
 * sequence. Every UI touchpoint that presents "a family's members" as an ordered sequence
 * (the member slider, the metric strip's x-axis, cross-family carry-over) should walk this
 * order instead of the raw array, so near-duplicate members sit adjacent and nothing
 * backtracks. Ties (equal r_peri_km) keep their original index order for a stable result.
 */
export function displayOrder(family: Family): number[] {
  return family.members
    .map((_, i) => i)
    .sort((a, b) => {
      const diff = family.members[a].r_peri_km - family.members[b].r_peri_km;
      return diff !== 0 ? diff : a - b;
    });
}

/**
 * Same fractional position along a family of a different length, but carried over in
 * *display-rank* space (see `displayOrder`) rather than raw storage-index space — the
 * catalog's near-degenerate walk order means a raw-index carry-over could land on a
 * wildly different orbit shape than the one actually adjacent on screen.
 */
export function nearestMemberIndexByRank(fromIndex: number, fromFamily: Family, toFamily: Family): number {
  const fromOrder = displayOrder(fromFamily);
  const toOrder = displayOrder(toFamily);
  const fromRank = Math.max(0, fromOrder.indexOf(fromIndex));
  const toRank = nearestMemberIndex(fromRank, fromOrder.length, toOrder.length);
  return toOrder[toRank] ?? 0;
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

/**
 * Sample indices covering a trailing time window ending at `t`, over a closed trajectory of
 * `sampleCount` uniformly-spaced samples spanning exactly one `period` (same layout as
 * `samplePosition`: sample i sits at time (i/sampleCount)*period, wrapping back to sample 0).
 * Returned indices are ordered oldest-to-newest and end at the sample nearest `t`; the window
 * wraps around the seam (index sampleCount-1 -> 0) and clamps to the full loop when `windowS`
 * exceeds `period`, rather than repeating indices from a second lap.
 */
export function trailingWindowIndices(
  sampleCount: number, period: number, t: number, windowS: number,
): number[] {
  if (sampleCount <= 0 || !(period > 0)) return [];
  const tt = ((t % period) + period) % period;
  const endIdx = Math.floor((tt / period) * sampleCount) % sampleCount;
  const spanS = Math.min(Math.max(0, windowS), period);
  const count = Math.min(sampleCount, Math.max(1, Math.round((spanS / period) * sampleCount) + 1));
  const out: number[] = [];
  for (let k = count - 1; k >= 0; k--) {
    out.push(((endIdx - k) % sampleCount + sampleCount) % sampleCount);
  }
  return out;
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

const SYNODIC_MONTH_S = 2_551_442.9;

/**
 * Sidereal-ish revs per closure: an M:k family's closure period spans k node-regression
 * periods, so each one gets M/k of the total M revs (74.5 for a 149:2 family).
 */
export function sidRevsPerClosure(revs: number, closures: number): number {
  return closures > 0 ? revs / closures : 0;
}

/**
 * Revs expressed in synodic months instead: `periodS` is the *full* k-closure period (not a
 * single node-regression period), so this is revs per closure scaled by how many synodic
 * months that whole closure spans.
 */
export function synodicRevs(revs: number, periodS: number): number {
  return periodS > 0 ? revs * (SYNODIC_MONTH_S / periodS) : 0;
}

/** Whole numbers print bare; anything else gets one decimal (74.5, not 74.50 or 74.5000001). */
export function formatRevs(x: number): string {
  return Number.isInteger(x) ? String(x) : x.toFixed(1);
}

export interface RationalFit { p: number; q: number; err: number }

/** Best p/q approximation of x with denominator q <= maxDen, by raw |x - p/q| (brute force). */
export function nearestRational(x: number, maxDen: number): RationalFit {
  let best: RationalFit = { p: Math.round(x), q: 1, err: Math.abs(x - Math.round(x)) };
  for (let q = 2; q <= maxDen; q++) {
    const p = Math.round(x * q);
    const err = Math.abs(x - p / q);
    if (err < best.err) best = { p, q, err };
  }
  return best;
}

/**
 * Human-readable rational-resonance badge for a synodic-month rev count, e.g. `≈161:2 syn
 * (3°)` — the residual is the orbit-phase error accumulated over q synodic months if x were
 * exactly p/q, in degrees. Returns '' when that residual exceeds `gateDeg`: a "resonance"
 * whose phase actually drifts by 100°+ per closure is folklore, not a real repeat.
 */
export function resonanceBadge(x: number, maxDen = 4, gateDeg = 20): string {
  const { p, q, err } = nearestRational(x, maxDen);
  const residualDeg = err * q * 360;
  if (residualDeg > gateDeg) return '';
  return `≈${p}:${q} syn (${Math.round(residualDeg)}°)`;
}

/** The slice of AppState worth surviving a page reload / HMR — small enough to serialize
 * wholesale, no ghost/animation/speed state (that's session-instant, not "context"). */
export interface PersistedUIState {
  comboId: string;
  familyN: number;
  memberIndex: number;
  metric: string;
  xAxis: string;
}

/**
 * Parses a JSON blob (typically sessionStorage's last-written value) into a validated UI-state
 * patch, falling back to `defaults` field-by-field for anything missing, malformed, or the
 * wrong type. Never throws: corrupt JSON, a non-object payload, or a stale/foreign shape all
 * just yield `defaults` back — a hydration bug should degrade to "start fresh", not crash boot.
 */
export function hydrateUIState(raw: string | null, defaults: PersistedUIState): PersistedUIState {
  if (!raw) return defaults;
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return defaults;
  }
  if (typeof parsed !== 'object' || parsed === null) return defaults;
  const p = parsed as Record<string, unknown>;
  const str = (v: unknown, fallback: string): string => (typeof v === 'string' && v !== '' ? v : fallback);
  const nonNegInt = (v: unknown, fallback: number): number =>
    (typeof v === 'number' && Number.isFinite(v) && v >= 0 ? Math.round(v) : fallback);
  return {
    comboId: str(p.comboId, defaults.comboId),
    familyN: typeof p.familyN === 'number' && Number.isFinite(p.familyN) ? p.familyN : defaults.familyN,
    memberIndex: nonNegInt(p.memberIndex, defaults.memberIndex),
    metric: str(p.metric, defaults.metric),
    xAxis: str(p.xAxis, defaults.xAxis),
  };
}
