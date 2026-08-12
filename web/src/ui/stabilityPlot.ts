import { scaleLinear } from 'd3-scale';
import { symlog } from '../state';
import type { Store } from '../state';
import type { Family } from '../types';

const SVG_NS = 'http://www.w3.org/2000/svg';
const MARGIN = { top: 14, right: 18, bottom: 26, left: 48 };

/** Symmetric symlog y-range that always contains the ±1 stability boundary. */
export function symlogDomain(nus: number[]): [number, number] {
  let m = 1.2;
  for (const v of nus) m = Math.max(m, Math.abs(symlog(v)) * 1.1);
  return [-m, m];
}

/** Plot-local x (px, already offset by the left margin) to a member index. */
export function indexFromX(px: number, plotWidth: number, count: number): number {
  if (count <= 1) return 0;
  const f = Math.min(1, Math.max(0, px / plotWidth));
  return Math.round(f * (count - 1));
}

export interface StabilityPlot {
  setFamily(family: Family): void;
  refresh(): void;
}

export function mountStabilityPlot(container: HTMLElement, store: Store): StabilityPlot {
  container.innerHTML = '';
  const svg = document.createElementNS(SVG_NS, 'svg');
  svg.setAttribute('class', 'stability-svg');
  container.appendChild(svg);

  let family: Family | null = null;

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

  function draw(): void {
    svg.innerHTML = '';
    if (!family) return;
    const w = container.clientWidth || 800;
    const h = container.clientHeight || 210;
    const iw = innerWidth();
    const ih = Math.max(10, h - MARGIN.top - MARGIN.bottom);
    svg.setAttribute('width', String(w));
    svg.setAttribute('height', String(h));
    svg.setAttribute('viewBox', `0 0 ${w} ${h}`);

    const members = family.members;
    const n = members.length;
    const domain = symlogDomain(members.flatMap((m) => [m.nu1, m.nu2]));
    const x = scaleLinear().domain([0, Math.max(1, n - 1)]).range([MARGIN.left, MARGIN.left + iw]);
    const y = scaleLinear().domain(domain).range([MARGIN.top + ih, MARGIN.top]);

    svg.appendChild(el('line', {
      x1: String(MARGIN.left), x2: String(MARGIN.left + iw),
      y1: String(y(0)), y2: String(y(0)), class: 'axis',
    }));
    for (const b of [1, -1]) {
      svg.appendChild(el('line', {
        x1: String(MARGIN.left), x2: String(MARGIN.left + iw),
        y1: String(y(b)), y2: String(y(b)), class: 'boundary',
      }));
    }

    const series = (pick: (i: number) => number, cls: string): void => {
      const pts = members.map((_, i) => `${x(i)},${y(symlog(pick(i)))}`).join(' ');
      svg.appendChild(el('polyline', { points: pts, class: cls }));
    };
    series((i) => members[i].nu1, 'nu1');
    series((i) => members[i].nu2, 'nu2');

    const idx = Math.min(Math.max(0, store.get().memberIndex), n - 1);
    svg.appendChild(el('line', {
      x1: String(x(idx)), x2: String(x(idx)),
      y1: String(MARGIN.top), y2: String(MARGIN.top + ih), class: 'cursor',
    }));

    for (const [v, label] of [[1, '+1'], [0, '0'], [-1, '−1']] as Array<[number, string]>) {
      svg.appendChild(text(
        { x: String(MARGIN.left - 8), y: String(y(v) + 4), class: 'tick', 'text-anchor': 'end' },
        label,
      ));
    }
    svg.appendChild(text({ x: String(MARGIN.left), y: String(h - 8), class: 'tick' }, 'member 0'));
    svg.appendChild(text(
      { x: String(MARGIN.left + iw), y: String(h - 8), class: 'tick', 'text-anchor': 'end' },
      `member ${n - 1}`,
    ));
    svg.appendChild(text(
      { x: String(MARGIN.left + 6), y: String(MARGIN.top + 12), class: 'legend' },
      'symlog ν₁ (amber) / ν₂ (teal) — |ν| ≤ 1 is linearly stable',
    ));
  }

  let dragging = false;
  const pickMember = (ev: MouseEvent): void => {
    if (!family) return;
    const rect = svg.getBoundingClientRect();
    const iw = Math.max(10, rect.width - MARGIN.left - MARGIN.right);
    const idx = indexFromX(ev.clientX - rect.left - MARGIN.left, iw, family.members.length);
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
    setFamily(f) {
      family = f;
      draw();
    },
    refresh: draw,
  };
}
