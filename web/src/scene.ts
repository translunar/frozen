import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';

/** Scene units are megametres: keeps three.js far from float32 precision cliffs. */
export const KM_TO_SCENE = 1 / 1000;
export const MOON_RADIUS_KM = 1737.4;
/** Spin applied to the textured sphere so longitude 0 (sub-Earth point) faces −x. */
export const MOON_TEXTURE_LON_OFFSET = Math.PI;

export type PresetName = 'south-pole' | 'earth-line';

export interface CameraPose {
  position: [number, number, number];
  up: [number, number, number];
}

export function presetCamera(name: PresetName, dist: number): CameraPose {
  if (name === 'south-pole') {
    // Straight up the −z axis at the south pole. camera.up = +z would be degenerate here,
    // so +x (anti-Earth) becomes screen-up and Earth points down the screen.
    return { position: [0, 0, -dist], up: [1, 0, 0] };
  }
  // Earth-Moon line across the screen, orbit normal up, slightly above the xy plane.
  return { position: [0, -dist, 0.25 * dist], up: [0, 0, 1] };
}

export function scenePositions(xyzKm: Float32Array): Float32Array {
  const out = new Float32Array(xyzKm.length);
  for (let i = 0; i < xyzKm.length; i++) out[i] = xyzKm[i] * KM_TO_SCENE;
  return out;
}

export interface Stage {
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  renderer: THREE.WebGLRenderer;
  controls: OrbitControls;
  setFamilyStack(loops: Float32Array[]): void;
  setSelected(traj: Float32Array | null): void;
  setGhost(traj: Float32Array | null): void;
  setGraticule(visible: boolean): void;
  setFrameRadiusKm(km: number): void;
  applyPreset(name: PresetName): void;
  render(): void;
  resize(): void;
}

const STACK_COLOR = 0x63b8ff;
const SELECTED_COLOR = 0xffc24a;
const GHOST_COLOR = 0x777777;

function makeLoop(xyzKm: Float32Array, material: THREE.LineBasicMaterial): THREE.LineLoop {
  const geo = new THREE.BufferGeometry();
  geo.setAttribute('position', new THREE.BufferAttribute(scenePositions(xyzKm), 3));
  return new THREE.LineLoop(geo, material);
}

function clearGroup(group: THREE.Group): void {
  for (const child of [...group.children]) {
    group.remove(child);
    (child as THREE.LineLoop).geometry.dispose();
  }
}

function makeGraticule(radiusKm: number): THREE.LineSegments {
  const r = radiusKm * 1.003 * KM_TO_SCENE;
  const seg = 72;
  const pts: number[] = [];
  const push = (lat: number, lon: number) => {
    pts.push(r * Math.cos(lat) * Math.cos(lon), r * Math.cos(lat) * Math.sin(lon), r * Math.sin(lat));
  };
  for (let latDeg = -60; latDeg <= 60; latDeg += 30) {
    const lat = (latDeg * Math.PI) / 180;
    for (let k = 0; k < seg; k++) {
      push(lat, (2 * Math.PI * k) / seg);
      push(lat, (2 * Math.PI * (k + 1)) / seg);
    }
  }
  for (let lonDeg = 0; lonDeg < 360; lonDeg += 30) {
    const lon = (lonDeg * Math.PI) / 180;
    for (let k = 0; k < seg; k++) {
      push(-Math.PI / 2 + (Math.PI * k) / seg, lon);
      push(-Math.PI / 2 + (Math.PI * (k + 1)) / seg, lon);
    }
  }
  const geo = new THREE.BufferGeometry();
  geo.setAttribute('position', new THREE.Float32BufferAttribute(pts, 3));
  return new THREE.LineSegments(
    geo,
    new THREE.LineBasicMaterial({ color: 0x55dd99, transparent: true, opacity: 0.22 }),
  );
}

export function createStage(container: HTMLElement): Stage {
  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0x05070d);

  const camera = new THREE.PerspectiveCamera(45, 1, 0.02, 5000);
  camera.up.set(0, 0, 1);

  const renderer = new THREE.WebGLRenderer({ antialias: true });
  renderer.setPixelRatio(window.devicePixelRatio);
  container.appendChild(renderer.domElement);

  const controls = new OrbitControls(camera, renderer.domElement);
  controls.enableDamping = true;
  controls.dampingFactor = 0.08;

  scene.add(new THREE.AmbientLight(0xffffff, 0.4));
  const key = new THREE.DirectionalLight(0xffffff, 2.0);
  key.position.set(-6, 3, 2); // lit from the Earth side so the near face reads
  scene.add(key);

  // --- Moon -------------------------------------------------------------
  const moonMat = new THREE.MeshStandardMaterial({ color: 0x9a9a9a, roughness: 1, metalness: 0 });
  const moon = new THREE.Mesh(
    new THREE.SphereGeometry(MOON_RADIUS_KM * KM_TO_SCENE, 96, 64),
    moonMat,
  );
  moon.rotation.x = Math.PI / 2; // three's poles (±y) onto our ±z
  const moonPivot = new THREE.Group();
  moonPivot.rotation.z = MOON_TEXTURE_LON_OFFSET;
  moonPivot.add(moon);
  scene.add(moonPivot);

  new THREE.TextureLoader().load(
    'moon.jpg',
    (tex) => {
      tex.colorSpace = THREE.SRGBColorSpace;
      moonMat.map = tex;
      moonMat.color.set(0xffffff);
      moonMat.needsUpdate = true;
    },
    undefined,
    () => {
      console.warn('scene: moon.jpg failed to load — falling back to flat gray');
    },
  );

  const graticule = makeGraticule(MOON_RADIUS_KM);
  scene.add(graticule);

  // --- South pole marker ------------------------------------------------
  const poleR = MOON_RADIUS_KM * KM_TO_SCENE;
  const poleDot = new THREE.Mesh(
    new THREE.SphereGeometry(0.07, 16, 12),
    new THREE.MeshBasicMaterial({ color: 0xff5566 }),
  );
  poleDot.position.set(0, 0, -poleR);
  scene.add(poleDot);
  const poleGeo = new THREE.BufferGeometry().setFromPoints([
    new THREE.Vector3(0, 0, -poleR),
    new THREE.Vector3(0, 0, -poleR * 1.7),
  ]);
  scene.add(new THREE.Line(poleGeo, new THREE.LineBasicMaterial({ color: 0xff5566 })));

  // --- Earth direction (−x) --------------------------------------------
  scene.add(new THREE.ArrowHelper(
    new THREE.Vector3(-1, 0, 0), new THREE.Vector3(0, 0, 0), 6 * poleR, 0x5aa9ff, 0.7, 0.35,
  ));
  const emGeo = new THREE.BufferGeometry().setFromPoints([
    new THREE.Vector3(0, 0, 0),
    new THREE.Vector3(-80, 0, 0),
  ]);
  scene.add(new THREE.Line(
    emGeo, new THREE.LineBasicMaterial({ color: 0x2a4a7a, transparent: true, opacity: 0.5 }),
  ));

  // --- Orbit layers -----------------------------------------------------
  const stackMat = new THREE.LineBasicMaterial({ color: STACK_COLOR, transparent: true, opacity: 0.15 });
  const selectedMat = new THREE.LineBasicMaterial({ color: SELECTED_COLOR });
  const ghostMat = new THREE.LineBasicMaterial({ color: GHOST_COLOR, transparent: true, opacity: 0.85 });
  const stack = new THREE.Group();
  const selected = new THREE.Group();
  const ghost = new THREE.Group();
  scene.add(stack, selected, ghost);

  let frameDist = 30;
  let preset: PresetName = 'south-pole';

  const stage: Stage = {
    scene,
    camera,
    renderer,
    controls,
    setFamilyStack(loops) {
      clearGroup(stack);
      for (const l of loops) stack.add(makeLoop(l, stackMat));
    },
    setSelected(traj) {
      clearGroup(selected);
      if (traj) selected.add(makeLoop(traj, selectedMat));
    },
    setGhost(traj) {
      clearGroup(ghost);
      if (traj) ghost.add(makeLoop(traj, ghostMat));
    },
    setGraticule(visible) {
      graticule.visible = visible;
    },
    setFrameRadiusKm(km) {
      frameDist = Math.max(4, km * KM_TO_SCENE * 2.6);
    },
    applyPreset(name) {
      preset = name;
      const pose = presetCamera(name, frameDist);
      camera.up.set(pose.up[0], pose.up[1], pose.up[2]);
      camera.position.set(pose.position[0], pose.position[1], pose.position[2]);
      controls.target.set(0, 0, 0);
      controls.update();
    },
    render() {
      controls.update();
      renderer.render(scene, camera);
    },
    resize() {
      const w = container.clientWidth || 1;
      const h = container.clientHeight || 1;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
    },
  };

  stage.resize();
  stage.applyPreset(preset);
  return stage;
}
