export const SCHEMA_VERSION = 1;

export interface Terms { j2: boolean; c22: boolean; j3: boolean; earth: boolean }

export interface Elements {
  a_km: number;
  e: number;
  i_deg: number;
  omega_deg: number;
  raan_deg: number;
}

export interface Member {
  index: number;
  state0: number[];        // nondimensional rotating-frame [x,y,z,vx,vy,vz]
  period_s: number;
  period_nd: number;
  elements: Elements;
  nu1: number;
  nu2: number;
  r_peri_km: number;
  r_apo_km: number;
  residual: number;
  traj: string;            // path relative to the catalog root
}

export interface Family {
  resonance_n: number;
  /** k in an M:k rational resonance (M = resonance_n revs per k node-regression periods). */
  closures?: number;
  members: Member[];
  preview: string;         // path relative to the catalog root
  preview_counts: number[]; // points per member inside preview.f32
}

/** k in an M:k resonance; serde default (and thus the JS default) is 1 — a plain integer family. */
export function familyClosures(f: Family): number {
  return f.closures ?? 1;
}

export interface Combo {
  id: string;
  name: string;
  terms: Terms;
  families: Family[];
}

export interface Catalog {
  schema_version: number;
  generated: { date: string; git_hash: string };
  constants: Record<string, number | string>;
  combos: Combo[];
}

/** Runtime shape check at the trust boundary: JSON off the network is `unknown`. */
export function assertCatalog(value: unknown): Catalog {
  if (typeof value !== 'object' || value === null) throw new Error('catalog: not an object');
  const c = value as Catalog;
  if (c.schema_version !== SCHEMA_VERSION) {
    throw new Error(`catalog: schema_version ${String(c.schema_version)} != ${SCHEMA_VERSION}`);
  }
  if (!Array.isArray(c.combos)) throw new Error('catalog: combos must be an array');
  for (const combo of c.combos) {
    if (typeof combo.id !== 'string') throw new Error('catalog: combo missing id');
    if (typeof combo.terms?.j2 !== 'boolean') throw new Error(`catalog: combo ${combo.id} missing terms`);
    if (!Array.isArray(combo.families)) throw new Error(`catalog: combo ${combo.id} missing families`);
    for (const fam of combo.families) {
      if (typeof fam.resonance_n !== 'number') {
        throw new Error(`catalog: combo ${combo.id} family missing resonance_n`);
      }
      if (!Array.isArray(fam.members) || fam.members.length === 0) {
        throw new Error(`catalog: combo ${combo.id} N=${fam.resonance_n} has no members`);
      }
    }
  }
  return c;
}
