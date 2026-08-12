import { formatRevs, sidRevsPerClosure, synodicRevs } from '../state';
import { MOON_RADIUS_KM } from '../scene';
import type { Member } from '../types';

// Always-visible orbit readout pinned to the 3D stage — unlike the left-rail readout, this
// stays on screen regardless of which bottom-panel tab is active, so the selected member's
// key numbers are never more than a glance away.

/**
 * One compact line summarizing a member's orbit: `N=<n> · hp <km> · ha <km> · e <e> ·
 * i <deg>° (EM plane) · ω <deg>° · <h> h/rev · <sid> sid / <syn> syn`. hp/ha are altitudes
 * above the Moon's mean radius, rounded and thousands-separated; revolution hours divide the
 * closure period (which spans `resonanceN` revs) down to a single rev. `closures` is k in an
 * M:k rational resonance; when k > 1 the N label reads `N=<n>:<k>` instead of the bare
 * integer. The trailing dual-clock segment mirrors the family rail's dual-clock line (revs
 * per sidereal-ish closure / revs per synodic month) so it's visible without opening the rail.
 */
export function overlayLine(member: Member, resonanceN: number, closures = 1): string {
  const hp = Math.round(member.r_peri_km - MOON_RADIUS_KM).toLocaleString('en-US');
  const ha = Math.round(member.r_apo_km - MOON_RADIUS_KM).toLocaleString('en-US');
  const e = member.elements.e.toFixed(3);
  const i = member.elements.i_deg.toFixed(1);
  const omega = member.elements.omega_deg.toFixed(1);
  const revHours = (member.period_s / 3_600 / resonanceN).toFixed(1);
  const nLabel = closures > 1 ? `${resonanceN}:${closures}` : `${resonanceN}`;
  const sid = formatRevs(sidRevsPerClosure(resonanceN, closures));
  const syn = synodicRevs(resonanceN, member.period_s).toFixed(1);
  return `N=${nLabel} · hp ${hp} km · ha ${ha} km · e ${e} · i ${i}° (EM plane) · ω ${omega}° · `
    + `${revHours} h/rev · ${sid} sid / ${syn} syn`;
}

export interface OrbitOverlay {
  setSelected(member: Member, resonanceN: number, closures?: number): void;
  /** Pass `null` for either argument to hide the ghost line (no ghost pinned, or a broken chain). */
  setGhost(member: Member | null, resonanceN: number | null, closures?: number): void;
}

export function mountOrbitOverlay(container: HTMLElement): OrbitOverlay {
  const card = document.createElement('div');
  card.className = 'orbit-overlay';
  const selectedLine = document.createElement('div');
  const ghostLine = document.createElement('div');
  ghostLine.className = 'orbit-overlay-ghost';
  ghostLine.hidden = true;
  card.append(selectedLine, ghostLine);
  container.appendChild(card);

  return {
    setSelected(member, resonanceN, closures = 1) {
      selectedLine.textContent = overlayLine(member, resonanceN, closures);
    },
    setGhost(member, resonanceN, closures = 1) {
      if (member && resonanceN !== null) {
        ghostLine.textContent = `ghost: ${overlayLine(member, resonanceN, closures)}`;
        ghostLine.hidden = false;
      } else {
        ghostLine.hidden = true;
      }
    },
  };
}
