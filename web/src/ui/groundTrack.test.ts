import { describe, expect, it } from 'vitest';
import { groundTrack, latDomain, splitAtWraps } from './groundTrack';

describe('groundTrack', () => {
  it('reads lat 90 at the pole and lat/lon 0 at the sub-Earth point', () => {
    const r = 1737.4;
    const traj = new Float32Array([
      0, 0, r,   // pole
      -r, 0, 0,  // sub-Earth point (−x axis)
    ]);
    const track = groundTrack(traj);
    expect(track[0].lat).toBeCloseTo(90, 5);
    expect(track[1].lat).toBeCloseTo(0, 5);
    expect(track[1].lon).toBeCloseTo(0, 5);
  });

  it('reports the farside on the +x axis as longitude ±180', () => {
    const r = 1737.4;
    const track = groundTrack(new Float32Array([r, 0, 0]));
    expect(Math.abs(track[0].lon)).toBeCloseTo(180, 5);
  });

  it('handles the empty trajectory', () => {
    expect(groundTrack(new Float32Array([]))).toEqual([]);
  });
});

describe('splitAtWraps', () => {
  it('splits a loop crossing the ±180 seam into two segments', () => {
    const pts = [
      { lat: 0, lon: -170 },
      { lat: 0, lon: -175 },
      { lat: 0, lon: 175 },
      { lat: 0, lon: 170 },
    ];
    const segs = splitAtWraps(pts);
    expect(segs.length).toBe(2);
    expect(segs[0]).toEqual([pts[0], pts[1]]);
    expect(segs[1]).toEqual([pts[2], pts[3]]);
  });

  it('keeps a non-wrapping track as a single segment', () => {
    const pts = [{ lat: 0, lon: -10 }, { lat: 5, lon: 0 }, { lat: 10, lon: 10 }];
    expect(splitAtWraps(pts).length).toBe(1);
  });

  it('handles the empty track', () => {
    expect(splitAtWraps([])).toEqual([]);
  });
});

describe('latDomain', () => {
  it('pads the track lat range by 10° on each side', () => {
    expect(latDomain([{ lat: -20, lon: 0 }, { lat: 30, lon: 0 }])).toEqual([-30, 40]);
  });

  it('clamps padding to the physical ±90° range', () => {
    expect(latDomain([{ lat: -85, lon: 0 }, { lat: 88, lon: 0 }])).toEqual([-90, 90]);
  });

  it('falls back to ±90 for an empty track', () => {
    expect(latDomain([])).toEqual([-90, 90]);
  });
});
