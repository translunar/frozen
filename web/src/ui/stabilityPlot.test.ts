import { describe, expect, it } from 'vitest';
import { indexFromX, symlogDomain } from './stabilityPlot';

describe('symlogDomain', () => {
  it('always shows the ±1 stability boundary with headroom', () => {
    expect(symlogDomain([])).toEqual([-1.2, 1.2]);
    expect(symlogDomain([0.2, -0.5])).toEqual([-1.2, 1.2]);
  });

  it('grows symmetrically to fit unstable members', () => {
    const [lo, hi] = symlogDomain([100]);      // symlog(100) = 3
    expect(hi).toBeCloseTo(3.3, 9);
    expect(lo).toBeCloseTo(-3.3, 9);
    expect(symlogDomain([-1000, 0.5])[1]).toBeCloseTo(4.4, 9);
  });
});

describe('indexFromX', () => {
  it('maps plot x to a member index and clamps to the ends', () => {
    expect(indexFromX(0, 300, 11)).toBe(0);
    expect(indexFromX(150, 300, 11)).toBe(5);
    expect(indexFromX(300, 300, 11)).toBe(10);
    expect(indexFromX(-40, 300, 11)).toBe(0);
    expect(indexFromX(1000, 300, 11)).toBe(10);
    expect(indexFromX(37, 300, 1)).toBe(0);
  });
});
