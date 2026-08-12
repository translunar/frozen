import { scaleLinear } from 'd3-scale';
import type { ScaleLinear } from 'd3-scale';
import { samplePosition, trailingWindowIndices } from '../state';

// Moon ground-track panel: the selected member's sub-satellite point in the Moon-fixed
// rotating frame, plotted equirectangular. Purely driven from the outside via setMember /
// setAnimTime — no store subscription of its own (see bottomTabs.ts, which owns that wiring).
//
// The full closed track (N revs, N = resonanceN) is drawn at low prominence — for members with
// many revs it reads as clutter — while a bright "recent" polyline covers the trailing ~1.2
// revs ending at the animated marker, rebuilt each throttled tick from `track` via
// trailingWindowIndices so it always shows where the satellite has actually just been.

const SVG_NS = 'http://www.w3.org/2000/svg';
const MARGIN = { top: 14, right: 14, bottom: 22, left: 34 };
const MARKER_THROTTLE_MS = 100; // ~10 Hz, per the animated-marker DOM-churn budget
const RECENT_WINDOW_REVS = 1.2;

export interface LatLon { lat: number; lon: number }

/** xyz (km, Moon-centered rotating frame) to sub-satellite lat/lon in degrees. */
export function xyzToLatLon(x: number, y: number, z: number): LatLon {
  const r = Math.sqrt(x * x + y * y + z * z);
  const lat = r > 0 ? Math.asin(Math.min(1, Math.max(-1, z / r))) * (180 / Math.PI) : 0;
  const lonRaw = Math.atan2(y, x) * (180 / Math.PI) - 180;
  const lon = (((lonRaw + 180) % 360) + 360) % 360 - 180;
  return { lat, lon };
}

/** Sub-satellite ground track for every sample of a closed trajectory. */
export function groundTrack(traj: Float32Array): LatLon[] {
  const n = Math.floor(traj.length / 3);
  const out: LatLon[] = [];
  for (let i = 0; i < n; i++) {
    out.push(xyzToLatLon(traj[i * 3], traj[i * 3 + 1], traj[i * 3 + 2]));
  }
  return out;
}

/** Splits a point sequence wherever consecutive longitudes cross the ±180° seam. */
export function splitAtWraps<T extends LatLon>(points: T[]): T[][] {
  if (points.length === 0) return [];
  const out: T[][] = [];
  let current: T[] = [points[0]];
  for (let i = 1; i < points.length; i++) {
    if (Math.abs(points[i].lon - points[i - 1].lon) > 180) {
      out.push(current);
      current = [];
    }
    current.push(points[i]);
  }
  out.push(current);
  return out;
}

/** Latitude axis domain: the track's lat range padded ±10°, clamped to the physical ±90°. */
export function latDomain(points: LatLon[]): [number, number] {
  if (points.length === 0) return [-90, 90];
  const lats = points.map((p) => p.lat);
  const lo = Math.max(-90, Math.min(...lats) - 10);
  const hi = Math.min(90, Math.max(...lats) + 10);
  return lo < hi ? [lo, hi] : [-90, 90];
}

export interface MoonTrackPanel {
  setMember(traj: Float32Array | null, periodS: number, resonanceN: number): void;
  setAnimTime(t: number): void;
  refresh(): void;
}

export function mountMoonTrackPanel(container: HTMLElement): MoonTrackPanel {
  container.innerHTML = '';
  const svg = document.createElementNS(SVG_NS, 'svg');
  svg.setAttribute('class', 'groundtrack-svg');
  container.appendChild(svg);

  const el = (name: string, attrs: Record<string, string>): SVGElement => {
    const node = document.createElementNS(SVG_NS, name) as SVGElement;
    for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
    return node;
  };
  const text = (attrs: Record<string, string>, content: string): SVGElement => {
    const node = el('text', attrs);
    node.textContent = content;
    return node;
  };

  let traj: Float32Array | null = null;
  let periodS = 0;
  let resonanceN = 0;
  let track: LatLon[] = [];
  let animTimeS = 0;
  let lastMarkerDrawMs = 0;
  let markerEl: SVGElement | null = null;
  let recentGroupEl: SVGElement | null = null;
  let xScale: ScaleLinear<number, number> | null = null;
  let yScale: ScaleLinear<number, number> | null = null;

  /** Trailing window duration: ~1.2 revs, each rev being periodS/resonanceN long. */
  function recentWindowS(): number {
    return resonanceN > 0 ? (RECENT_WINDOW_REVS * periodS) / resonanceN : 0;
  }

  function positionMarker(): void {
    if (!traj || !xScale || !yScale || !markerEl || periodS <= 0) return;
    const [x, y, z] = samplePosition(traj, periodS, animTimeS);
    const { lat, lon } = xyzToLatLon(x, y, z);
    markerEl.setAttribute('cx', String(xScale(lon)));
    markerEl.setAttribute('cy', String(yScale(lat)));
  }

  function updateRecent(): void {
    if (!recentGroupEl || !xScale || !yScale || track.length === 0 || periodS <= 0) return;
    const x = xScale;
    const y = yScale;
    recentGroupEl.innerHTML = '';
    const idx = trailingWindowIndices(track.length, periodS, animTimeS, recentWindowS());
    const pts = idx.map((i) => track[i]);
    for (const seg of splitAtWraps(pts)) {
      if (seg.length < 2) continue;
      recentGroupEl.appendChild(el('polyline', {
        points: seg.map((p) => `${x(p.lon)},${y(p.lat)}`).join(' '),
        class: 'track-line-recent',
      }));
    }
  }

  function draw(): void {
    svg.innerHTML = '';
    markerEl = null;
    recentGroupEl = null;
    xScale = null;
    yScale = null;
    const w = container.clientWidth || 800;
    const h = container.clientHeight || 210;
    svg.setAttribute('width', String(w));
    svg.setAttribute('height', String(h));
    svg.setAttribute('viewBox', `0 0 ${w} ${h}`);
    if (!traj || track.length === 0) return;

    const iw = Math.max(10, w - MARGIN.left - MARGIN.right);
    const ih = Math.max(10, h - MARGIN.top - MARGIN.bottom);
    const [latLo, latHi] = latDomain(track);
    const x = scaleLinear().domain([-180, 180]).range([MARGIN.left, MARGIN.left + iw]);
    const y = scaleLinear().domain([latLo, latHi]).range([MARGIN.top + ih, MARGIN.top]);
    xScale = x;
    yScale = y;

    for (let lon = -180; lon <= 180; lon += 30) {
      svg.appendChild(el('line', {
        x1: String(x(lon)), x2: String(x(lon)), y1: String(MARGIN.top), y2: String(MARGIN.top + ih),
        class: 'grat',
      }));
    }
    for (let lat = Math.ceil(latLo / 30) * 30; lat <= latHi; lat += 30) {
      svg.appendChild(el('line', {
        x1: String(MARGIN.left), x2: String(MARGIN.left + iw), y1: String(y(lat)), y2: String(y(lat)),
        class: 'grat',
      }));
    }

    for (const seg of splitAtWraps(track)) {
      if (seg.length < 2) continue;
      const pts = seg.map((p) => `${x(p.lon)},${y(p.lat)}`).join(' ');
      svg.appendChild(el('polyline', { points: pts, class: 'track-line' }));
    }

    recentGroupEl = el('g', { class: 'track-recent-group' });
    svg.appendChild(recentGroupEl);

    svg.appendChild(el('circle', { cx: String(x(0)), cy: String(y(0)), r: '3', class: 'subearth-dot' }));
    svg.appendChild(text(
      { x: String(x(0) + 6), y: String(y(0) - 6), class: 'label' }, 'sub-Earth',
    ));
    svg.appendChild(text(
      { x: String(MARGIN.left), y: String(h - 6), class: 'tick' }, 'south pole toward −z (off-screen)',
    ));

    markerEl = el('circle', { cx: '0', cy: '0', r: '4', class: 'anim-marker' });
    svg.appendChild(markerEl);
    updateRecent();
    positionMarker();
  }

  window.addEventListener('resize', () => draw());

  return {
    setMember(t, p, n) {
      traj = t;
      periodS = p;
      resonanceN = n;
      track = t ? groundTrack(t) : [];
      draw();
    },
    setAnimTime(t) {
      animTimeS = t;
      const now = performance.now();
      if (now - lastMarkerDrawMs < MARKER_THROTTLE_MS) return;
      lastMarkerDrawMs = now;
      updateRecent();
      positionMarker();
    },
    refresh() {
      draw();
    },
  };
}
