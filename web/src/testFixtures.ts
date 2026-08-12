import type { Catalog, Combo, Family, Member, Terms } from './types';

export function makeMember(index: number, over: Partial<Member> = {}): Member {
  return {
    index,
    state0: [0.02, 0, 0.01, 0, -0.6, 0.3],
    period_s: 2360591,
    period_nd: 6.2831853,
    elements: { a_km: 6000 + index * 10, e: 0.6, i_deg: 57, omega_deg: 90, raan_deg: 90 },
    nu1: 1.2 + index * 0.1,
    nu2: -0.4,
    r_peri_km: 2400,
    r_apo_km: 9600,
    residual: 1e-11,
    traj: `full/n25/${index}.f32`,
    ...over,
  };
}

export function makeFamily(n: number, count: number): Family {
  return {
    resonance_n: n,
    members: Array.from({ length: count }, (_, i) => makeMember(i, { traj: `full/n${n}/${i}.f32` })),
    preview: `full/n${n}/preview.f32`,
    preview_counts: Array.from({ length: count }, () => 1000),
  };
}

export function makeCombo(id: string, name: string, terms: Terms, families: Family[]): Combo {
  return { id, name, terms, families };
}

export function makeCatalog(): Catalog {
  return {
    schema_version: 1,
    generated: { date: '2026-08-11T00:00:00Z', git_hash: 'abc1234' },
    constants: { R_MOON_KM: 1737.4, source: 'DE440 / GRGM1200' },
    combos: [
      makeCombo('full', 'J2 + C22 + J3 + Earth',
        { j2: true, c22: true, j3: true, earth: true },
        [makeFamily(25, 5), makeFamily(30, 4)]),
      makeCombo('no-c22', 'J2 + J3 + Earth (C22 off)',
        { j2: true, c22: false, j3: true, earth: true },
        [makeFamily(25, 7)]),
      makeCombo('no-earth', 'J2 + C22 + J3 (Earth off)',
        { j2: true, c22: true, j3: true, earth: false },
        [makeFamily(40, 3)]),
    ],
  };
}
