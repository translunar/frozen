import { describe, expect, it } from 'vitest';
import { makeMember } from '../testFixtures';
import {
  indexFromX, linearDomain, log10Domain, log10TickLabel,
  metricCursorText, singleMetricValue, symlogDomain,
} from './stabilityPlot';

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

describe('linearDomain', () => {
  it('auto-fits with 10% padding on each side', () => {
    expect(linearDomain([10, 20])).toEqual([9, 21]);
  });

  it('widens a degenerate (all-equal) domain instead of collapsing to zero width', () => {
    expect(linearDomain([5])).toEqual([3.8, 6.2]);
    expect(linearDomain([5, 5, 5])).toEqual([3.8, 6.2]);
  });
});

describe('log10Domain', () => {
  it('returns whole-decade exponent bounds around the data', () => {
    expect(log10Domain([0.1, 0.001])).toEqual([-3, -1]);
  });

  it('floors values before logging so zero/negative inputs cannot blow up', () => {
    expect(log10Domain([1e-6])).toEqual([-7, -5]);
  });
});

describe('log10TickLabel', () => {
  it('formats an exponent as a unicode-superscript power of ten', () => {
    expect(log10TickLabel(-1)).toBe('10⁻¹');
    expect(log10TickLabel(0)).toBe('10⁰');
    expect(log10TickLabel(2)).toBe('10²');
  });
});

describe('singleMetricValue', () => {
  it('computes each single-series metric from a member', () => {
    const m = makeMember(0, {
      nu2: 0.8, r_peri_km: 2400, r_apo_km: 9600,
      elements: { a_km: 6000, e: 0.6, i_deg: 57, omega_deg: 90, raan_deg: 90 },
    });
    expect(singleMetricValue('winding', m)).toBeCloseTo(36.87, 2);
    expect(singleMetricValue('margin', m)).toBeCloseTo(0.2, 12);
    expect(singleMetricValue('peri', m)).toBeCloseTo(662.6, 6);
    expect(singleMetricValue('apo', m)).toBeCloseTo(7862.6, 6);
    expect(singleMetricValue('ecc', m)).toBe(0.6);
    expect(singleMetricValue('inc', m)).toBe(57);
  });
});

describe('metricCursorText', () => {
  it('shows both angle and libration period for the winding metric', () => {
    const m = makeMember(0, { nu2: 0.8, period_s: 2_360_591 });
    expect(metricCursorText('winding', m)).toMatch(/^θ = 36\.9° · libration ≈ \d+\.\d months$/);
  });

  it('reports no finite libration period when theta is zero', () => {
    const m = makeMember(0, { nu2: 1 });
    expect(metricCursorText('winding', m)).toBe('θ = 0.0° · no libration (θ = 0°)');
  });

  it('shows the plain metric value for non-winding metrics', () => {
    const m = makeMember(0, { r_peri_km: 2400, r_apo_km: 9600 });
    expect(metricCursorText('peri', m)).toBe('peri alt = 663 km');
    expect(metricCursorText('apo', m)).toBe('apo alt = 7863 km');
  });
});
