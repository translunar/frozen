import { assertCatalog } from './types';
import type { Catalog, Family, Member } from './types';

const cache = new Map<string, Promise<Float32Array>>();

export function clearCache(): void {
  cache.clear();
}

export function joinUrl(base: string, path: string): string {
  const b = base.replace(/\/+$/, '');
  const p = path.replace(/^\/+/, '');
  return b === '' ? p : `${b}/${p}`;
}

/** Raw little-endian float32 xyz triples in km, no header (catalog binary contract). */
export function parseF32(buf: ArrayBuffer): Float32Array {
  const n = Math.floor(buf.byteLength / 4);
  const view = new DataView(buf);
  const out = new Float32Array(n);
  for (let i = 0; i < n; i++) out[i] = view.getFloat32(i * 4, true);
  return out;
}

/** preview.f32 is every member's decimated loop concatenated; counts are point counts. */
export function splitPreview(data: Float32Array, counts: number[]): Float32Array[] {
  const out: Float32Array[] = [];
  let off = 0;
  for (const c of counts) {
    out.push(data.subarray(off, off + c * 3));
    off += c * 3;
  }
  return out;
}

export async function loadCatalog(baseUrl: string): Promise<Catalog> {
  const url = joinUrl(baseUrl, 'catalog.json');
  const res = await fetch(url);
  if (!res.ok) throw new Error(`${url}: HTTP ${res.status}`);
  return assertCatalog(await res.json());
}

export function loadF32(url: string): Promise<Float32Array> {
  const hit = cache.get(url);
  if (hit) return hit;
  const pending = fetch(url).then(async (res) => {
    if (!res.ok) throw new Error(`${url}: HTTP ${res.status}`);
    return parseF32(await res.arrayBuffer());
  });
  cache.set(url, pending);
  return pending;
}

export function memberTrajectory(baseUrl: string, member: Member): Promise<Float32Array> {
  return loadF32(joinUrl(baseUrl, member.traj));
}

export async function familyPreview(baseUrl: string, family: Family): Promise<Float32Array[]> {
  const data = await loadF32(joinUrl(baseUrl, family.preview));
  return splitPreview(data, family.preview_counts);
}

/** Fire-and-forget warming of the neighbours the member slider is about to reach. */
export function prefetchNeighbors(baseUrl: string, family: Family, index: number): void {
  for (const i of [index - 1, index + 1]) {
    const m = family.members[i];
    if (m) void memberTrajectory(baseUrl, m).catch(() => undefined);
  }
}
