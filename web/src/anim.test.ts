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
