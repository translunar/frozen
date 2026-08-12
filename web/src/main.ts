import './style.css';
import { advanceTime, createLoop, createSatellite, SPEED_DEFAULT } from './anim';
import { familyPreview, loadCatalog, memberTrajectory, prefetchNeighbors } from './data';
import { createStage } from './scene';
import {
  comboById, createStore, familyByN, findCombo, flipTerm,
  nearestMemberIndex, nearestResonance, termAvailability,
} from './state';
import { mountBottomPanel } from './ui/bottomTabs';
import { mountLeftRail } from './ui/leftRail';
import { mountOrbitOverlay } from './ui/overlay';
import type { Combo, Family, Terms } from './types';

const CATALOG_BASE = 'catalog';

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

  const store = createStore({
    comboId: combo0.id,
    familyN: family0.resonance_n,
    memberIndex: Math.floor(family0.members.length / 2),
    animTime: 0,
    playing: true,
    speed: SPEED_DEFAULT,
    ghost: null,
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

    const memberIndex = nearestMemberIndex(
      store.get().memberIndex, fromFamily.members.length, toFamily.members.length,
    );
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

    bottom.setFamily(family);
    stage.setFrameRadiusKm(member.r_apo_km);
    const [loops, traj] = await Promise.all([
      familyPreview(CATALOG_BASE, family),
      memberTrajectory(CATALOG_BASE, member),
    ]);
    if (gen !== generation) return; // a newer selection won the race
    stage.setFamilyStack(loops);
    stage.setSelected(traj);
    satellite.setMember(traj, member.period_s, family.resonance_n);
    bottom.setMember(traj, member);
    overlay.setSelected(member, family.resonance_n);
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
    overlay.setGhost(member ?? null, family ? family.resonance_n : null);
  }

  store.subscribe((s, p) => {
    if (s.comboId !== p.comboId || s.familyN !== p.familyN || s.memberIndex !== p.memberIndex) {
      void refreshAll();
    }
    if (s.ghost !== p.ghost) void refreshGhost();
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
