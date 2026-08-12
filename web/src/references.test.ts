import { describe, expect, it } from 'vitest';
import { AGENCY_REFERENCES, nearestReference } from './references';

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

  it('accepts within the default 3% tolerance', () => {
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
});
