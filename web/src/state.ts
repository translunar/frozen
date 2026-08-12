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
