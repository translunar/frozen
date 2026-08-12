import type { Store } from '../state';
import type { Family, Member, Terms } from '../types';
import { mountMoonTrackPanel } from './groundTrack';
import { mountSkyViewPanel, occultedFraction } from './skyView';
import { mountStabilityPlot } from './stabilityPlot';

export type PaneKey = 'metrics' | 'moon' | 'earth';

const PANES: PaneKey[] = ['metrics', 'moon', 'earth'];
const PANE_TITLES: Record<PaneKey, string> = {
  metrics: 'Family metrics',
  moon: 'Moon ground track',
  earth: 'Earth view',
};
// Matches the grid's default `1.2fr 1.2fr 0.8fr` — sky view is squarer and needs less width.
const PANE_FR: Record<PaneKey, number> = { metrics: 1.2, moon: 1.2, earth: 0.8 };
const COLLAPSED_WIDTH = '28px';

export interface BottomPanel {
  setFamily(family: Family, terms: Terms): void;
  setMember(traj: Float32Array | null, member: Member, resonanceN: number): void;
  setAnimTime(t: number): void;
  refresh(): void;
}

/**
 * Three-pane bottom section, all panes always visible side by side: Family metrics, Moon
 * ground track, Earth plane-of-sky view. Previously these were tabbed (one super-expanded
 * pane at a time); a `display:none` pane can't be measured or drawn into, so switching tabs
 * had to force a redraw of the newly-shown pane. With all three panes always in the document
 * flow that workaround is gone — each panel just re-measures its own container on mount,
 * on `setMember`/`setFamily`, and on window resize, same as before.
 *
 * Each panel is driven externally (setFamily / setMember / setAnimTime) rather than
 * subscribing to the store itself. A pane header click collapses that pane to a slim vertical
 * restore bar, giving the other two more room; nothing is persisted, all panes start open.
 */
export function mountBottomPanel(container: HTMLElement, store: Store): BottomPanel {
  container.innerHTML = '';
  const root = document.createElement('div');
  root.className = 'bottom-panel';
  container.appendChild(root);

  const collapsed = new Set<PaneKey>();
  const bodyEls = {} as Record<PaneKey, HTMLDivElement>;
  const asideEls = {} as Record<PaneKey, HTMLSpanElement>;
  const paneEls = {} as Record<PaneKey, HTMLDivElement>;

  for (const key of PANES) {
    const pane = document.createElement('div');
    pane.className = 'bpane';

    const header = document.createElement('div');
    header.className = 'bpane-header';
    const title = document.createElement('span');
    title.className = 'bpane-title';
    title.textContent = PANE_TITLES[key];
    const aside = document.createElement('span');
    aside.className = 'bpane-aside';
    // The metrics pane hosts an interactive <select> in its aside slot — stop its clicks
    // (opening the dropdown, picking an option) from bubbling up and collapsing the pane.
    aside.addEventListener('click', (ev) => ev.stopPropagation());
    header.append(title, aside);
    header.title = 'Click to collapse / restore';
    header.addEventListener('click', () => toggleCollapse(key));

    const body = document.createElement('div');
    body.className = 'bpane-body';

    pane.append(header, body);
    root.appendChild(pane);
    paneEls[key] = pane;
    bodyEls[key] = body;
    asideEls[key] = aside;
  }

  const metrics = mountStabilityPlot(bodyEls.metrics, store, asideEls.metrics);
  const moon = mountMoonTrackPanel(bodyEls.moon);
  const earth = mountSkyViewPanel(bodyEls.earth);

  function applyLayout(): void {
    root.style.gridTemplateColumns = PANES
      .map((key) => (collapsed.has(key) ? COLLAPSED_WIDTH : `${PANE_FR[key]}fr`))
      .join(' ');
    for (const key of PANES) {
      const isCollapsed = collapsed.has(key);
      paneEls[key].classList.toggle('collapsed', isCollapsed);
      bodyEls[key].style.display = isCollapsed ? 'none' : 'block';
    }
  }

  function toggleCollapse(key: PaneKey): void {
    if (collapsed.has(key)) {
      collapsed.delete(key);
      applyLayout();
      // Coming back from `display:none`: re-measure and redraw now that the body has real size.
      if (key === 'metrics') metrics.refresh();
      else if (key === 'moon') moon.refresh();
      else earth.refresh();
    } else {
      collapsed.add(key);
      applyLayout();
    }
  }
  applyLayout();

  return {
    setFamily(family, terms) {
      metrics.setFamily(family, terms);
    },
    setMember(traj, member, resonanceN) {
      moon.setMember(traj, member.period_s, resonanceN);
      earth.setMember(traj, member.period_s, resonanceN);
      asideEls.earth.textContent = traj ? `outage ${(occultedFraction(traj) * 100).toFixed(1)}%` : '';
    },
    setAnimTime(t) {
      moon.setAnimTime(t);
      earth.setAnimTime(t);
    },
    refresh() {
      metrics.refresh();
      moon.refresh();
      earth.refresh();
    },
  };
}
