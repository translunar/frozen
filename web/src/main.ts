import './style.css';
import { advanceTime, createLoop, createSatellite, SPEED_DEFAULT } from './anim';
import { familyPreview, loadCatalog, memberTrajectory, prefetchNeighbors } from './data';
import { createStage } from './scene';
import { comboById, createStore, familyByN } from './state';
import { mountLeftRail } from './ui/leftRail';
import { mountStabilityPlot } from './ui/stabilityPlot';
import type { Family, Terms } from './types';

const CATALOG_BASE = 'catalog';

async function boot(): Promise<void> {
  const catalog = await loadCatalog(CATALOG_BASE);
  const combo0 = catalog.combos[0];
  const family0 = combo0.families[0];
  const store = createStore({
    comboId: combo0.id,
    familyN: family0.resonance_n,
    memberIndex: Math.floor(family0.members.length / 2),
    animTime: 0,
    playing: true,
    speed: SPEED_DEFAULT,
    ghost: null,
  });

  const stage = createStage(document.getElementById('stage') as HTMLElement);
  const satellite = createSatellite();
  stage.scene.add(satellite.group);
  const plot = mountStabilityPlot(document.getElementById('plot') as HTMLElement, store);

  const rail = mountLeftRail(document.getElementById('rail') as HTMLElement, store, catalog, {
    // Task 21 replaces this with termAvailability(catalog, currentCombo().terms).
    availability: (): Record<keyof Terms, boolean> =>
      ({ j2: false, c22: false, j3: false, earth: false }),
    onToggle: () => undefined,
    onPinGhost: () => {
      const s = store.get();
      store.update({ ghost: { comboId: s.comboId, familyN: s.familyN, memberIndex: s.memberIndex } });
    },
    onClearGhost: () => store.update({ ghost: null }),
    onPreset: (name) => stage.applyPreset(name),
    onGraticule: (visible) => stage.setGraticule(visible),
  });

  const currentFamily = (): Family => {
    const combo = comboById(catalog, store.get().comboId) ?? catalog.combos[0];
    return familyByN(combo, store.get().familyN) ?? combo.families[0];
  };

  let generation = 0;
  async function refreshAll(): Promise<void> {
    const gen = ++generation;
    const family = currentFamily();
    const idx = Math.min(Math.max(0, store.get().memberIndex), family.members.length - 1);
    const member = family.members[idx];
    plot.setFamily(family);
    stage.setFrameRadiusKm(member.r_apo_km);
    const [loops, traj] = await Promise.all([
      familyPreview(CATALOG_BASE, family),
      memberTrajectory(CATALOG_BASE, member),
    ]);
    if (gen !== generation) return;
    stage.setFamilyStack(loops);
    stage.setSelected(traj);
    satellite.setMember(traj, member.period_s, family.resonance_n);
    prefetchNeighbors(CATALOG_BASE, family, idx);
  }

  store.subscribe((s, p) => {
    if (s.comboId !== p.comboId || s.familyN !== p.familyN || s.memberIndex !== p.memberIndex) {
      void refreshAll();
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
    stage.render();
  }).start();
}

boot().catch((err: unknown) => {
  document.body.insertAdjacentHTML(
    'afterbegin',
    `<pre style="color:#ff6b6b;padding:12px">${String(err)}</pre>`,
  );
});
