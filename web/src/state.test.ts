import { describe, expect, it } from 'vitest';
import {
  comboById, createStore, elapsedRevs, energyNd, familyByN,
  librationPeriodMonths, nearestMemberIndex, nearestRational, resonanceBadge,
  samplePosition, sidRevsPerClosure, stabilityMargin, symlog, synodicRevs,
  trailingWindowIndices, windingAngleDeg,
} from './state';
import type { AppState } from './state';
import { makeCatalog } from './testFixtures';
import type { Terms } from './types';

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

describe('windingAngleDeg', () => {
  it('is acos(nu) in degrees, hand-checked', () => {
    expect(windingAngleDeg(0.8)).toBeCloseTo(36.87, 2);
    expect(windingAngleDeg(1)).toBeCloseTo(0, 9);
    expect(windingAngleDeg(-1)).toBeCloseTo(180, 9);
    expect(windingAngleDeg(0)).toBeCloseTo(90, 9);
  });

  it('clamps out-of-range nu before taking acos', () => {
    expect(windingAngleDeg(1.4)).toBeCloseTo(0, 9);
    expect(windingAngleDeg(-1.4)).toBeCloseTo(180, 9);
  });
});

describe('librationPeriodMonths', () => {
  const SIDEREAL_MONTH_S = 86_400 * 27.321661;

  it('converts winding angle + closure period into sidereal months', () => {
    // theta = acos(0.8) = 36.8699 deg; libration period = (360/theta) * periodS / month
    const periodS = 2_360_591;
    const expected = (360 / windingAngleDeg(0.8)) * periodS / SIDEREAL_MONTH_S;
    expect(librationPeriodMonths(0.8, periodS)).toBeCloseTo(expected, 9);
  });

  it('is Infinity when theta is exactly zero (nu clamps to 1, div-by-zero guard)', () => {
    expect(librationPeriodMonths(1, 2_360_591)).toBe(Infinity);
    expect(librationPeriodMonths(1.5, 2_360_591)).toBe(Infinity);
  });
});

describe('stabilityMargin', () => {
  it('is 1 - |nu|, floored for log plotting', () => {
    expect(stabilityMargin(0.4)).toBeCloseTo(0.6, 12);
    expect(stabilityMargin(-0.4)).toBeCloseTo(0.6, 12);
    expect(stabilityMargin(0)).toBeCloseTo(1, 12);
  });

  it('floors at 1e-6 instead of hitting zero or negative', () => {
    expect(stabilityMargin(1)).toBe(1e-6);
    expect(stabilityMargin(1.2)).toBe(1e-6);
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

describe('trailingWindowIndices', () => {
  it('returns a non-wrapping trailing window ending at the sample for t', () => {
    // 10 samples over period 100: sample i at time 10*i. t=50 -> sample 5; a 20s window
    // covers 2 sample steps back, i.e. 3 samples: [3, 4, 5].
    expect(trailingWindowIndices(10, 100, 50, 20)).toEqual([3, 4, 5]);
  });

  it('wraps around the seam back to the end of the array', () => {
    // t=5 -> sample 0; a 30s window (3 sample steps) wraps to [7, 8, 9, 0].
    expect(trailingWindowIndices(10, 100, 5, 30)).toEqual([7, 8, 9, 0]);
  });

  it('clamps a window longer than the period to the full loop, not a repeated lap', () => {
    const idx = trailingWindowIndices(10, 100, 0, 250);
    expect(idx).toHaveLength(10);
    expect(idx).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 0]);
    expect(new Set(idx).size).toBe(10); // no index repeated from a second lap
  });

  it('handles degenerate inputs without throwing', () => {
    expect(trailingWindowIndices(0, 100, 5, 20)).toEqual([]);
    expect(trailingWindowIndices(10, 0, 5, 20)).toEqual([]);
    expect(trailingWindowIndices(10, -5, 5, 20)).toEqual([]);
  });
});

describe('elapsedRevs', () => {
  it('counts revs as the resonance number scaled by period fraction', () => {
    expect(elapsedRevs(1180295.5, 2360591, 25)).toBeCloseTo(12.5, 9);
    expect(elapsedRevs(0, 2360591, 25)).toBe(0);
  });
});

const SYNODIC_MONTH_S = 2_551_442.9;

describe('sidRevsPerClosure', () => {
  it('divides total revs by the closure count (74.5 for a 149:2 family)', () => {
    expect(sidRevsPerClosure(149, 2)).toBeCloseTo(74.5, 12);
    expect(sidRevsPerClosure(25, 1)).toBe(25);
  });

  it('is 0 for a degenerate (non-positive) closure count', () => {
    expect(sidRevsPerClosure(149, 0)).toBe(0);
  });
});

describe('synodicRevs', () => {
  it('scales revs by the ratio of the synodic month to the full closure period', () => {
    // 149 revs over a period of exactly one synodic month -> 149 rev/syn-mo.
    expect(synodicRevs(149, SYNODIC_MONTH_S)).toBeCloseTo(149, 9);
    // Half as long a period -> twice as many synodic months fit -> half the revs each.
    expect(synodicRevs(149, SYNODIC_MONTH_S / 2)).toBeCloseTo(298, 9);
  });

  it('is 0 for a degenerate (non-positive) period', () => {
    expect(synodicRevs(149, 0)).toBe(0);
  });
});

describe('nearestRational', () => {
  it('hand-checks exact and near-exact hits', () => {
    expect(nearestRational(80.5, 4)).toEqual({ p: 161, q: 2, err: 0 });
    expect(nearestRational(0.75, 4)).toEqual({ p: 3, q: 4, err: 0 });
  });

  it('picks the closest denominator by raw error, hand-checked against 54.64', () => {
    // q=1: |54.64-55|=0.36; q=2: |54.64-54.5|=0.14; q=3: |54.64-164/3|=0.0267 (best);
    // q=4: |54.64-54.75|=0.11.
    const fit = nearestRational(54.64, 4);
    expect(fit).toEqual({ p: 164, q: 3, err: expect.closeTo(0.026667, 5) });
  });

  it('is forced to q=1 when maxDen is 1', () => {
    expect(nearestRational(74.5, 1)).toEqual({ p: 75, q: 1, err: 0.5 });
  });
});

describe('resonanceBadge', () => {
  it('formats a passing-gate badge with the residual in degrees, hand-checked to 149:2', () => {
    // Construct a period so that 149 revs land at exactly 80.5 + 3/720 synodic months —
    // engineered so nearestRational(x, 4) lands on 161:2 with a residual of exactly 3 deg:
    // err = |x - 80.5| = 3/720, residual = err * q(=2) * 360 = 3.
    const x = 80.5 + 3 / 720;
    const periodS = (149 * SYNODIC_MONTH_S) / x;
    expect(sidRevsPerClosure(149, 2)).toBeCloseTo(74.5, 9);
    expect(synodicRevs(149, periodS)).toBeCloseTo(x, 6);
    expect(resonanceBadge(synodicRevs(149, periodS))).toBe('≈161:2 syn (3°)');
  });

  it('rejects the folklore case (x=54.64 claimed as 109:2): residual is 100deg+', () => {
    expect(resonanceBadge(54.64)).toBe('');
  });

  it('rejects when the best-fit residual still exceeds the gate', () => {
    expect(resonanceBadge(80.5 + 3 / 720, 4, 2)).toBe(''); // 3deg residual > 2deg gate
  });

  it('accepts an exact hit with a 0deg residual', () => {
    expect(resonanceBadge(74.5, 4)).toBe('≈149:2 syn (0°)');
  });
});

describe('energyNd', () => {
  // Controller-computed ground truth: state0 [0.02, 0, 0.01, 0, -0.6, 0.3].
  const STATE0 = [0.02, 0, 0.01, 0, -0.6, 0.3];
  const FULL: Terms = { j2: true, c22: true, j3: true, earth: true };

  it('matches the controller-computed value for the full force model', () => {
    expect(energyNd(STATE0, FULL)).toBeCloseTo(-1.794706268957019, 12);
  });

  it('matches the controller-computed value with earth off', () => {
    expect(energyNd(STATE0, { ...FULL, earth: false })).toBeCloseTo(-0.318592767741606, 12);
  });

  it('matches the controller-computed value with c22 off', () => {
    expect(energyNd(STATE0, { ...FULL, c22: false })).toBeCloseTo(-1.794705074039040, 12);
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
