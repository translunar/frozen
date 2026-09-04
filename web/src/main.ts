import './style.css';
import { advanceTime, createLoop, createSatellite, SPEED_DEFAULT } from './anim';
import { familyPreview, joinUrl, loadCatalog, memberTrajectory, prefetchNeighbors } from './data';
import { createStage } from './scene';
import {
  comboById, createStore, familyByN, findCombo, flipTerm, hydrateUIState,
  nearestMemberIndexByRank, nearestResonance, termAvailability,
} from './state';
import type { PersistedUIState } from './state';
import { mountBottomPanel } from './ui/bottomTabs';
import { mountLeftRail } from './ui/leftRail';
import { mountOrbitOverlay } from './ui/overlay';
import { DEFAULT_METRIC, DEFAULT_X_AXIS } from './ui/stabilityPlot';
import { familyClosures } from './types';
import type { Combo, Family, Terms } from './types';

// import.meta.env.BASE_URL carries the deploy-time subpath (e.g. '/tools/frozen/'), so
// catalog fetches resolve correctly regardless of the page URL's trailing slash.
const CATALOG_BASE = joinUrl(import.meta.env.BASE_URL, 'catalog');
// A user's combo/family/member/metric/x-axis choice survives a page reload or an HMR remount
// during a live demo — sessionStorage rather than localStorage, since it's per-tab context,
// not a durable preference. Both reads and writes are wrapped defensively: some browsers throw
// on sessionStorage access in private/locked-down modes, and that must never block boot.
const SESSION_KEY = 'elfo-ui-state';

function readSession(): string | null {
  try {
    return sessionStorage.getItem(SESSION_KEY);
  } catch {
    return null;
  }
}

function writeSession(state: PersistedUIState): void {
  try {
    sessionStorage.setItem(SESSION_KEY, JSON.stringify(state));
  } catch {
    // Storage unavailable or over quota — persistence is a nicety, not a requirement.
  }
}

async function boot(): Promise<void> {
  const catalog = await loadCatalog(CATALOG_BASE);
  // A zero-family combo is a legitimate catalog outcome (e.g. `no-earth`), and
  // nothing guarantees combos[0] is the populated one — catalog.toml ordering
  // must not decide whether boot crashes. Pick the first combo that actually
  // has a family; if none does, fail through the existing error-banner path
  // with a readable message instead of a TypeError on `.families[0]`.
  const combo0 = catalog.combos.find((c) => c.families.length > 0);
  if (!combo0) {
    throw new Error('catalog contains no families in any combo — nothing to display');
  }
  const family0 = combo0.families[0];

  const defaults: PersistedUIState = {
    comboId: combo0.id,
    familyN: family0.resonance_n,
    memberIndex: Math.floor(family0.members.length / 2),
    metric: DEFAULT_METRIC,
    xAxis: DEFAULT_X_AXIS,
  };
  // Hydration is best-effort: a stale/foreign comboId or familyN just falls through
  // currentCombo()/currentFamily()'s existing `?? combo0`/`?? family0` fallbacks below, same
  // as any other invalid store value would.
  const hydrated = hydrateUIState(readSession(), defaults);

  const store = createStore({
    comboId: hydrated.comboId,
    familyN: hydrated.familyN,
    memberIndex: hydrated.memberIndex,
    animTime: 0,
    playing: true,
    speed: SPEED_DEFAULT,
    ghost: null,
    metric: hydrated.metric,
    xAxis: hydrated.xAxis,
  });

  const stageEl = document.getElementById('stage') as HTMLElement;
  const stage = createStage(stageEl);
  const satellite = createSatellite();
  stage.scene.add(satellite.group);
  const bottom = mountBottomPanel(document.getElementById('plot') as HTMLElement, store);
  const overlay = mountOrbitOverlay(stageEl);

  const currentCombo = (): Combo => comboById(catalog, store.get().comboId) ?? combo0;
  const currentFamily = (): Family => {
    const combo = currentCombo();
    // `combo0`/`family0` are known non-empty (checked above); fall back to
    // them rather than blindly indexing `combo.families[0]`, which would be
    // undefined for a combo the store should never point at but a future
    // caller might (see the boot-time bug this guards against).
    return familyByN(combo, store.get().familyN) ?? combo.families[0] ?? family0;
  };

  const rail = mountLeftRail(document.getElementById('rail') as HTMLElement, store, catalog, {
    availability: () => termAvailability(catalog, currentCombo().terms),
    onToggle: (term) => toggleTerm(term),
    onPinGhost: () => {
      const s = store.get();
      store.update({ ghost: { comboId: s.comboId, familyN: s.familyN, memberIndex: s.memberIndex } });
    },
    onClearGhost: () => store.update({ ghost: null }),
    onPreset: (name) => stage.applyPreset(name),
    onGraticule: (visible) => stage.setGraticule(visible),
  });

  /** Toggle one force term: swap combos, carry the family and member across. */
  function toggleTerm(term: keyof Terms): void {
    const from = currentCombo();
    const target = findCombo(catalog, flipTerm(from.terms, term));
    if (!target) {
      rail.refresh(); // defensive: the checkbox should have been disabled
      return;
    }
    const fromFamily = currentFamily();
    const wantN = fromFamily.resonance_n;

    let toFamily = familyByN(target, wantN);
    if (toFamily) {
      rail.setNotice('');
    } else {
      const n = nearestResonance(target, wantN);
      if (n === null) {
        rail.setNotice(`${target.name} has no frozen families in the catalog`);
        rail.refresh();
        return;
      }
      toFamily = familyByN(target, n) as Family;
      rail.setNotice(`No frozen N=${wantN} family in ${target.name} — showing N=${n}`);
    }

    const memberIndex = nearestMemberIndexByRank(store.get().memberIndex, fromFamily, toFamily);
    store.update({
      comboId: target.id,
      familyN: toFamily.resonance_n,
      memberIndex,
      animTime: 0,
    });
  }

  let generation = 0;
  async function refreshAll(): Promise<void> {
    const gen = ++generation;
    const family = currentFamily();
    const idx = Math.min(Math.max(0, store.get().memberIndex), family.members.length - 1);
    const member = family.members[idx];

    bottom.setFamily(family, currentCombo().terms);
    stage.setFrameRadiusKm(member.r_apo_km);
    const [loops, traj] = await Promise.all([
      familyPreview(CATALOG_BASE, family),
      memberTrajectory(CATALOG_BASE, member),
    ]);
    if (gen !== generation) return; // a newer selection won the race
    stage.setFamilyStack(loops);
    stage.setSelected(traj);
    satellite.setMember(traj, member.period_s, family.resonance_n);
    bottom.setMember(traj, member, family.resonance_n);
    overlay.setSelected(member, family.resonance_n, familyClosures(family));
    prefetchNeighbors(CATALOG_BASE, family, idx);
  }

  let ghostKey = '';
  async function refreshGhost(): Promise<void> {
    const g = store.get().ghost;
    const key = g ? `${g.comboId}/${g.familyN}/${g.memberIndex}` : '';
    if (key === ghostKey) return;
    ghostKey = key;
    if (!g) {
      stage.setGhost(null);
      overlay.setGhost(null, null);
      return;
    }
    const combo = comboById(catalog, g.comboId);
    const family = combo ? familyByN(combo, g.familyN) : undefined;
    const member = family?.members[g.memberIndex];
    stage.setGhost(member ? await memberTrajectory(CATALOG_BASE, member) : null);
    overlay.setGhost(member ?? null, family ? family.resonance_n : null, family ? familyClosures(family) : 1);
  }

  store.subscribe((s, p) => {
    if (s.comboId !== p.comboId || s.familyN !== p.familyN || s.memberIndex !== p.memberIndex) {
      void refreshAll();
    }
    if (s.ghost !== p.ghost) void refreshGhost();
    // Metric/x-axis changes deliberately are NOT in the refreshAll() condition above — they're
    // a redraw-only concern for the metric strip (see stabilityPlot.ts's own subscription),
    // never a trajectory refetch. Persist whatever the write-worthy fields currently are.
    if (
      s.comboId !== p.comboId || s.familyN !== p.familyN || s.memberIndex !== p.memberIndex
      || s.metric !== p.metric || s.xAxis !== p.xAxis
    ) {
      writeSession({
        comboId: s.comboId, familyN: s.familyN, memberIndex: s.memberIndex,
        metric: s.metric, xAxis: s.xAxis,
      });
    }
  });

  await refreshAll();
  stage.applyPreset('south-pole');
  window.addEventListener('resize', () => stage.resize());

  createLoop((dtWall) => {
    const s = store.get();
    const family = currentFamily();
    const member = family.members[Math.min(Math.max(0, s.memberIndex), family.members.length - 1)];
    if (s.playing) {
      store.update({ animTime: advanceTime(s.animTime, dtWall, s.speed, member.period_s) });
    }
    const info = satellite.update(store.get().animTime);
    if (info) rail.anim.setReadout(info.days, info.revs, family.resonance_n);
    bottom.setAnimTime(store.get().animTime);
    stage.render();
  }).start();
}

boot().catch((err: unknown) => {
  document.body.insertAdjacentHTML(
    'afterbegin',
    `<pre style="color:#ff6b6b;padding:12px">${String(err)}</pre>`,
  );
});
