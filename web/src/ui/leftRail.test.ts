import { describe, expect, it } from 'vitest';
import {
  familyDualClockLines, familyHpRangeKm, familyMainLabel, familyReferenceTag, formatReadout,
  memberEndpointLabel, revHoursPerOrbit, TERM_LABELS,
} from './leftRail';
import { makeFamily, makeMember } from '../testFixtures';

const SYNODIC_MONTH_S = 2_551_442.9;

describe('formatReadout', () => {
  it('renders every metadata field the readout card shows', () => {
    const rows = formatReadout(makeMember(0), makeFamily(25, 1));
    const get = (label: string) => rows.find((r) => r.label === label)?.value;
    expect(get('a')).toBe('6000 km');
    expect(get('e')).toBe('0.6000');
    expect(get('i (EM plane)')).toBe('57.00°');
    expect(get('period')).toBe('27.322 d');
    expect(get('revs')).toBe('25');
    expect(get('syn revs')).toBe('27.0'); // synodicRevs(25, 2_360_591) = 27.04... -> 27.0
    expect(get('peri alt')).toBe('663 km');   // 2400 − 1737.4 km
    expect(get('apo alt')).toBe('7863 km');   // 9600 − 1737.4 km
    expect(get('ν₁')).toBe('1.200');
    expect(get('ν₂')).toBe('-0.400');
    expect(get('residual')).toBe('1.0e-11');
    expect(rows).toHaveLength(13);
  });

  it('shows "<M> over <k> closures" for the revs row of a k>1 rational-resonance family', () => {
    const family = makeFamily(149, 1, { closures: 2 });
    const rows = formatReadout(makeMember(0), family);
    expect(rows.find((r) => r.label === 'revs')?.value).toBe('149 over 2 closures');
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

describe('familyMainLabel', () => {
  it('is unchanged for a k=1 (plain integer) family', () => {
    expect(familyMainLabel(makeFamily(25, 4))).toBe('N = 25 · ~26 h/rev');
  });

  it('shows the per-closure rev count and the M:k pair for a k>1 family', () => {
    // periodS engineered so periodS/3600/149 = 8.8 exactly.
    const periodS = 149 * 8.8 * 3_600;
    const family = makeFamily(149, 3, { closures: 2 });
    family.members.forEach((m) => { m.period_s = periodS; });
    expect(familyMainLabel(family)).toBe('N = 74.5 (149:2) · ~8.8 h/rev');
  });
});

describe('familyDualClockLines', () => {
  it('k=1, no sun-geometry repeat within 4 months: plain-language lines + precise tooltips', () => {
    // revs=25, closures=1, period engineered so synodicRevs(25, periodS) ~= 28.3 -> nearestRational
    // lands on 85:3 with a 36 deg residual (> the 20 deg gate) — see state.test.ts resonanceBadge.
    const family = makeFamily(25, 3, { closures: 1 });
    family.members.forEach((m) => { m.period_s = 2_253_924.82; });
    const lines = familyDualClockLines(family);
    expect(lines.line1).toBe('track repeats: 25 orbits ≈ 26.1 d');
    expect(lines.line1Title).toBe('25 rev / sidereal closure · sidereal closure = ground-track repeat period');
    expect(lines.line2).toBe('sun geometry: no repeat within 4 months');
    expect(lines.line2Title).toBe(
      '28.30 rev / synodic month · 85:3 · residual 36.0° · synodic month = Sun-Earth-Moon alignment period',
    );
  });

  it('k>1, sun-geometry repeat passes the gate: plain-language lines + precise tooltips', () => {
    // Engineered (see state.test.ts resonanceBadge) so synodicRevs lands exactly 3 deg of
    // residual off 161:2.
    const x = 80.5 + 3 / 720;
    const periodS = (149 * SYNODIC_MONTH_S) / x;
    const family = makeFamily(149, 3, { closures: 2 });
    family.members.forEach((m) => { m.period_s = periodS; });
    const lines = familyDualClockLines(family);
    expect(lines.line1).toBe('track repeats: 149 orbits ≈ 54.7 d');
    expect(lines.line1Title).toBe('74.5 rev / sidereal closure · sidereal closure = ground-track repeat period');
    expect(lines.line2).toBe('sun geometry repeats: ~161 orbits ≈ 2 synodic months (3° drift)');
    expect(lines.line2Title).toBe(
      '80.50 rev / synodic month · 161:2 · residual 3.0° · synodic month = Sun-Earth-Moon alignment period',
    );
  });

  it('pluralizes "synodic month" to singular when q=1', () => {
    // revs=5, period engineered so synodicRevs(5, periodS) = 5.02 -> nearestRational lands on
    // 5:1 with a 7.2 deg residual (passes the gate).
    const family = makeFamily(5, 2, { closures: 1 });
    family.members.forEach((m) => { m.period_s = (5 * SYNODIC_MONTH_S) / 5.02; });
    const lines = familyDualClockLines(family);
    expect(lines.line1).toBe('track repeats: 5 orbits ≈ 29.4 d');
    expect(lines.line2).toBe('sun geometry repeats: ~5 orbits ≈ 1 synodic month (7° drift)');
    expect(lines.line2Title).toBe(
      '5.02 rev / synodic month · 5:1 · residual 7.2° · synodic month = Sun-Earth-Moon alignment period',
    );
  });
});

describe('familyReferenceTag', () => {
  it('flags a family whose mid-member a_km lands within an agency reference band, with its signed offset', () => {
    const family = makeFamily(30, 5);
    family.members[2].elements.a_km = 11_315.9; // exact NASA LCRNS hit
    expect(familyReferenceTag(family)).toBe('≈ NASA LCRNS (+0.0%) band');
  });

  it('is empty when no reference is within tolerance', () => {
    const family = makeFamily(30, 5);
    family.members[2].elements.a_km = 50_000;
    expect(familyReferenceTag(family)).toBe('');
  });

  it('comma-joins every crowded band with its own offset, closest first, singular "band"', () => {
    const family = makeFamily(30, 5);
    family.members[2].elements.a_km = 6_100; // Stanford LNCSS (-0.7%) and ESA LCNS COM (+1.7%)
    expect(familyReferenceTag(family)).toBe('≈ Stanford LNCSS (−0.7%), ESA LCNS COM (+1.7%) band');
  });

  it('distinguishes two families near the same reference from opposite sides', () => {
    // N=25-style case: a_mid=10049.3 is +3.1% above ESA LCNS NAV (9750.7).
    const above = makeFamily(25, 5);
    above.members[2].elements.a_km = 10_049.3;
    expect(familyReferenceTag(above)).toBe('≈ ESA LCNS NAV (+3.1%) band');

    // N=27-style case: a_mid=9569.3 is -1.9% below the same reference.
    const below = makeFamily(27, 5);
    below.members[2].elements.a_km = 9_569.3;
    expect(familyReferenceTag(below)).toBe('≈ ESA LCNS NAV (−1.9%) band');
  });
});
