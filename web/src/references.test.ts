import { describe, expect, it } from 'vitest';
import {
  AGENCY_REFERENCES, formatReferenceOffset, nearestReference, referencesWithin,
} from './references';

describe('AGENCY_REFERENCES', () => {
  it('covers all seven agency/proposal orbits', () => {
    expect(AGENCY_REFERENCES.map((r) => r.name)).toEqual([
      'NASA LCRNS', 'ESA LCNS NAV', 'ESA LCNS COM', 'JAXA LNSS',
      'Stanford LNCSS', 'IM Khonstellation', 'JAXA demo',
    ]);
  });

  it('derives IM Khonstellation a_km from its period via Kepler, near the ~12,013 km figure', () => {
    const ref = AGENCY_REFERENCES.find((r) => r.name === 'IM Khonstellation')!;
    expect(ref.a_km).toBeCloseTo(12_008, -1); // within ~10 km of the Kepler-consistent value
    expect(Math.abs(ref.a_km - 12_013) / 12_013).toBeLessThan(0.01);
  });
});

describe('nearestReference', () => {
  it('finds an exact hit', () => {
    expect(nearestReference(11_315.9)?.name).toBe('NASA LCRNS');
    expect(nearestReference(3_870)?.name).toBe('JAXA demo');
  });

  it('accepts within the default 4% tolerance', () => {
    // 6541.4 * 1.02 = 6672.2 (2% high) -> still matches JAXA LNSS
    expect(nearestReference(6541.4 * 1.02)?.name).toBe('JAXA LNSS');
  });

  it('rejects beyond tolerance', () => {
    expect(nearestReference(6541.4 * 1.10)).toBeNull(); // 10% off, no reference that close
    expect(nearestReference(1)).toBeNull();
  });

  it('respects a custom tolFrac', () => {
    expect(nearestReference(6541.4 * 1.02, 0.01)).toBeNull();
    expect(nearestReference(6541.4 * 1.02, 0.05)?.name).toBe('JAXA LNSS');
  });

  it('picks the closest reference when two bands are near, not just the first', () => {
    // ESA LCNS COM (6000) and Stanford LNCSS (6143) are both plausible near 6070 — Stanford
    // is closer in absolute terms but ESA COM is closer fractionally; assert against the
    // fractional (not absolute) distance the function documents.
    const target = 6_070;
    const fracCom = Math.abs(target - 6_000) / 6_000;
    const fracStanford = Math.abs(target - 6_143) / 6_143;
    const expected = fracCom < fracStanford ? 'ESA LCNS COM' : 'Stanford LNCSS';
    expect(nearestReference(target, 0.02)?.name).toBe(expected);
  });

  it('N=60 (a≈5,765) now matches ESA LCNS COM under the bumped 4% default, not Stanford', () => {
    // ESA LCNS COM: |5765-6000|/6000 = 3.92% (within 4%, was missed at the old 3% default).
    // Stanford LNCSS: |5765-6143|/6143 = 6.15% (still out of range).
    expect(nearestReference(5_765)?.name).toBe('ESA LCNS COM');
  });
});

describe('referencesWithin', () => {
  it('returns every reference within tolerance, closest first', () => {
    // a≈6,100: Stanford LNCSS (6,143) is 0.70% away, ESA LCNS COM (6,000) is 1.67% away —
    // both within the default 4% tolerance, Stanford first.
    const names = referencesWithin(6_100).map((m) => m.reference.name);
    expect(names).toEqual(['Stanford LNCSS', 'ESA LCNS COM']);
  });

  it('is [] when nothing is within tolerance', () => {
    expect(referencesWithin(50_000)).toEqual([]);
  });

  it('is a single-element array for an isolated exact hit', () => {
    expect(referencesWithin(11_315.9).map((m) => m.reference.name)).toEqual(['NASA LCRNS']);
  });

  it('respects a custom tolFrac', () => {
    expect(referencesWithin(6_100, 0.005)).toEqual([]); // both bands out of a 0.5% tolerance
    expect(referencesWithin(6_100, 0.02).map((m) => m.reference.name))
      .toEqual(['Stanford LNCSS', 'ESA LCNS COM']);
  });

  it('carries a signed offsetPct per match: negative when aKm is below the reference', () => {
    // Stanford LNCSS a_km=6143; 6100 is below it -> negative offset.
    // ESA LCNS COM a_km=6000; 6100 is above it -> positive offset.
    const [stanford, com] = referencesWithin(6_100);
    expect(stanford.offsetPct).toBeCloseTo(-0.6999837, 5);
    expect(com.offsetPct).toBeCloseTo(1.6666667, 5);
  });

  it('nearestReference is just the reference of its first match', () => {
    expect(nearestReference(6_100)).toEqual(referencesWithin(6_100)[0].reference);
    expect(nearestReference(50_000)).toBeNull();
  });
});

describe('formatReferenceOffset', () => {
  it('formats a positive offset with an explicit + and 1 decimal', () => {
    expect(formatReferenceOffset(3.0623)).toBe('+3.1%');
  });

  it('formats a negative offset with a unicode minus and 1 decimal', () => {
    expect(formatReferenceOffset(-1.8604)).toBe('−1.9%');
  });

  it('is +0.0% (not -0.0%) at exactly zero offset', () => {
    expect(formatReferenceOffset(0)).toBe('+0.0%');
  });
});
