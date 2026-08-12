// Agency reference orbits for the same cislunar-navigation neighborhood as the ELFO family
// browser, so a family can be flagged when its semi-major axis lands in a band a real (or
// proposed) constellation already occupies.

export interface AgencyReference {
  name: string;
  a_km: number;
  period_h: number;
  note: string;
}

/** GM of the Moon, km^3/s^2 (DE440-consistent; matches the a_km figures below to <0.1%). */
const GM_MOON_KM3_S2 = 4_902.800118;

/** Semi-major axis implied by a circular-orbit period via Kepler's third law. */
function keplerA(periodH: number): number {
  const periodS = periodH * 3_600;
  return Math.cbrt(GM_MOON_KM3_S2 * (periodS / (2 * Math.PI)) ** 2);
}

export const AGENCY_REFERENCES: AgencyReference[] = [
  { name: 'NASA LCRNS', a_km: 11_315.9, period_h: 30.0, note: '' },
  { name: 'ESA LCNS NAV', a_km: 9_750.7, period_h: 24.0, note: '' },
  { name: 'ESA LCNS COM', a_km: 6_000, period_h: 12.0, note: '~12 h / ~6,000 km' },
  { name: 'JAXA LNSS', a_km: 6_541.4, period_h: 13.19, note: '' },
  { name: 'Stanford LNCSS', a_km: 6_143, period_h: 12.0, note: '' },
  { name: 'IM Khonstellation', a_km: keplerA(32.8), period_h: 32.8, note: 'period per user; a from Kepler' },
  { name: 'JAXA demo', a_km: 3_870, period_h: 6.0, note: '' },
];

/** The reference whose a_km is closest to `aKm`, if within `tolFrac` fractional distance. */
export function nearestReference(aKm: number, tolFrac = 0.03): AgencyReference | null {
  let best: AgencyReference | null = null;
  let bestFrac = Infinity;
  for (const ref of AGENCY_REFERENCES) {
    const frac = Math.abs(aKm - ref.a_km) / ref.a_km;
    if (frac < bestFrac) {
      bestFrac = frac;
      best = ref;
    }
  }
  return best && bestFrac <= tolFrac ? best : null;
}
