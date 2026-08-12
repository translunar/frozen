import { describe, expect, it } from 'vitest';
import { MOON_RADIUS_KM } from '../scene';
import { occultedFraction, skyClassify, skyProject } from './skyView';

describe('skyClassify', () => {
  it('is occulted behind the disk: inside the disk radius and past the Moon (x > 0)', () => {
    expect(skyClassify(100, 0, 0)).toBe('occulted');
  });

  it('is transit in front of the disk: inside the disk radius, at or before the Moon (x <= 0)', () => {
    expect(skyClassify(-100, 0, 0)).toBe('transit');
    expect(skyClassify(0, 0, 0)).toBe('transit');
  });

  it('is clear well outside the disk radius', () => {
    expect(skyClassify(100, 3000, 0)).toBe('clear');
  });

  it('treats the disk-edge boundary (rho == R) as clear, not occulted/transit', () => {
    expect(skyClassify(100, MOON_RADIUS_KM, 0)).toBe('clear');
    expect(skyClassify(-100, 0, MOON_RADIUS_KM)).toBe('clear');
  });
});

describe('skyProject', () => {
  it('mirrors y (viewer at −x looking toward +x) and keeps z as the vertical sky axis', () => {
    expect(skyProject(0, 10, 20)).toEqual({ x: -10, y: 20 });
    expect(skyProject(500, -5, -7)).toEqual({ x: 5, y: -7 });
  });
});

describe('occultedFraction', () => {
  it('is the fraction of samples classified occulted', () => {
    const traj = new Float32Array([
      100, 0, 0,      // occulted
      100, 0, 0,      // occulted
      -100, 0, 0,     // transit
      5000, 5000, 0,  // clear
    ]);
    expect(occultedFraction(traj)).toBeCloseTo(0.5, 9);
  });

  it('is zero for an empty trajectory', () => {
    expect(occultedFraction(new Float32Array([]))).toBe(0);
  });
});
