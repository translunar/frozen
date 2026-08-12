import { describe, expect, it } from 'vitest';
import { makeFamily, makeMember } from '../testFixtures';
import type { Terms } from '../types';
import {
  energyLegendLabel, formatEnergyOffset, indexFromX, linearDomain, log10Domain,
  log10TickLabel, memberIndexFromEnergy, metricCursorText, orientedOrder, singleMetricValue,
  symlogDomain, xAxisValues, xTickCount,
} from './stabilityPlot';

const FULL_TERMS: Terms = { j2: true, c22: true, j3: true, earth: true };

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

describe('memberIndexFromEnergy', () => {
  it('picks the member whose energy is closest to the target', () => {
    const energies = [-1.7950, -1.7947, -1.7940, -1.7930];
    expect(memberIndexFromEnergy(energies, -1.7947)).toBe(1); // exact hit
    expect(memberIndexFromEnergy(energies, -1.7938)).toBe(2); // closer to index 2 than 3
    expect(memberIndexFromEnergy(energies, -2)).toBe(0);      // clamps to the nearest end
    expect(memberIndexFromEnergy(energies, 0)).toBe(3);
  });

  it('keeps the earlier index on an exact tie', () => {
    expect(memberIndexFromEnergy([1, 3], 2)).toBe(0);
  });

  it('is 0 for an empty energy list (degenerate guard)', () => {
    expect(memberIndexFromEnergy([], 5)).toBe(0);
  });

  it('honestly follows a non-monotone (folded) energy sequence rather than assuming sorted input', () => {
    // A fold: energies rise then fall across member order 0..3.
    const energies = [-1.795, -1.793, -1.794, -1.796];
    expect(memberIndexFromEnergy(energies, -1.7934)).toBe(1); // closer to -1.793 than -1.794
    expect(memberIndexFromEnergy(energies, -1.7955)).toBe(3); // closer to -1.796 than -1.795
  });
});

describe('formatEnergyOffset', () => {
  it('formats a positive offset from E0 in engineering notation', () => {
    expect(formatEnergyOffset(3.2e-4)).toBe('E₀ + 3.2e-4');
  });

  it('formats a negative offset with a unicode minus', () => {
    expect(formatEnergyOffset(-1.1e-5)).toBe('E₀ − 1.1e-5');
  });

  it('is the bare E0 label at exactly zero offset', () => {
    expect(formatEnergyOffset(0)).toBe('E₀');
  });
});

describe('energyLegendLabel', () => {
  it('formats a negative E0 to 5 decimals with a unicode minus', () => {
    expect(energyLegendLabel(-1.794706268957019)).toBe('E₀ = −1.79471');
  });

  it('formats a positive E0 with no leading sign', () => {
    expect(energyLegendLabel(0.318592767741606)).toBe('E₀ = 0.31859');
  });
});

describe('xTickCount', () => {
  it('thins to 2 ticks when the pane is narrower than ~380px', () => {
    expect(xTickCount(379)).toBe(2);
    expect(xTickCount(200)).toBe(2);
  });

  it('stays at 4 ticks at 380px and wider', () => {
    expect(xTickCount(380)).toBe(4);
    expect(xTickCount(800)).toBe(4);
  });
});

describe('xAxisValues', () => {
  it('reads hp/ha/eccentricity straight off each member, in raw (true-index) order', () => {
    const family = makeFamily(25, 2);
    family.members[0].r_peri_km = 2_400;
    family.members[0].r_apo_km = 9_600;
    family.members[0].elements.e = 0.6;
    family.members[1].r_peri_km = 2_200;
    family.members[1].r_apo_km = 9_800;
    family.members[1].elements.e = 0.65;
    const hp = xAxisValues(family, 'hp', FULL_TERMS);
    const ha = xAxisValues(family, 'ha', FULL_TERMS);
    hp.forEach((v, i) => expect(v).toBeCloseTo([662.6, 462.6][i], 9));
    ha.forEach((v, i) => expect(v).toBeCloseTo([7862.6, 8062.6][i], 9));
    expect(xAxisValues(family, 'ecc', FULL_TERMS)).toEqual([0.6, 0.65]);
  });

  it('computes energy via energyNd for the "energy" mode', () => {
    const family = makeFamily(25, 1);
    const values = xAxisValues(family, 'energy', FULL_TERMS);
    expect(values).toHaveLength(1);
    expect(Number.isFinite(values[0])).toBe(true);
  });
});

describe('orientedOrder', () => {
  it('leaves the order unchanged when values already ascend along it', () => {
    const order = [2, 0, 1];
    // Value at each order slot: values[2]=10, values[0]=20, values[1]=30 -> already ascending.
    const values = [20, 30, 10];
    expect(orientedOrder(order, values)).toBe(order); // same reference: no reversal needed
  });

  it('reverses the order when values descend along it', () => {
    const order = [0, 1, 2];
    const values = [30, 20, 10]; // descending along the given order
    expect(orientedOrder(order, values)).toEqual([2, 1, 0]);
  });

  it('is a no-op for a 0- or 1-length order', () => {
    expect(orientedOrder([], [])).toEqual([]);
    expect(orientedOrder([0], [42])).toEqual([0]);
  });
});
