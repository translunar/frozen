import { scaleLinear } from 'd3-scale';
import { MOON_RADIUS_KM } from '../scene';
import {
  displayOrder, energyNd, librationPeriodMonths, stabilityMargin, symlog, windingAngleDeg,
} from '../state';
import type { Store } from '../state';
import type { Family, Member, Terms } from '../types';

// This strip is a family metric strip: an x-axis walking a family's members in *display
// order* (see state.ts's displayOrder — periapsis-altitude-sorted, since the catalog's raw
// continuation-walk order can zigzag/backtrack near a degenerate step), positioned by one of
// several physically meaningful x-axis choices (periapsis/apoapsis altitude, eccentricity, or
// nondim energy — selectable in a second top-right <select>). A chosen y-metric is selectable
// in the first top-right <select>. The file/export names stay `stabilityPlot` /
// `StabilityPlot` for API stability; only wording inside the module has moved on.

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

/** The rank whose value is closest to `target` (ties keep the earlier rank). Generic over any
 * numeric series — used for both the energy and the periapsis/apoapsis/eccentricity axes. */
export function memberIndexFromEnergy(values: number[], target: number): number {
  if (values.length === 0) return 0;
  let best = 0;
  let bestDiff = Math.abs(values[0] - target);
  for (let i = 1; i < values.length; i++) {
    const diff = Math.abs(values[i] - target);
    if (diff < bestDiff) {
      bestDiff = diff;
      best = i;
    }
  }
  return best;
}

/**
 * `order`, reversed if needed, so `values` (indexed by true member index — one entry per
 * `order` slot) reads ascending left-to-right. Only the two endpoints decide direction: the
 * base order (periapsis-altitude-sorted) is never independently re-sorted by `values` itself,
 * so an axis choice that moves in the opposite sense along the same hp-anchored walk (h_a
 * often falls as h_p rises) still lands in a single global orientation rather than being
 * resorted point-by-point, which would undo the zigzag fix `displayOrder` exists for.
 */
export function orientedOrder(order: number[], values: number[]): number[] {
  if (order.length < 2) return order;
  const first = values[order[0]];
  const last = values[order[order.length - 1]];
  return first <= last ? order : [...order].reverse();
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

/** X-axis tick count: thins from 4 to 2 when the pane is too narrow to fit four labels
 * without overlap (the three-pane always-visible layout can leave this strip quite narrow). */
export function xTickCount(width: number): number {
  return width < 380 ? 2 : 4;
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

export type XAxisKey = 'hp' | 'ha' | 'ecc' | 'energy';

interface XAxisOption { key: XAxisKey; label: string }

const X_AXIS_OPTIONS: XAxisOption[] = [
  { key: 'hp', label: 'h_p (km)' },
  { key: 'ha', label: 'h_a (km)' },
  { key: 'ecc', label: 'eccentricity' },
  { key: 'energy', label: 'energy (near-degenerate for resonant families)' },
];

/** Per-member value for the chosen x-axis, indexed by TRUE member index (raw storage order —
 * callers reindex through `displayOrder`/`orientedOrder` for plotting). */
export function xAxisValues(family: Family, mode: XAxisKey, terms: Terms): number[] {
  switch (mode) {
    case 'hp': return family.members.map((m) => m.r_peri_km - MOON_RADIUS_KM);
    case 'ha': return family.members.map((m) => m.r_apo_km - MOON_RADIUS_KM);
    case 'ecc': return family.members.map((m) => m.elements.e);
    case 'energy': return family.members.map((m) => energyNd(m.state0, terms));
    default: return [];
  }
}

function xAxisTitle(mode: XAxisKey): string {
  switch (mode) {
    case 'hp': return 'peri alt (km)';
    case 'ha': return 'apo alt (km)';
    case 'ecc': return 'eccentricity';
    case 'energy': return 'energy (nondim)';
    default: return '';
  }
}

function formatXTick(mode: XAxisKey, v: number, e0: number): string {
  switch (mode) {
    case 'hp':
    case 'ha': return `${Math.round(v)} km`;
    case 'ecc': return v.toFixed(2);
    case 'energy': return formatEnergyOffset(v - e0);
    default: return String(v);
  }
}

export interface StabilityPlot {
  setFamily(family: Family, terms: Terms): void;
  refresh(): void;
}

/**
 * `headerAside`, when given, is an external slot (the pane header's right-aligned area in the
 * bottom three-pane layout) to host the two `<select>`s instead of floating them over the SVG.
 * Falls back to appending inside `container` so the plot stays usable when mounted standalone.
 */
export function mountStabilityPlot(
  container: HTMLElement, store: Store, headerAside?: HTMLElement,
): StabilityPlot {
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
  (headerAside ?? container).appendChild(select);

  const xSelect = document.createElement('select');
  xSelect.className = 'metric-select';
  for (const opt of X_AXIS_OPTIONS) {
    const o = document.createElement('option');
    o.value = opt.key;
    o.textContent = opt.label;
    xSelect.appendChild(o);
  }
  (headerAside ?? container).appendChild(xSelect);

  let family: Family | null = null;
  let terms: Terms | null = null;
  let activeMetric: MetricKey = 'winding';
  let activeXAxis: XAxisKey = 'hp';
  select.value = activeMetric;
  xSelect.value = activeXAxis;
  select.addEventListener('change', () => {
    activeMetric = select.value as MetricKey;
    draw();
  });
  xSelect.addEventListener('change', () => {
    activeXAxis = xSelect.value as XAxisKey;
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

  /** Display-rank order for the active x-axis: hp-sorted, then oriented (possibly reversed as
   * a whole) so the chosen axis variable itself reads ascending left-to-right. */
  const orderFor = (fam: Family, mode: XAxisKey, t: Terms): { order: number[]; values: number[] } => {
    const base = displayOrder(fam);
    const values = xAxisValues(fam, mode, t);
    return { order: orientedOrder(base, values), values };
  };

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
    const fam = family;
    const w = container.clientWidth || 800;
    const h = container.clientHeight || 230;
    const iw = innerWidth();
    const ih = Math.max(10, h - MARGIN.top - MARGIN.bottom);
    svg.setAttribute('width', String(w));
    svg.setAttribute('height', String(h));
    svg.setAttribute('viewBox', `0 0 ${w} ${h}`);

    const n = fam.members.length;
    const hpOrder = displayOrder(fam);
    const { order, values } = orderFor(fam, activeXAxis, terms);
    const orderedMembers = order.map((idx) => fam.members[idx]);
    const orderedValues = order.map((idx) => values[idx]);
    const [xLo, xHi] = linearDomain(orderedValues);
    const x = scaleLinear().domain([xLo, xHi]).range([MARGIN.left, MARGIN.left + iw]);
    const xAt = (rank: number): number => x(orderedValues[rank]);

    if (activeMetric === 'raw') {
      drawRaw(orderedMembers, n, xAt, ih);
    } else {
      drawSingle(activeMetric, orderedMembers, n, xAt, ih);
    }

    const trueIdx = Math.min(Math.max(0, store.get().memberIndex), n - 1);
    const cursorRank = Math.max(0, order.indexOf(trueIdx));
    svg.appendChild(el('line', {
      x1: String(xAt(cursorRank)), x2: String(xAt(cursorRank)),
      y1: String(MARGIN.top), y2: String(MARGIN.top + ih), class: 'cursor',
    }));

    // X-axis: title, then tick labels — energy uses engineering-offset-from-E0 notation (its
    // span is tiny relative to |E| ~ 1.8); the others are plain values in their own units.
    svg.appendChild(text(
      { x: String(MARGIN.left + iw / 2), y: String(h - 34), class: 'axis-label', 'text-anchor': 'middle' },
      xAxisTitle(activeXAxis),
    ));
    const e0 = Math.min(...orderedValues);
    for (const t of x.ticks(xTickCount(w))) {
      svg.appendChild(text(
        { x: String(x(t)), y: String(h - 20), class: 'tick', 'text-anchor': 'middle' },
        formatXTick(activeXAxis, t, e0),
      ));
    }

    // Endpoint hp labels stay as a secondary annotation under the x-axis regardless of which
    // axis is active — display order is periapsis-altitude-sorted, so these are always the
    // family's true min/max hp members now (not just its first/last shooting endpoints).
    const hpOf = (m: Member): string => (m.r_peri_km - MOON_RADIUS_KM).toFixed(0);
    svg.appendChild(text(
      { x: String(MARGIN.left), y: String(h - 6), class: 'tick' },
      `hp ${hpOf(fam.members[hpOrder[0]])} km`,
    ));
    svg.appendChild(text(
      { x: String(MARGIN.left + iw), y: String(h - 6), class: 'tick', 'text-anchor': 'end' },
      `hp ${hpOf(fam.members[hpOrder[n - 1]])} km`,
    ));

    svg.appendChild(text(
      { x: String(MARGIN.left + 6), y: String(MARGIN.top + 12), class: 'legend' },
      metricCursorText(activeMetric, fam.members[trueIdx]),
    ));
    if (activeXAxis === 'energy') {
      svg.appendChild(text(
        { x: String(MARGIN.left + 6), y: String(MARGIN.top + 26), class: 'legend-e0' },
        energyLegendLabel(e0),
      ));
    }
  }

  let dragging = false;
  const pickMember = (ev: MouseEvent): void => {
    if (!family || !terms) return;
    const fam = family;
    const rect = svg.getBoundingClientRect();
    const iw = Math.max(10, rect.width - MARGIN.left - MARGIN.right);
    const { order, values } = orderFor(fam, activeXAxis, terms);
    const orderedValues = order.map((idx) => values[idx]);
    const [xLo, xHi] = linearDomain(orderedValues);
    const x = scaleLinear().domain([xLo, xHi]).range([MARGIN.left, MARGIN.left + iw]);
    const target = x.invert(ev.clientX - rect.left);
    const rank = memberIndexFromEnergy(orderedValues, target);
    const trueIdx = order[rank] ?? 0;
    if (trueIdx !== store.get().memberIndex) store.update({ memberIndex: trueIdx, animTime: 0 });
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
