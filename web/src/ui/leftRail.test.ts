import { describe, expect, it } from 'vitest';
import {
  familyHpRangeKm, formatReadout, memberEndpointLabel, revHoursPerOrbit, TERM_LABELS,
} from './leftRail';
import { makeFamily, makeMember } from '../testFixtures';

describe('formatReadout', () => {
  it('renders every metadata field the readout card shows', () => {
    const rows = formatReadout(makeMember(0), makeFamily(25, 1));
    const get = (label: string) => rows.find((r) => r.label === label)?.value;
    expect(get('a')).toBe('6000 km');
    expect(get('e')).toBe('0.6000');
    expect(get('i (EM plane)')).toBe('57.00°');
    expect(get('period')).toBe('27.322 d');
    expect(get('revs')).toBe('25');
    expect(get('peri alt')).toBe('663 km');   // 2400 − 1737.4 km
    expect(get('apo alt')).toBe('7863 km');   // 9600 − 1737.4 km
    expect(get('ν₁')).toBe('1.200');
    expect(get('ν₂')).toBe('-0.400');
    expect(get('residual')).toBe('1.0e-11');
    expect(rows).toHaveLength(12);
  });
});

describe('memberEndpointLabel', () => {
  it('annotates a slider endpoint with peri/apo altitude and eccentricity, no index', () => {
    expect(memberEndpointLabel(makeMember(0))).toBe('hp 663 · ha 7863 km · e 0.600');
    // Index must not leak into the label — this is a downplayed, count-free readout.
    expect(memberEndpointLabel(makeMember(17))).not.toContain('17');
  });
});

describe('revHoursPerOrbit', () => {
  it('is the closure period divided into hours-per-revolution by the resonance number', () => {
    // fixture period_s = 2_360_591 (27.322 d = 655.72 h); N = 25 -> 26.23 h/rev
    expect(revHoursPerOrbit(makeFamily(25, 4))).toBe(26);
  });
});

describe('familyHpRangeKm', () => {
  it('spans min/max periapsis altitude across the family, rounded to whole km', () => {
    const family = makeFamily(25, 3);
    family.members[0].r_peri_km = 2400;   // hp 663
    family.members[1].r_peri_km = 2200;   // hp 463 (min)
    family.members[2].r_peri_km = 2600;   // hp 863 (max)
    expect(familyHpRangeKm(family)).toEqual([463, 863]);
  });
});

describe('TERM_LABELS', () => {
  it('covers all four toggleable force terms in a stable order', () => {
    expect(TERM_LABELS.map(([k]) => k)).toEqual(['j2', 'c22', 'j3', 'earth']);
  });
});
