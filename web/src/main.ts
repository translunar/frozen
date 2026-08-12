import './style.css';
import { familyPreview, loadCatalog, memberTrajectory } from './data';
import { createStage } from './scene';

const CATALOG_BASE = 'catalog';

async function boot(): Promise<void> {
  const stage = createStage(document.getElementById('stage') as HTMLElement);
  window.addEventListener('resize', () => stage.resize());

  const catalog = await loadCatalog(CATALOG_BASE);
  const combo = catalog.combos[0];
  const family = combo.families[0];
  const member = family.members[Math.floor(family.members.length / 2)];

  stage.setFamilyStack(await familyPreview(CATALOG_BASE, family));
  stage.setSelected(await memberTrajectory(CATALOG_BASE, member));
  stage.setGraticule(true);
  stage.setFrameRadiusKm(member.r_apo_km);
  stage.applyPreset('south-pole');

  const tick = (): void => {
    stage.render();
    requestAnimationFrame(tick);
  };
  tick();
}

boot().catch((err: unknown) => {
  document.body.insertAdjacentHTML(
    'afterbegin',
    `<pre style="color:#ff6b6b;padding:12px">${String(err)}</pre>`,
  );
});
