import type { Store } from '../state';
import type { Family, Member, Terms } from '../types';
import { mountMoonTrackPanel } from './groundTrack';
import { mountSkyViewPanel } from './skyView';
import { mountStabilityPlot } from './stabilityPlot';

export type TabKey = 'metrics' | 'moon' | 'earth';

const TABS: Array<{ key: TabKey; label: string }> = [
  { key: 'metrics', label: 'Metrics' },
  { key: 'moon', label: 'Moon track' },
  { key: 'earth', label: 'Earth view' },
];

export interface BottomPanel {
  setFamily(family: Family, terms: Terms): void;
  setMember(traj: Float32Array | null, member: Member, resonanceN: number): void;
  setAnimTime(t: number): void;
  refresh(): void;
}

/**
 * Tabbed bottom section: Metrics (the existing family stability strip, unchanged), Moon
 * ground track, and Earth plane-of-sky view. Tab selection is local UI state, not stored in
 * the app Store. Each panel is driven externally (setFamily / setMember / setAnimTime) rather
 * than subscribing to the store itself; switching tabs re-renders the newly-shown pane, since
 * an SVG drawn while its container was `display: none` would have measured a zero-size box.
 */
export function mountBottomPanel(container: HTMLElement, store: Store): BottomPanel {
  container.innerHTML = '';
  const root = document.createElement('div');
  root.className = 'bottom-panel';
  container.appendChild(root);

  const tabBar = document.createElement('div');
  tabBar.className = 'tab-bar';
  root.appendChild(tabBar);

  const paneEls: Record<TabKey, HTMLDivElement> = {
    metrics: document.createElement('div'),
    moon: document.createElement('div'),
    earth: document.createElement('div'),
  };
  const buttons = new Map<TabKey, HTMLButtonElement>();
  for (const t of TABS) {
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'tab-btn';
    btn.textContent = t.label;
    btn.addEventListener('click', () => setActive(t.key));
    tabBar.appendChild(btn);
    buttons.set(t.key, btn);

    paneEls[t.key].className = 'tab-pane';
    root.appendChild(paneEls[t.key]);
  }

  const metrics = mountStabilityPlot(paneEls.metrics, store);
  const moon = mountMoonTrackPanel(paneEls.moon);
  const earth = mountSkyViewPanel(paneEls.earth);

  let active: TabKey = 'metrics';

  function setActive(key: TabKey): void {
    active = key;
    for (const t of TABS) {
      paneEls[t.key].style.display = t.key === key ? 'block' : 'none';
      buttons.get(t.key)?.classList.toggle('active', t.key === key);
    }
    if (key === 'metrics') metrics.refresh();
    else if (key === 'moon') moon.refresh();
    else earth.refresh();
  }
  setActive(active);

  return {
    setFamily(family, terms) {
      metrics.setFamily(family, terms);
    },
    setMember(traj, member, resonanceN) {
      moon.setMember(traj, member.period_s, resonanceN);
      earth.setMember(traj, member.period_s, resonanceN);
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
