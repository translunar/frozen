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
