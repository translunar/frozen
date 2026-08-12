import { scaleLinear } from 'd3-scale';
import { MOON_RADIUS_KM } from '../scene';
import { energyNd, librationPeriodMonths, stabilityMargin, symlog, windingAngleDeg } from '../state';
import type { Store } from '../state';
import type { Family, Member, Terms } from '../types';

// This strip is a family metric strip: an x-axis walking a family's members, positioned by
// their nondimensional energy (the Jacobi-like integral of motion, in the active combo's
// force model) rather than an abstract member index — spacing along the axis is now
// physically meaningful. A chosen metric is selectable in the top-right <select>. The
// file/export names stay `stabilityPlot` / `StabilityPlot` for API stability; only wording
// inside the module has moved on.

const SVG_NS = 'http://www.w3.org/2000/svg';
const MARGIN = { top: 14, right: 18, bottom: 46, left: 48 };

/** Symmetric symlog y-range that always contains the ±1 stability boundary. */
export function symlogDomain(nus: number[]): [number, number] {
  let m = 1.2;
  for (const v of nus) m = Math.max(m, Math.abs(symlog(v)) * 1.1);
  return [-m, m];
}

/** Auto-fit linear y-range with 10% padding; widens a degenerate (all-equal) domain. */
export function linearDomain(values: number[]): [number, number] {
  if (values.length === 0) return [0, 1];
  let lo = Math.min(...values);
  let hi = Math.max(...values);
  if (lo === hi) {
    lo -= 1;
    hi += 1;
  }
  const pad = (hi - lo) * 0.1;
  return [lo - pad, hi + pad];
}

/** Whole-decade exponent bounds around the data, for a log10 axis. Floors before logging. */
export function log10Domain(values: number[]): [number, number] {
  const logs = values.map((v) => Math.log10(Math.max(1e-6, v)));
  let lo = Math.min(...logs);
  let hi = Math.max(...logs);
  if (lo === hi) {
    lo -= 1;
    hi += 1;
  }
  return [Math.floor(lo), Math.ceil(hi)];
}

const SUPERSCRIPT: Record<string, string> = {
  '0': '⁰', '1': '¹', '2': '²', '3': '³', '4': '⁴',
  '5': '⁵', '6': '⁶', '7': '⁷', '8': '⁸', '9': '⁹', '-': '⁻',
};

/** Formats an exponent as a unicode-superscript power of ten, e.g. -1 -> "10⁻¹". */
export function log10TickLabel(exponent: number): string {
  const digits = String(exponent).split('').map((c) => SUPERSCRIPT[c] ?? c).join('');
  return `10${digits}`;
}

/** Plot-local x (px, already offset by the left margin) to a member index. */
export function indexFromX(px: number, plotWidth: number, count: number): number {
  if (count <= 1) return 0;
  const f = Math.min(1, Math.max(0, px / plotWidth));
  return Math.round(f * (count - 1));
}

/** The member whose energy is closest to `target` (ties keep the earlier index). */
export function memberIndexFromEnergy(energies: number[], target: number): number {
  if (energies.length === 0) return 0;
  let best = 0;
  let bestDiff = Math.abs(energies[0] - target);
  for (let i = 1; i < energies.length; i++) {
    const diff = Math.abs(energies[i] - target);
    if (diff < bestDiff) {
      bestDiff = diff;
      best = i;
    }
  }
  return best;
}

/**
 * Engineering-notation tick label for an energy value, expressed as an offset from the
 * family-minimum energy E0: `E₀ + 3.2e-4` (or `E₀ − ...` below E0, `E₀` exactly at it). The
 * energy span across a family is tiny relative to |E| ~ 1.8, so absolute values would print
 * as indistinguishable digits — the offset is where the shape actually lives.
 */
export function formatEnergyOffset(offset: number): string {
  if (offset === 0) return 'E₀';
  const sign = offset > 0 ? '+' : '−';
  return `E₀ ${sign} ${Math.abs(offset).toExponential(1)}`;
}

/** The `E₀ = ...` legend line noting the family-minimum energy the tick offsets are from. */
export function energyLegendLabel(e0: number): string {
  const sign = e0 < 0 ? '−' : '';
  return `E₀ = ${sign}${Math.abs(e0).toFixed(5)}`;
}

export type MetricKey = 'winding' | 'margin' | 'raw' | 'peri' | 'apo' | 'ecc' | 'inc';
export type SingleMetricKey = Exclude<MetricKey, 'raw'>;

interface MetricOption { key: MetricKey; label: string }

const METRIC_OPTIONS: MetricOption[] = [
  { key: 'winding', label: 'Winding angle θ (deg)' },
  { key: 'margin', label: 'Stability margin 1−|ν₂| (log10)' },
  { key: 'raw', label: 'Raw ν₁/ν₂ (symlog)' },
  { key: 'peri', label: 'Periapsis alt (km)' },
  { key: 'apo', label: 'Apoapsis alt (km)' },
  { key: 'ecc', label: 'Eccentricity' },
  { key: 'inc', label: 'Inclination (deg)' },
];

/** A single-series metric's value for one member (everything except the two-series raw ν). */
export function singleMetricValue(key: SingleMetricKey, member: Member): number {
  switch (key) {
    case 'winding': return windingAngleDeg(member.nu2);
    case 'margin': return stabilityMargin(member.nu2);
    case 'peri': return member.r_peri_km - MOON_RADIUS_KM;
    case 'apo': return member.r_apo_km - MOON_RADIUS_KM;
    case 'ecc': return member.elements.e;
    case 'inc': return member.elements.i_deg;
    default: return 0;
  }
}

function formatLinearTick(key: SingleMetricKey, v: number): string {
  switch (key) {
    case 'winding': return `${v.toFixed(0)}°`;
    case 'peri':
    case 'apo': return v.toFixed(0);
    case 'ecc': return v.toFixed(2);
    case 'inc': return `${v.toFixed(0)}°`;
    default: return v.toFixed(2);
  }
}

/** Cursor/legend readout for the active metric at one member; θ also shows the libration period. */
export function metricCursorText(key: MetricKey, member: Member): string {
  switch (key) {
    case 'winding': {
      const theta = windingAngleDeg(member.nu2);
      const months = librationPeriodMonths(member.nu2, member.period_s);
      const lib = Number.isFinite(months)
        ? `libration ≈ ${months.toFixed(1)} months`
        : 'no libration (θ = 0°)';
      return `θ = ${theta.toFixed(1)}° · ${lib}`;
    }
    case 'margin':
      return `1−|ν₂| = ${stabilityMargin(member.nu2).toExponential(2)}`;
    case 'raw':
      return `ν₁ = ${member.nu1.toFixed(3)} · ν₂ = ${member.nu2.toFixed(3)}`;
    case 'peri':
      return `peri alt = ${(member.r_peri_km - MOON_RADIUS_KM).toFixed(0)} km`;
    case 'apo':
      return `apo alt = ${(member.r_apo_km - MOON_RADIUS_KM).toFixed(0)} km`;
    case 'ecc':
      return `e = ${member.elements.e.toFixed(4)}`;
    case 'inc':
      return `i = ${member.elements.i_deg.toFixed(2)}°`;
    default:
      return '';
  }
}

export interface StabilityPlot {
  setFamily(family: Family, terms: Terms): void;
  refresh(): void;
}

export function mountStabilityPlot(container: HTMLElement, store: Store): StabilityPlot {
  container.innerHTML = '';
  const svg = document.createElementNS(SVG_NS, 'svg');
  svg.setAttribute('class', 'stability-svg');
  container.appendChild(svg);

  const select = document.createElement('select');
  select.className = 'metric-select';
  for (const opt of METRIC_OPTIONS) {
    const o = document.createElement('option');
    o.value = opt.key;
    o.textContent = opt.label;
    select.appendChild(o);
  }
  container.appendChild(select);

  let family: Family | null = null;
  let terms: Terms | null = null;
  let activeMetric: MetricKey = 'winding';
  select.value = activeMetric;
  select.addEventListener('change', () => {
    activeMetric = select.value as MetricKey;
    draw();
  });

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

  const innerWidth = (): number =>
    Math.max(10, (container.clientWidth || 800) - MARGIN.left - MARGIN.right);

  /** Family members' energies in the active combo's force model, in member order (unsorted —
   * a non-monotone (folded) family plots honestly, doubling back rather than being reordered). */
  const energiesOf = (fam: Family, t: Terms): number[] =>
    fam.members.map((m) => energyNd(m.state0, t));

  /** The raw ν₁/ν₂ symlog view: two polylines, sample dots, and the ±1 stability boundary. */
  function drawRaw(members: Member[], n: number, xAt: (i: number) => number, ih: number): void {
    const domain = symlogDomain(members.flatMap((m) => [m.nu1, m.nu2]));
    const y = scaleLinear().domain(domain).range([MARGIN.top + ih, MARGIN.top]);

    svg.appendChild(el('line', {
      x1: String(MARGIN.left), x2: String(MARGIN.left + innerWidth()),
      y1: String(y(0)), y2: String(y(0)), class: 'axis',
    }));
    for (const b of [1, -1]) {
      svg.appendChild(el('line', {
        x1: String(MARGIN.left), x2: String(MARGIN.left + innerWidth()),
        y1: String(y(b)), y2: String(y(b)), class: 'boundary',
      }));
    }

    const series = (pick: (i: number) => number, cls: string): void => {
      const pts = members.map((_, i) => `${xAt(i)},${y(symlog(pick(i)))}`).join(' ');
      svg.appendChild(el('polyline', { points: pts, class: cls }));
      for (let i = 0; i < n; i++) {
        svg.appendChild(el('circle', {
          cx: String(xAt(i)), cy: String(y(symlog(pick(i)))), r: '2', class: `${cls}-dot`,
        }));
      }
    };
    series((i) => members[i].nu1, 'nu1');
    series((i) => members[i].nu2, 'nu2');

    for (const [v, label] of [[1, '+1'], [0, '0'], [-1, '−1']] as Array<[number, string]>) {
      svg.appendChild(text(
        { x: String(MARGIN.left - 8), y: String(y(v) + 4), class: 'tick', 'text-anchor': 'end' },
        label,
      ));
    }
  }

  /** Any single-series metric: one polyline + sample dots, linear or log10 y-axis. */
  function drawSingle(
    key: SingleMetricKey, members: Member[], n: number, xAt: (i: number) => number, ih: number,
  ): void {
    const values = members.map((m) => singleMetricValue(key, m));
    const isLog = key === 'margin';
    const [domLo, domHi] = isLog ? log10Domain(values) : linearDomain(values);
    const y = scaleLinear().domain([domLo, domHi]).range([MARGIN.top + ih, MARGIN.top]);
    const yOf = (v: number): number => (isLog ? y(Math.log10(Math.max(1e-6, v))) : y(v));

    const pts = members.map((_, i) => `${xAt(i)},${yOf(values[i])}`).join(' ');
    svg.appendChild(el('polyline', { points: pts, class: 'metric-line' }));
    for (let i = 0; i < n; i++) {
      svg.appendChild(el('circle', { cx: String(xAt(i)), cy: String(yOf(values[i])), r: '2', class: 'metric-dot' }));
    }

    if (isLog) {
      for (let e = domLo; e <= domHi; e++) {
        svg.appendChild(text(
          { x: String(MARGIN.left - 8), y: String(y(e) + 4), class: 'tick', 'text-anchor': 'end' },
          log10TickLabel(e),
        ));
      }
    } else {
      for (const t of y.ticks(4)) {
        svg.appendChild(text(
          { x: String(MARGIN.left - 8), y: String(y(t) + 4), class: 'tick', 'text-anchor': 'end' },
          formatLinearTick(key, t),
        ));
      }
    }
  }

  function draw(): void {
    svg.innerHTML = '';
    if (!family || !terms) return;
    const w = container.clientWidth || 800;
    const h = container.clientHeight || 210;
    const iw = innerWidth();
    const ih = Math.max(10, h - MARGIN.top - MARGIN.bottom);
    svg.setAttribute('width', String(w));
    svg.setAttribute('height', String(h));
    svg.setAttribute('viewBox', `0 0 ${w} ${h}`);

    const members = family.members;
    const n = members.length;
    const energies = energiesOf(family, terms);
    const [eLo, eHi] = linearDomain(energies);
    const x = scaleLinear().domain([eLo, eHi]).range([MARGIN.left, MARGIN.left + iw]);
    const xAt = (i: number): number => x(energies[i]);

    if (activeMetric === 'raw') {
      drawRaw(members, n, xAt, ih);
    } else {
      drawSingle(activeMetric, members, n, xAt, ih);
    }

    const idx = Math.min(Math.max(0, store.get().memberIndex), n - 1);
    svg.appendChild(el('line', {
      x1: String(xAt(idx)), x2: String(xAt(idx)),
      y1: String(MARGIN.top), y2: String(MARGIN.top + ih), class: 'cursor',
    }));

    // Energy axis: title, then tick labels as offsets from the family-minimum energy E0
    // (the raw values are indistinguishable at |E| ~ 1.8 — the offset is where the shape is).
    svg.appendChild(text(
      { x: String(MARGIN.left + iw / 2), y: String(h - 34), class: 'axis-label', 'text-anchor': 'middle' },
      'energy (nondim)',
    ));
    const e0 = Math.min(...energies);
    for (const t of x.ticks(4)) {
      svg.appendChild(text(
        { x: String(x(t)), y: String(h - 20), class: 'tick', 'text-anchor': 'middle' },
        formatEnergyOffset(t - e0),
      ));
    }

    // Endpoint hp labels stay as a secondary annotation under the energy axis — the x-axis
    // itself no longer walks members in order, so these anchor the family's two shooting
    // endpoints rather than implying "leftmost"/"rightmost" member.
    const hpOf = (m: Member): string => (m.r_peri_km - MOON_RADIUS_KM).toFixed(0);
    svg.appendChild(text(
      { x: String(MARGIN.left), y: String(h - 6), class: 'tick' },
      `hp ${hpOf(members[0])} km`,
    ));
    svg.appendChild(text(
      { x: String(MARGIN.left + iw), y: String(h - 6), class: 'tick', 'text-anchor': 'end' },
      `hp ${hpOf(members[n - 1])} km`,
    ));

    svg.appendChild(text(
      { x: String(MARGIN.left + 6), y: String(MARGIN.top + 12), class: 'legend' },
      metricCursorText(activeMetric, members[idx]),
    ));
    svg.appendChild(text(
      { x: String(MARGIN.left + 6), y: String(MARGIN.top + 26), class: 'legend-e0' },
      energyLegendLabel(e0),
    ));
  }

  let dragging = false;
  const pickMember = (ev: MouseEvent): void => {
    if (!family || !terms) return;
    const rect = svg.getBoundingClientRect();
    const iw = Math.max(10, rect.width - MARGIN.left - MARGIN.right);
    const energies = energiesOf(family, terms);
    const [eLo, eHi] = linearDomain(energies);
    const x = scaleLinear().domain([eLo, eHi]).range([MARGIN.left, MARGIN.left + iw]);
    const targetEnergy = x.invert(ev.clientX - rect.left);
    const idx = memberIndexFromEnergy(energies, targetEnergy);
    if (idx !== store.get().memberIndex) store.update({ memberIndex: idx, animTime: 0 });
  };

  svg.addEventListener('mousedown', (ev) => {
    dragging = true;
    pickMember(ev);
  });
  window.addEventListener('mousemove', (ev) => {
    if (dragging) pickMember(ev);
  });
  window.addEventListener('mouseup', () => {
    dragging = false;
  });
  window.addEventListener('resize', () => draw());
  store.subscribe((s, p) => {
    if (s.memberIndex !== p.memberIndex || s.familyN !== p.familyN || s.comboId !== p.comboId) draw();
  });

  return {
    setFamily(f, t) {
      family = f;
      terms = t;
      draw();
    },
    refresh: draw,
  };
}
