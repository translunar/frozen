import './style.css';
import { advanceTime, createLoop, createSatellite, mountAnimControls, SPEED_DEFAULT } from './anim';
import { familyPreview, loadCatalog, memberTrajectory } from './data';
import { createStage } from './scene';
import { createStore } from './state';

const CATALOG_BASE = 'catalog';

async function boot(): Promise<void> {
  const stage = createStage(document.getElementById('stage') as HTMLElement);
  window.addEventListener('resize', () => stage.resize());

  const catalog = await loadCatalog(CATALOG_BASE);
  const combo = catalog.combos[0];
  const family = combo.families[0];
  const memberIndex = Math.floor(family.members.length / 2);
  const member = family.members[memberIndex];

  const store = createStore({
    comboId: combo.id,
    familyN: family.resonance_n,
    memberIndex,
    animTime: 0,
    playing: true,
    speed: SPEED_DEFAULT,
    ghost: null,
  });

  const rail = document.getElementById('rail') as HTMLElement;
  rail.innerHTML = '<section class="card"><h2>Animation</h2><div id="anim-slot"></div></section>';
  const animControls = mountAnimControls(document.getElementById('anim-slot') as HTMLElement, store);

  const satellite = createSatellite();
  stage.scene.add(satellite.group);

  const traj = await memberTrajectory(CATALOG_BASE, member);
  stage.setFamilyStack(await familyPreview(CATALOG_BASE, family));
  stage.setSelected(traj);
  stage.setGraticule(true);
  stage.setFrameRadiusKm(member.r_apo_km);
  stage.applyPreset('south-pole');
  satellite.setMember(traj, member.period_s, family.resonance_n);

  createLoop((dtWall) => {
    const s = store.get();
    if (s.playing) {
      store.update({ animTime: advanceTime(s.animTime, dtWall, s.speed, member.period_s) });
    }
    const info = satellite.update(store.get().animTime);
    if (info) animControls.setReadout(info.days, info.revs, family.resonance_n);
    stage.render();
  }).start();
}

boot().catch((err: unknown) => {
  document.body.insertAdjacentHTML(
    'afterbegin',
    `<pre style="color:#ff6b6b;padding:12px">${String(err)}</pre>`,
  );
});
