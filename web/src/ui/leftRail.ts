import { mountAnimControls } from '../anim';
import type { AnimControls } from '../anim';
import { formatReferenceOffset, referencesWithin } from '../references';
import { MOON_RADIUS_KM } from '../scene';
import type { PresetName } from '../scene';
import {
  comboById, displayOrder, familyByN, formatRevs, nearestMemberIndexByRank, nearestRational,
  resonanceBadge, sidRevsPerClosure, synodicRevs,
} from '../state';
import type { Store } from '../state';
import { familyClosures } from '../types';
import type { Catalog, Combo, Family, Member, Terms } from '../types';

export const TERM_LABELS: Array<[keyof Terms, string]> = [
  ['j2', 'J₂ — oblateness'],
  ['c22', 'C₂₂ — equatorial ellipticity'],
  ['j3', 'J₃ — pear shape'],
  ['earth', 'Earth third body'],
];

export function formatReadout(member: Member, family: Family): Array<{ label: string; value: string }> {
  const e = member.elements;
  const closures = familyClosures(family);
  const revs = closures > 1
    ? `${family.resonance_n} over ${closures} closures`
    : `${family.resonance_n}`;
  const synRevs = synodicRevs(family.resonance_n, member.period_s);
  return [
    { label: 'a', value: `${e.a_km.toFixed(0)} km` },
    { label: 'e', value: e.e.toFixed(4) },
    { label: 'i (EM plane)', value: `${e.i_deg.toFixed(2)}°` },
    { label: 'ω', value: `${e.omega_deg.toFixed(2)}°` },
    { label: 'Ω', value: `${e.raan_deg.toFixed(2)}°` },
    { label: 'period', value: `${(member.period_s / 86_400).toFixed(3)} d` },
    { label: 'revs', value: revs },
    { label: 'syn revs', value: synRevs.toFixed(1) },
    { label: 'peri alt', value: `${(member.r_peri_km - MOON_RADIUS_KM).toFixed(0)} km` },
    { label: 'apo alt', value: `${(member.r_apo_km - MOON_RADIUS_KM).toFixed(0)} km` },
    { label: 'ν₁', value: member.nu1.toFixed(3) },
    { label: 'ν₂', value: member.nu2.toFixed(3) },
    { label: 'residual', value: member.residual.toExponential(1) },
  ];
}

export function memberEndpointLabel(member: Member): string {
  const hp = (member.r_peri_km - MOON_RADIUS_KM).toFixed(0);
  const ha = (member.r_apo_km - MOON_RADIUS_KM).toFixed(0);
  return `hp ${hp} · ha ${ha} km · e ${member.elements.e.toFixed(3)}`;
}

/** Hours per revolution: the closure period spans `resonance_n` revs. */
export function revHoursPerOrbit(family: Family): number {
  const periodS = family.members[0]?.period_s ?? 0;
  return Math.round(periodS / 3_600 / family.resonance_n);
}

/** Periapsis-altitude span across a family's members, rounded to whole km. */
export function familyHpRangeKm(family: Family): [number, number] {
  const alts = family.members.map((m) => m.r_peri_km - MOON_RADIUS_KM);
  return [Math.round(Math.min(...alts)), Math.round(Math.max(...alts))];
}

/**
 * Family button headline. k=1 is unchanged (`N = 25 · ~26 h/rev`). k>1 (a rational M:k
 * resonance) leads with the per-closure rev count instead of the raw M, with the M:k pair
 * alongside it for precision: `N = 74.5 (149:2) · ~8.8 h/rev`.
 */
export function familyMainLabel(family: Family): string {
  const closures = familyClosures(family);
  if (closures <= 1) {
    return `N = ${family.resonance_n} · ~${revHoursPerOrbit(family)} h/rev`;
  }
  const periodS = family.members[0]?.period_s ?? 0;
  const hrPerRev = family.resonance_n > 0 ? periodS / 3_600 / family.resonance_n : 0;
  const sid = sidRevsPerClosure(family.resonance_n, closures);
  return `N = ${formatRevs(sid)} (${family.resonance_n}:${closures}) · ~${hrPerRev.toFixed(1)} h/rev`;
}

const MAX_SYN_DEN = 4;
const SIDEREAL_TITLE = 'sidereal closure = ground-track repeat period';
const SYNODIC_TITLE = 'synodic month = Sun-Earth-Moon alignment period';

export interface DualClockLines {
  line1: string;
  line1Title: string;
  line2: string;
  line2Title: string;
}

/**
 * The two-line, plain-language dual-clock readout shown under every family button (k=1 and
 * k>1 alike). Line 1 is the ground-track repeat, always concrete (a family always closes).
 * Line 2 is the sun-geometry repeat, which may or may not exist within a low-denominator
 * rational — `resonanceBadge`'s gate decides whether the nearest p:q fit is a real repeat or
 * just where nearestRational happened to land. Both lines carry a `title` tooltip with the
 * full precision (see formatReadout's `revs`/`syn revs` rows for the same numbers elsewhere)
 * plus a one-phrase definition of the clock in question, for anyone who hovers wondering what
 * "sidereal closure" or "synodic month" means here.
 */
export function familyDualClockLines(family: Family): DualClockLines {
  const closures = familyClosures(family);
  const periodS = family.members[0]?.period_s ?? 0;
  const sid = sidRevsPerClosure(family.resonance_n, closures);
  const syn = synodicRevs(family.resonance_n, periodS);
  const days = periodS / 86_400;

  const line1 = `track repeats: ${family.resonance_n} orbits ≈ ${days.toFixed(1)} d`;
  const line1Title = `${formatRevs(sid)} rev / sidereal closure · ${SIDEREAL_TITLE}`;

  const { p, q, err } = nearestRational(syn, MAX_SYN_DEN);
  const residualDeg = err * q * 360;
  const gatePasses = resonanceBadge(syn, MAX_SYN_DEN) !== '';
  const monthWord = q === 1 ? 'synodic month' : 'synodic months';
  const line2 = gatePasses
    ? `sun geometry repeats: ~${p} orbits ≈ ${q} ${monthWord} (${Math.round(residualDeg)}° drift)`
    : `sun geometry: no repeat within ${MAX_SYN_DEN} months`;
  const line2Title = `${syn.toFixed(2)} rev / synodic month · ${p}:${q} · residual `
    + `${residualDeg.toFixed(1)}° · ${SYNODIC_TITLE}`;

  return { line1, line1Title, line2, line2Title };
}

const REFERENCE_TAG_CAP = 3;

/**
 * Tiny muted tag for a family whose mid-member semi-major axis lands within one or more
 * agency reference orbits' bands (default tolerance), each annotated with its signed percent
 * offset so two families near the same reference (one a touch under, one a touch over) don't
 * render identical tags: `≈ ESA LCNS NAV (−1.9%) band`, or when two bands crowd the same
 * neighborhood, `≈ Stanford LNCSS (−0.7%), ESA LCNS COM (+1.7%) band` (closest first, capped
 * at 3, singular "band" regardless of match count). '' when nothing is that close.
 */
export function familyReferenceTag(family: Family): string {
  const mid = family.members[Math.floor(family.members.length / 2)];
  if (!mid) return '';
  const matches = referencesWithin(mid.elements.a_km).slice(0, REFERENCE_TAG_CAP);
  if (matches.length === 0) return '';
  const parts = matches.map((m) => `${m.reference.name} (${formatReferenceOffset(m.offsetPct)})`);
  return `≈ ${parts.join(', ')} band`;
}

export interface LeftRailHooks {
  /** Per-term: would flipping this term land on a combo that exists in the catalog? */
  availability(): Record<keyof Terms, boolean>;
  onToggle(term: keyof Terms): void;
  onPinGhost(): void;
  onClearGhost(): void;
  onPreset(name: PresetName): void;
  onGraticule(visible: boolean): void;
}

export interface LeftRail {
  anim: AnimControls;
  setNotice(text: string): void;
  refresh(): void;
}

const TEMPLATE = `
  <section class="card">
    <h2>Force model</h2>
    <div id="toggle-board"></div>
    <p id="combo-name" class="muted"></p>
    <p id="notice" class="notice" hidden></p>
  </section>
  <section class="card">
    <h2>Families</h2>
    <div id="family-list"></div>
  </section>
  <section class="card">
    <h2>Member</h2>
    <input id="member-slider" type="range" min="0" max="0" step="1" />
    <div class="endpoints"><span id="ep-lo"></span><span id="ep-hi"></span></div>
  </section>
  <section class="card">
    <h2>Animation</h2>
    <div id="anim-slot"></div>
    <div class="btn-row">
      <button id="pin-ghost" type="button">Pin ghost</button>
      <button id="clear-ghost" type="button">Clear ghost</button>
    </div>
  </section>
  <section class="card">
    <h2>View</h2>
    <div class="btn-row">
      <button id="preset-pole" type="button">South-pole view</button>
      <button id="preset-earth" type="button">Earth-line view</button>
    </div>
    <label class="toggle-row"><input id="graticule-box" type="checkbox" checked /> lat/lon graticule</label>
  </section>
  <section class="card">
    <h2>Selected member</h2>
    <dl id="readout"></dl>
  </section>`;

export function mountLeftRail(
  container: HTMLElement,
  store: Store,
  catalog: Catalog,
  hooks: LeftRailHooks,
): LeftRail {
  container.innerHTML = TEMPLATE;
  const pick = <T extends Element>(sel: string): T => container.querySelector(sel) as T;

  const anim = mountAnimControls(pick<HTMLElement>('#anim-slot'), store);
  const slider = pick<HTMLInputElement>('#member-slider');
  const notice = pick<HTMLParagraphElement>('#notice');
  const clearGhost = pick<HTMLButtonElement>('#clear-ghost');

  // The slider walks *display rank* (periapsis-altitude-sorted), not the raw storage index —
  // see displayOrder's docs. Its value is a rank; convert back to the true member index the
  // store tracks via the current family's display order.
  slider.addEventListener('input', () => {
    const order = displayOrder(currentFamily());
    const rank = Number(slider.value);
    store.update({ memberIndex: order[rank] ?? 0, animTime: 0 });
  });
  pick<HTMLButtonElement>('#pin-ghost').addEventListener('click', () => hooks.onPinGhost());
  clearGhost.addEventListener('click', () => hooks.onClearGhost());
  pick<HTMLButtonElement>('#preset-pole').addEventListener('click', () => hooks.onPreset('south-pole'));
  pick<HTMLButtonElement>('#preset-earth').addEventListener('click', () => hooks.onPreset('earth-line'));
  const gratBox = pick<HTMLInputElement>('#graticule-box');
  gratBox.addEventListener('change', () => hooks.onGraticule(gratBox.checked));

  // A zero-family combo is a legitimate catalog outcome (e.g. `no-earth`), so
  // `catalog.combos[0]`/`combo.families[0]` cannot be trusted as fallbacks —
  // see main.ts's boot-time guard for the same invariant. `mountLeftRail` is
  // only called after that guard has passed, so a combo with families is
  // guaranteed to exist.
  const safeCombo = catalog.combos.find((c) => c.families.length > 0);
  if (!safeCombo) {
    throw new Error('catalog contains no families in any combo — nothing to display');
  }
  const currentCombo = (): Combo => comboById(catalog, store.get().comboId) ?? safeCombo;
  const currentFamily = (): Family => {
    const combo = currentCombo();
    return familyByN(combo, store.get().familyN) ?? combo.families[0] ?? safeCombo.families[0];
  };

  function renderToggles(): void {
    const combo = currentCombo();
    const avail = hooks.availability();
    const board = pick<HTMLDivElement>('#toggle-board');
    board.innerHTML = '';
    for (const [term, label] of TERM_LABELS) {
      const row = document.createElement('label');
      row.className = 'toggle-row';
      const box = document.createElement('input');
      box.type = 'checkbox';
      box.checked = combo.terms[term];
      box.disabled = !avail[term];
      if (!avail[term]) {
        row.classList.add('disabled');
        row.title = 'not in catalog';
      }
      box.addEventListener('change', () => hooks.onToggle(term));
      row.append(box, document.createTextNode(` ${label}`));
      board.appendChild(row);
    }
    pick<HTMLElement>('#combo-name').textContent = combo.name;
  }

  function renderFamilies(): void {
    const combo = currentCombo();
    const from = currentFamily();
    const list = pick<HTMLDivElement>('#family-list');
    list.innerHTML = '';
    for (const fam of combo.families) {
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = `family-btn${fam.resonance_n === from.resonance_n ? ' active' : ''}`;
      const main = document.createElement('span');
      main.className = 'family-btn-main';
      main.textContent = familyMainLabel(fam);
      const [hpMin, hpMax] = familyHpRangeKm(fam);
      const refTag = familyReferenceTag(fam);
      const sub = document.createElement('span');
      sub.className = 'family-btn-sub';
      sub.textContent = refTag ? `hp ${hpMin}–${hpMax} km · ${refTag}` : `hp ${hpMin}–${hpMax} km`;
      btn.append(main, sub);
      const lines = familyDualClockLines(fam);
      const dual1 = document.createElement('span');
      dual1.className = 'family-btn-dual';
      dual1.textContent = lines.line1;
      dual1.title = lines.line1Title;
      const dual2 = document.createElement('span');
      dual2.className = 'family-btn-dual';
      dual2.textContent = lines.line2;
      dual2.title = lines.line2Title;
      btn.append(dual1, dual2);
      btn.addEventListener('click', () => {
        const idx = nearestMemberIndexByRank(store.get().memberIndex, from, fam);
        store.update({ familyN: fam.resonance_n, memberIndex: idx, animTime: 0 });
      });
      list.appendChild(btn);
    }
  }

  function renderMember(): void {
    const family = currentFamily();
    const idx = Math.min(Math.max(0, store.get().memberIndex), family.members.length - 1);
    const member = family.members[idx];
    const order = displayOrder(family);
    const rank = Math.max(0, order.indexOf(idx));

    slider.max = String(family.members.length - 1);
    slider.value = String(rank);
    pick<HTMLElement>('#ep-lo').textContent = memberEndpointLabel(family.members[order[0]]);
    pick<HTMLElement>('#ep-hi').textContent = memberEndpointLabel(family.members[order[order.length - 1]]);

    const dl = pick<HTMLElement>('#readout');
    dl.innerHTML = '';
    for (const row of formatReadout(member, family)) {
      const dt = document.createElement('dt');
      dt.textContent = row.label;
      const dd = document.createElement('dd');
      dd.textContent = row.value;
      dl.append(dt, dd);
    }
    clearGhost.disabled = store.get().ghost === null;
  }

  function refresh(): void {
    renderToggles();
    renderFamilies();
    renderMember();
  }

  function setNotice(text: string): void {
    notice.textContent = text;
    notice.hidden = text === '';
  }

  // animTime ticks every frame — only redraw on the fields the rail displays.
  store.subscribe((s, p) => {
    if (s.comboId !== p.comboId || s.familyN !== p.familyN
        || s.memberIndex !== p.memberIndex || s.ghost !== p.ghost) {
      refresh();
    }
  });
  refresh();

  return { anim, setNotice, refresh };
}
