import { describe, expect, it } from 'vitest';
import { overlayLine } from './overlay';
import { makeMember } from '../testFixtures';

describe('overlayLine', () => {
  it('formats N, altitudes, eccentricity, inclination, argument of periapsis, and rev hours', () => {
    const member = makeMember(0, {
      r_peri_km: 4_831.4,  // hp 3,094 km above the 1737.4 km Moon radius
      r_apo_km: 9_981.4,   // ha 8,244 km
      elements: { a_km: 6000, e: 0.52, i_deg: 48.8, omega_deg: 90, raan_deg: 90 },
      period_s: 2_349_000, // 26.1 h/rev * 3600 * 25
    });
    expect(overlayLine(member, 25)).toBe(
      'N=25 · hp 3,094 km · ha 8,244 km · e 0.520 · i 48.8° (EM plane) · ω 90.0° · 26.1 h/rev',
    );
  });

  it('defaults closures to 1, leaving the N label a bare integer', () => {
    expect(overlayLine(makeMember(0), 25)).toContain('N=25 ·');
  });

  it('shows N=<M>:<k> for a k>1 rational-resonance family', () => {
    const member = makeMember(0, { period_s: 2_349_000 });
    expect(overlayLine(member, 149, 2)).toContain('N=149:2 ·');
  });
});
