import { scaleLinear } from 'd3-scale';
import type { ScaleLinear } from 'd3-scale';
import { MOON_RADIUS_KM } from '../scene';
import { samplePosition, trailingWindowIndices } from '../state';

// Earth plane-of-sky panel: the satellite's apparent track against the Moon's disk as seen
// from Earth (viewer at −x looking toward +x). Purely driven from the outside via setMember /
// setAnimTime — no store subscription of its own (see bottomTabs.ts, which owns that wiring).
//
// The full closed track (N revs, N = resonanceN) is drawn at low prominence — for members with
// many revs it reads as clutter — while a bright "recent" polyline covers the trailing ~1.2
// revs ending at the animated marker, rebuilt each throttled tick from `points` via
// trailingWindowIndices. The recent window keeps the clear/transit/occulted classification
// coloring, same as the dim full track.

const SVG_NS = 'http://www.w3.org/2000/svg';
const MARGIN = { top: 14, right: 14, bottom: 22, left: 14 };
const MARKER_THROTTLE_MS = 100; // ~10 Hz, per the animated-marker DOM-churn budget
const RECENT_WINDOW_REVS = 1.2;

export type SkyClass = 'occulted' | 'transit' | 'clear';

/**
 * Classifies one sample against the Moon's disk as seen from Earth: inside the disk's
 * angular radius (small-angle: linear km, not angle) and behind it (x > 0) is a comm
 * outage; inside and in front (x <= 0) is a transit across the disk; otherwise clear.
 */
export function skyClassify(x: number, y: number, z: number): SkyClass {
  const rho = Math.sqrt(y * y + z * z);
  if (rho < MOON_RADIUS_KM) return x > 0 ? 'occulted' : 'transit';
  return 'clear';
}

/** Plane-of-sky projection: mirror y (viewer at −x looks toward +x), z stays vertical. */
export function skyProject(_x: number, y: number, z: number): { x: number; y: number } {
  return { x: -y, y: z };
}

/** Fraction of a uniformly time-sampled period the satellite spends occulted. */
export function occultedFraction(traj: Float32Array): number {
  const n = Math.floor(traj.length / 3);
  if (n === 0) return 0;
  let count = 0;
  for (let i = 0; i < n; i++) {
    if (skyClassify(traj[i * 3], traj[i * 3 + 1], traj[i * 3 + 2]) === 'occulted') count++;
  }
  return count / n;
}

interface SkyPoint { x: number; y: number; cls: SkyClass }

function skyPoints(traj: Float32Array): SkyPoint[] {
  const n = Math.floor(traj.length / 3);
  const out: SkyPoint[] = [];
  for (let i = 0; i < n; i++) {
    const x = traj[i * 3];
    const y = traj[i * 3 + 1];
    const z = traj[i * 3 + 2];
    const p = skyProject(x, y, z);
    out.push({ x: p.x, y: p.y, cls: skyClassify(x, y, z) });
  }
  return out;
}

/** Splits a point sequence into runs of matching sky classification (shared boundary vertex). */
function segmentsByClass(points: SkyPoint[]): SkyPoint[][] {
  if (points.length === 0) return [];
  const out: SkyPoint[][] = [];
  let current: SkyPoint[] = [points[0]];
  for (let i = 1; i < points.length; i++) {
    if (points[i].cls !== points[i - 1].cls) {
      current.push(points[i]); // shared vertex: segments touch, no visual gap
      out.push(current);
      current = [points[i]];
    } else {
      current.push(points[i]);
    }
  }
  out.push(current);
  return out;
}

export interface SkyViewPanel {
  setMember(traj: Float32Array | null, periodS: number, resonanceN: number): void;
  setAnimTime(t: number): void;
  refresh(): void;
}

export function mountSkyViewPanel(container: HTMLElement): SkyViewPanel {
  container.innerHTML = '';
  const svg = document.createElementNS(SVG_NS, 'svg');
  svg.setAttribute('class', 'skyview-svg');
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
  let points: SkyPoint[] = [];
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
    const sp = skyProject(x, y, z);
    markerEl.setAttribute('cx', String(xScale(sp.x)));
    markerEl.setAttribute('cy', String(yScale(sp.y)));
  }

  function updateRecent(): void {
    if (!recentGroupEl || !xScale || !yScale || points.length === 0 || periodS <= 0) return;
    const x = xScale;
    const y = yScale;
    recentGroupEl.innerHTML = '';
    const idx = trailingWindowIndices(points.length, periodS, animTimeS, recentWindowS());
    const recentPoints = idx.map((i) => points[i]);
    for (const seg of segmentsByClass(recentPoints)) {
      if (seg.length < 2) continue;
      const pts = seg.map((p) => `${x(p.x)},${y(p.y)}`).join(' ');
      recentGroupEl.appendChild(el('polyline', {
        points: pts, class: `sky-${seg[seg.length - 1].cls} recent`,
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
    if (!traj || points.length === 0) return;

    const iw = Math.max(10, w - MARGIN.left - MARGIN.right);
    const ih = Math.max(10, h - MARGIN.top - MARGIN.bottom);

    let maxAbs = MOON_RADIUS_KM;
    for (const p of points) maxAbs = Math.max(maxAbs, Math.abs(p.x), Math.abs(p.y));
    const half = maxAbs * 1.15;
    const size = Math.max(10, Math.min(iw, ih));
    const cx = MARGIN.left + iw / 2;
    const cy = MARGIN.top + ih / 2;
    const x = scaleLinear().domain([-half, half]).range([cx - size / 2, cx + size / 2]);
    const y = scaleLinear().domain([-half, half]).range([cy + size / 2, cy - size / 2]);
    xScale = x;
    yScale = y;

    svg.appendChild(el('circle', {
      cx: String(x(0)), cy: String(y(0)), r: String(x(MOON_RADIUS_KM) - x(0)), class: 'moon-disk',
    }));

    for (const seg of segmentsByClass(points)) {
      if (seg.length < 2) continue;
      const pts = seg.map((p) => `${x(p.x)},${y(p.y)}`).join(' ');
      svg.appendChild(el('polyline', { points: pts, class: `sky-${seg[seg.length - 1].cls}` }));
    }

    recentGroupEl = el('g', { class: 'sky-recent-group' });
    svg.appendChild(recentGroupEl);

    for (const t of x.ticks(5)) {
      if (t === 0) continue;
      svg.appendChild(text(
        { x: String(x(t)), y: String(h - 6), class: 'tick', 'text-anchor': 'middle' },
        (t / 1000).toFixed(0),
      ));
    }
    svg.appendChild(text(
      { x: String(MARGIN.left + iw), y: String(h - 6), class: 'tick', 'text-anchor': 'end' },
      'axes ×10³ km',
    ));

    const frac = occultedFraction(traj);
    svg.appendChild(text(
      { x: String(MARGIN.left + 4), y: String(MARGIN.top + 10), class: 'legend' },
      `outage ${(frac * 100).toFixed(1)}% of period`,
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
      points = t ? skyPoints(t) : [];
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
