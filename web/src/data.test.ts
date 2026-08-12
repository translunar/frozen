import { afterEach, describe, expect, it, vi } from 'vitest';
import { assertCatalog } from './types';
import {
  clearCache, familyPreview, joinUrl, loadCatalog, loadF32,
  memberTrajectory, parseF32, splitPreview,
} from './data';
import { makeCatalog, makeMember } from './testFixtures';

function f32Buffer(values: number[]): ArrayBuffer {
  const buf = new ArrayBuffer(values.length * 4);
  const view = new DataView(buf);
  values.forEach((v, i) => view.setFloat32(i * 4, v, true));
  return buf;
}

afterEach(() => {
  clearCache();
  vi.unstubAllGlobals();
});

describe('joinUrl', () => {
  it('joins without doubling or dropping slashes', () => {
    expect(joinUrl('catalog', 'full/n25/0.f32')).toBe('catalog/full/n25/0.f32');
    expect(joinUrl('catalog/', '/full/n25/0.f32')).toBe('catalog/full/n25/0.f32');
    expect(joinUrl('', 'catalog.json')).toBe('catalog.json');
  });
});

describe('parseF32', () => {
  it('decodes little-endian float32 xyz triples regardless of host endianness', () => {
    const out = parseF32(f32Buffer([1000, -2000, 3000, 4, 5, 6]));
    expect(Array.from(out)).toEqual([1000, -2000, 3000, 4, 5, 6]);
    expect(out.length / 3).toBe(2);
  });
});

describe('splitPreview', () => {
  it('slices a concatenated preview by per-member point counts', () => {
    const data = parseF32(f32Buffer(Array.from({ length: 18 }, (_, i) => i)));
    const loops = splitPreview(data, [2, 4]);
    expect(loops).toHaveLength(2);
    expect(Array.from(loops[0])).toEqual([0, 1, 2, 3, 4, 5]);
    expect(loops[1].length).toBe(12);
    expect(loops[1][0]).toBe(6);
  });
});

describe('assertCatalog', () => {
  it('accepts a well-formed catalog and rejects broken ones', () => {
    expect(assertCatalog(makeCatalog()).combos).toHaveLength(3);
    expect(() => assertCatalog(null)).toThrow(/not an object/);
    expect(() => assertCatalog({ ...makeCatalog(), schema_version: 2 })).toThrow(/schema_version/);
    expect(() => assertCatalog({ schema_version: 1 })).toThrow(/combos/);
    const noMembers = makeCatalog();
    noMembers.combos[0].families[0].members = [];
    expect(() => assertCatalog(noMembers)).toThrow(/no members/);
  });
});

describe('loadCatalog', () => {
  it('fetches <base>/catalog.json and validates it', async () => {
    const fetchMock = vi.fn(async () => ({ ok: true, status: 200, json: async () => makeCatalog() }));
    vi.stubGlobal('fetch', fetchMock);
    const cat = await loadCatalog('catalog');
    expect(fetchMock).toHaveBeenCalledWith('catalog/catalog.json');
    expect(cat.combos[0].id).toBe('full');
  });

  it('throws on a non-ok response', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => ({ ok: false, status: 404, json: async () => ({}) })));
    await expect(loadCatalog('catalog')).rejects.toThrow(/404/);
  });
});

describe('loadF32 cache', () => {
  it('fetches each url exactly once', async () => {
    const fetchMock = vi.fn(async () => ({
      ok: true, status: 200, arrayBuffer: async () => f32Buffer([1, 2, 3]),
    }));
    vi.stubGlobal('fetch', fetchMock);
    const a = await loadF32('catalog/full/n25/0.f32');
    const b = await loadF32('catalog/full/n25/0.f32');
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(Array.from(a)).toEqual([1, 2, 3]);
    expect(b).toBe(a);
  });
});

describe('memberTrajectory / familyPreview', () => {
  it('resolves member and preview paths against the catalog base', async () => {
    const seen: string[] = [];
    vi.stubGlobal('fetch', vi.fn(async (url: string) => {
      seen.push(url);
      return { ok: true, status: 200, arrayBuffer: async () => f32Buffer([0, 0, 0, 1, 1, 1]) };
    }));
    await memberTrajectory('catalog', makeMember(3, { traj: 'full/n25/3.f32' }));
    const loops = await familyPreview('catalog', {
      resonance_n: 25, members: [], preview: 'full/n25/preview.f32', preview_counts: [1, 1],
    });
    expect(seen).toEqual(['catalog/full/n25/3.f32', 'catalog/full/n25/preview.f32']);
    expect(loops.map((l) => l.length)).toEqual([3, 3]);
  });
});
