import { describe, expect, it } from 'vitest';
import { formatReadout, memberEndpointLabel, TERM_LABELS } from './leftRail';
import { makeFamily, makeMember } from '../testFixtures';

describe('formatReadout', () => {
  it('renders every metadata field the readout card shows', () => {
    const rows = formatReadout(makeMember(0), makeFamily(25, 1));
    const get = (label: string) => rows.find((r) => r.label === label)?.value;
    expect(get('a')).toBe('6000 km');
    expect(get('e')).toBe('0.6000');
    expect(get('i')).toBe('57.00°');
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
  it('annotates a slider endpoint with periapsis altitude and eccentricity', () => {
    expect(memberEndpointLabel(makeMember(0))).toBe('#0 · hp 663 km · e 0.600');
  });
});

describe('TERM_LABELS', () => {
  it('covers all four toggleable force terms in a stable order', () => {
    expect(TERM_LABELS.map(([k]) => k)).toEqual(['j2', 'c22', 'j3', 'earth']);
  });
});
