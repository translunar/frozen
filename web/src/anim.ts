import * as THREE from 'three';
import { KM_TO_SCENE } from './scene';
import { samplePosition } from './state';
import type { Store } from './state';

/** Simulated seconds per wall-clock second. */
export const SPEED_MIN = 60;        // 1 minute per second
export const SPEED_MAX = 864_000;   // 10 days per second
export const SPEED_DEFAULT = 21_600; // 6 hours per second
export const TRAIL_POINTS = 200;
export const TRAIL_SPAN_FRAC = 0.08; // trail covers 8% of the closure period

export function speedFromDial(u: number): number {
  const c = Math.min(1, Math.max(0, u));
  return SPEED_MIN * Math.pow(SPEED_MAX / SPEED_MIN, c);
}

export function dialFromSpeed(v: number): number {
  const c = Math.min(SPEED_MAX, Math.max(SPEED_MIN, v));
  return Math.log(c / SPEED_MIN) / Math.log(SPEED_MAX / SPEED_MIN);
}

export function advanceTime(t: number, dtWallS: number, speed: number, periodS: number): number {
  if (!(periodS > 0)) return 0;
  const next = t + dtWallS * speed;
  return ((next % periodS) + periodS) % periodS;
}

export function trailTimes(
  tNow: number,
  periodS: number,
  count: number = TRAIL_POINTS,
  spanFrac: number = TRAIL_SPAN_FRAC,
): number[] {
  const span = periodS * spanFrac;
  const out: number[] = [];
  for (let k = 0; k < count; k++) out.push(tNow - (span * k) / (count - 1));
  return out;
}

export function readout(tS: number, periodS: number, resonanceN: number): { days: number; revs: number } {
  return {
    days: tS / 86_400,
    revs: periodS > 0 ? (tS / periodS) * resonanceN : 0,
  };
}

export interface Satellite {
  group: THREE.Group;
  setMember(traj: Float32Array | null, periodS: number, resonanceN: number): void;
  update(animTimeS: number): { positionKm: [number, number, number]; days: number; revs: number } | null;
}

export function createSatellite(): Satellite {
  const group = new THREE.Group();

  const marker = new THREE.Mesh(
    new THREE.SphereGeometry(0.09, 16, 12),
    new THREE.MeshBasicMaterial({ color: 0xffffff }),
  );
  group.add(marker);

  const positions = new Float32Array(TRAIL_POINTS * 3);
  const colors = new Float32Array(TRAIL_POINTS * 3);
  const trailGeo = new THREE.BufferGeometry();
  trailGeo.setAttribute('position', new THREE.BufferAttribute(positions, 3));
  trailGeo.setAttribute('color', new THREE.BufferAttribute(colors, 3));
  // Per-vertex alpha is not available on lines; ramp the colour toward the
  // background instead, which reads as a fading trail on the dark stage.
  const trail = new THREE.Line(
    trailGeo,
    new THREE.LineBasicMaterial({ vertexColors: true, transparent: true, opacity: 0.9 }),
  );
  group.add(trail);

  let traj: Float32Array | null = null;
  let periodS = 0;
  let resonanceN = 1;

  return {
    group,
    setMember(t, p, n) {
      traj = t;
      periodS = p;
      resonanceN = n;
      group.visible = t !== null;
    },
    update(animTimeS) {
      if (!traj || periodS <= 0) return null;
      const p = samplePosition(traj, periodS, animTimeS);
      marker.position.set(p[0] * KM_TO_SCENE, p[1] * KM_TO_SCENE, p[2] * KM_TO_SCENE);

      const times = trailTimes(animTimeS, periodS);
      for (let k = 0; k < times.length; k++) {
        const q = samplePosition(traj, periodS, times[k]);
        positions[k * 3] = q[0] * KM_TO_SCENE;
        positions[k * 3 + 1] = q[1] * KM_TO_SCENE;
        positions[k * 3 + 2] = q[2] * KM_TO_SCENE;
        const fade = 1 - k / (times.length - 1);
        colors[k * 3] = fade;
        colors[k * 3 + 1] = fade * 0.85;
        colors[k * 3 + 2] = fade * 0.55;
      }
      trailGeo.attributes.position.needsUpdate = true;
      trailGeo.attributes.color.needsUpdate = true;
      trailGeo.computeBoundingSphere();

      const r = readout(animTimeS, periodS, resonanceN);
      return { positionKm: p, days: r.days, revs: r.revs };
    },
  };
}

export function createLoop(onTick: (dtWallS: number) => void): { start(): void; stop(): void } {
  let raf = 0;
  let last = 0;
  const frame = (now: number): void => {
    const dt = last === 0 ? 0 : Math.min(0.1, (now - last) / 1000);
    last = now;
    onTick(dt);
    raf = requestAnimationFrame(frame);
  };
  return {
    start() {
      if (raf === 0) {
        last = 0;
        raf = requestAnimationFrame(frame);
      }
    },
    stop() {
      if (raf !== 0) {
        cancelAnimationFrame(raf);
        raf = 0;
      }
    },
  };
}

export interface AnimControls {
  setReadout(days: number, revs: number, resonanceN: number): void;
}

function formatSpeed(v: number): string {
  if (v >= 86_400) return `${(v / 86_400).toFixed(1)} d/s`;
  if (v >= 3_600) return `${(v / 3_600).toFixed(1)} h/s`;
  return `${(v / 60).toFixed(0)} min/s`;
}

export function mountAnimControls(container: HTMLElement, store: Store): AnimControls {
  container.innerHTML = `
    <div class="anim-controls">
      <button id="play-btn" type="button">Play</button>
      <label class="dial">
        <span>Speed</span>
        <input id="speed-dial" type="range" min="0" max="1" step="0.001" />
        <span id="speed-label" class="muted"></span>
      </label>
      <div id="anim-readout" class="readout-line">t + 0.00 d · rev 0.0</div>
    </div>`;

  const play = container.querySelector('#play-btn') as HTMLButtonElement;
  const dial = container.querySelector('#speed-dial') as HTMLInputElement;
  const speedLabel = container.querySelector('#speed-label') as HTMLSpanElement;
  const out = container.querySelector('#anim-readout') as HTMLDivElement;

  const sync = (): void => {
    const s = store.get();
    play.textContent = s.playing ? 'Pause' : 'Play';
    dial.value = String(dialFromSpeed(s.speed));
    speedLabel.textContent = formatSpeed(s.speed);
  };

  play.addEventListener('click', () => store.update({ playing: !store.get().playing }));
  dial.addEventListener('input', () => store.update({ speed: speedFromDial(Number(dial.value)) }));
  // animTime changes every frame — only re-sync on the fields these controls show.
  store.subscribe((s, p) => {
    if (s.playing !== p.playing || s.speed !== p.speed) sync();
  });
  sync();

  return {
    setReadout(days, revs, resonanceN) {
      out.textContent = `t + ${days.toFixed(2)} d · rev ${revs.toFixed(1)} / ${resonanceN}`;
    },
  };
}
