import { describe, expect, it } from 'vitest';
import { comboById, findCombo, flipTerm, nearestResonance, termAvailability } from './state';
import { makeCatalog } from './testFixtures';
import type { Terms } from './types';

const FULL: Terms = { j2: true, c22: true, j3: true, earth: true };

describe('flipTerm', () => {
  it('returns a new object with exactly one term inverted', () => {
    const flipped = flipTerm(FULL, 'c22');
    expect(flipped).toEqual({ j2: true, c22: false, j3: true, earth: true });
    expect(FULL.c22).toBe(true);
  });
});

describe('findCombo', () => {
  it('matches on the full four-term signature', () => {
    expect(findCombo(makeCatalog(), FULL)?.id).toBe('full');
    expect(findCombo(makeCatalog(), flipTerm(FULL, 'c22'))?.id).toBe('no-c22');
    expect(findCombo(makeCatalog(), flipTerm(FULL, 'earth'))?.id).toBe('no-earth');
  });

  it('returns undefined for a toggle state outside the curated set', () => {
    expect(findCombo(makeCatalog(), flipTerm(FULL, 'j2'))).toBeUndefined();
    expect(findCombo(makeCatalog(), { j2: false, c22: false, j3: false, earth: false })).toBeUndefined();
  });
});

describe('termAvailability', () => {
  it('marks only the flips that land on a catalogued combo', () => {
    expect(termAvailability(makeCatalog(), FULL)).toEqual({
      j2: false, c22: true, j3: false, earth: true,
    });
  });

  it('is computed from the target combo, so it changes as you move around', () => {
    const noEarth = findCombo(makeCatalog(), flipTerm(FULL, 'earth'))!;
    // flipping earth back on returns to "full"; nothing else is catalogued
    expect(termAvailability(makeCatalog(), noEarth.terms)).toEqual({
      j2: false, c22: false, j3: false, earth: true,
    });
  });
});

describe('nearestResonance', () => {
  it('picks the closest available resonance in the target combo', () => {
    const cat = makeCatalog();
    expect(nearestResonance(comboById(cat, 'full')!, 40)).toBe(30);
    expect(nearestResonance(comboById(cat, 'full')!, 25)).toBe(25);
    expect(nearestResonance(comboById(cat, 'no-earth')!, 25)).toBe(40);
  });

  it('returns null for a combo with no families', () => {
    expect(nearestResonance({ id: 'x', name: 'x', terms: FULL, families: [] }, 25)).toBeNull();
  });
});
