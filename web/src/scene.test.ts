import { describe, expect, it } from 'vitest';
import { KM_TO_SCENE, MOON_RADIUS_KM, presetCamera, scenePositions } from './scene';

describe('scenePositions', () => {
  it('converts km to scene megametres without changing the axis order', () => {
    const out = scenePositions(new Float32Array([1000, -2000, 3000, 0, 0, 1737.4]));
    expect(Array.from(out.slice(0, 3))).toEqual([1, -2, 3]);
    expect(out[5]).toBeCloseTo(MOON_RADIUS_KM * KM_TO_SCENE, 6);
    expect(out.length).toBe(6);
  });
});

describe('presetCamera', () => {
  it('south-pole view looks along +z from below with +x as screen-up', () => {
    const pose = presetCamera('south-pole', 12);
    expect(pose.position).toEqual([0, 0, -12]);
    expect(pose.up).toEqual([1, 0, 0]);
  });

  it('earth-line view keeps the orbit normal as screen-up and stays off the x axis', () => {
    const pose = presetCamera('earth-line', 12);
    expect(pose.up).toEqual([0, 0, 1]);
    expect(pose.position[0]).toBe(0);
    expect(pose.position[1]).toBeLessThan(0);
    // the up vector must not be parallel to the view direction
    expect(Math.abs(pose.position[1])).toBeGreaterThan(Math.abs(pose.position[2]));
  });
});
