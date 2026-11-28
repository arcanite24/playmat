import { useEffect, useRef } from 'react';
import {
  AmbientLight,
  Box3,
  BoxGeometry,
  ConeGeometry,
  CylinderGeometry,
  DirectionalLight,
  Group,
  HemisphereLight,
  type Material,
  Mesh,
  MeshStandardMaterial,
  NearestFilter,
  type Object3D,
  OctahedronGeometry,
  PerspectiveCamera,
  Scene,
  SRGBColorSpace,
  type Texture,
  TorusGeometry,
  Vector3,
  WebGLRenderer,
} from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';

/**
 * A pet drawn with three.js in its own small canvas: it breathes, wags and looks about while idle, and celebrates each
 * time `cheers` goes up (a hop and a spin, the head thrown back, the hat and the pom bouncing, the tail going fast).
 * The parts it moves are found by name (tail, head, ear, hat, pom); a model without one just doesn't move it. With
 * reduced motion it sits still and only gives a small hop. Loads with the first pet at the table.
 */

const CELEBRATE_MS = 1100;
/** idle frames are cheap but needn't be every frame */
const IDLE_FPS = 30;

interface Rig {
  root: Group;
  body: Group;
  head: Object3D | null;
  headRest: { x: number; y: number; z: number };
  tail: Group | null;
  ears: Group[];
  hat: Object3D | null;
  hatRest: { x: number; z: number };
  pom: Object3D | null;
  pomY: number;
  aura: Group | null;
}

const models = new Map<string, Promise<Object3D>>();

/** Builds an articulated low-poly 3D companion rig for earnable Career 3D pets (Phase 6.1). */
function buildProceduralPet(id: string): Object3D {
  const root = new Group();
  if (id === 'clockwork-owl') {
    const bronze = new MeshStandardMaterial({ color: 0xb8864b, metalness: 0.65, roughness: 0.35 });
    const copper = new MeshStandardMaterial({ color: 0x8f4e2b, metalness: 0.6, roughness: 0.4 });
    const dark = new MeshStandardMaterial({ color: 0x2c241d, roughness: 0.7 });
    const eyeGlow = new MeshStandardMaterial({ color: 0xfde047, emissive: 0xf59e0b, emissiveIntensity: 0.75, roughness: 0.2 });

    const torso = new Mesh(new CylinderGeometry(0.32, 0.38, 0.62, 8), bronze);
    torso.position.y = 0.44;
    root.add(torso);

    const gear = new Mesh(new TorusGeometry(0.13, 0.03, 6, 10), copper);
    gear.position.set(0, 0.42, 0.34);
    root.add(gear);

    for (const sx of [-1, 1]) {
      const foot = new Mesh(new BoxGeometry(0.14, 0.08, 0.2), dark);
      foot.position.set(sx * 0.16, 0.06, 0.05);
      root.add(foot);
      const wing = new Mesh(new BoxGeometry(0.12, 0.44, 0.32), copper);
      wing.name = sx < 0 ? 'ear' : 'ear~x';
      wing.position.set(sx * 0.4, 0.46, -0.02);
      wing.rotation.z = sx * 0.15;
      root.add(wing);
    }

    const head = new Group();
    head.name = 'head';
    head.position.set(0, 0.88, 0.02);
    head.add(new Mesh(new CylinderGeometry(0.28, 0.3, 0.34, 8), bronze));
    for (const sx of [-1, 1]) {
      const rim = new Mesh(new TorusGeometry(0.085, 0.02, 6, 10), dark);
      rim.position.set(sx * 0.12, 0.04, 0.26);
      const eye = new Mesh(new CylinderGeometry(0.06, 0.06, 0.04, 8), eyeGlow);
      eye.rotation.x = Math.PI / 2;
      eye.position.set(sx * 0.12, 0.04, 0.26);
      const tuft = new Mesh(new ConeGeometry(0.06, 0.16, 4), copper);
      tuft.position.set(sx * 0.19, 0.22, 0.04);
      tuft.rotation.z = -sx * 0.35;
      head.add(rim, eye, tuft);
    }
    const beak = new Mesh(new ConeGeometry(0.06, 0.16, 4), copper);
    beak.rotation.x = Math.PI / 2;
    beak.position.set(0, -0.02, 0.32);
    head.add(beak);
    root.add(head);

    const tail = new Mesh(new BoxGeometry(0.24, 0.1, 0.24), copper);
    tail.name = 'tail';
    tail.position.set(0, 0.22, -0.38);
    tail.rotation.x = -0.35;
    root.add(tail);
    return root;
  }

  if (id === 'mini-dragon') {
    const crimson = new MeshStandardMaterial({ color: 0xb8342a, roughness: 0.45, metalness: 0.15 });
    const belly = new MeshStandardMaterial({ color: 0xe6b96c, roughness: 0.5 });
    const hornMat = new MeshStandardMaterial({ color: 0xf0d69c, roughness: 0.35, metalness: 0.25 });
    const eyeGlow = new MeshStandardMaterial({ color: 0x34d399, emissive: 0x10b981, emissiveIntensity: 0.8 });

    const body = new Mesh(new BoxGeometry(0.46, 0.42, 0.62), crimson);
    body.position.set(0, 0.38, 0);
    const chest = new Mesh(new BoxGeometry(0.32, 0.34, 0.12), belly);
    chest.position.set(0, 0.36, 0.29);
    root.add(body, chest);

    for (const sx of [-1, 1]) {
      for (const sz of [-0.2, 0.2]) {
        const leg = new Mesh(new BoxGeometry(0.14, 0.22, 0.16), crimson);
        leg.position.set(sx * 0.19, 0.12, sz);
        root.add(leg);
      }
      const wing = new Mesh(new BoxGeometry(0.5, 0.08, 0.34), crimson);
      wing.name = sx < 0 ? 'ear' : 'ear~x';
      wing.position.set(sx * 0.44, 0.56, -0.06);
      wing.rotation.z = sx * 0.45;
      root.add(wing);
    }

    const head = new Group();
    head.name = 'head';
    head.position.set(0, 0.72, 0.22);
    head.add(new Mesh(new BoxGeometry(0.36, 0.32, 0.38), crimson));
    const snout = new Mesh(new BoxGeometry(0.24, 0.18, 0.22), crimson);
    snout.position.set(0, -0.04, 0.26);
    head.add(snout);
    for (const sx of [-1, 1]) {
      const horn = new Mesh(new ConeGeometry(0.055, 0.24, 5), hornMat);
      horn.position.set(sx * 0.12, 0.22, -0.08);
      horn.rotation.x = -0.45;
      horn.rotation.z = -sx * 0.2;
      const eye = new Mesh(new BoxGeometry(0.06, 0.06, 0.04), eyeGlow);
      eye.position.set(sx * 0.11, 0.04, 0.19);
      head.add(horn, eye);
    }
    root.add(head);

    const tail = new Mesh(new ConeGeometry(0.11, 0.56, 5), crimson);
    tail.name = 'tail';
    tail.rotation.x = -Math.PI / 2 - 0.25;
    tail.position.set(0, 0.28, -0.52);
    root.add(tail);
    return root;
  }

  // golem-pup: sturdy stone-and-brass rune puppy with mossy shoulders and glowing amber eyes
  const stone = new MeshStandardMaterial({ color: 0x7a7872, roughness: 0.85, metalness: 0.1 });
  const moss = new MeshStandardMaterial({ color: 0x4d7c47, roughness: 0.9 });
  const brass = new MeshStandardMaterial({ color: 0xd4a44a, metalness: 0.7, roughness: 0.28 });
  const runeGlow = new MeshStandardMaterial({ color: 0xfbbf24, emissive: 0xf59e0b, emissiveIntensity: 0.85 });

  const body = new Mesh(new BoxGeometry(0.5, 0.42, 0.62), stone);
  body.position.set(0, 0.36, 0);
  const mossCap = new Mesh(new BoxGeometry(0.52, 0.1, 0.34), moss);
  mossCap.position.set(0, 0.56, -0.04);
  const collar = new Mesh(new BoxGeometry(0.46, 0.1, 0.1), brass);
  collar.position.set(0, 0.44, 0.3);
  root.add(body, mossCap, collar);

  for (const sx of [-1, 1]) {
    for (const sz of [-0.2, 0.2]) {
      const paw = new Mesh(new BoxGeometry(0.16, 0.22, 0.18), stone);
      paw.position.set(sx * 0.2, 0.11, sz);
      root.add(paw);
    }
  }

  const head = new Group();
  head.name = 'head';
  head.position.set(0, 0.68, 0.24);
  head.add(new Mesh(new BoxGeometry(0.42, 0.36, 0.4), stone));
  const snout = new Mesh(new BoxGeometry(0.24, 0.16, 0.16), stone);
  snout.position.set(0, -0.07, 0.24);
  const nose = new Mesh(new BoxGeometry(0.09, 0.06, 0.06), brass);
  nose.position.set(0, -0.02, 0.33);
  head.add(snout, nose);
  for (const sx of [-1, 1]) {
    const eye = new Mesh(new BoxGeometry(0.07, 0.07, 0.04), runeGlow);
    eye.position.set(sx * 0.11, 0.04, 0.2);
    const ear = new Mesh(new BoxGeometry(0.1, 0.24, 0.16), stone);
    ear.name = sx < 0 ? 'ear' : 'ear~x';
    ear.position.set(sx * 0.25, 0.04, 0);
    ear.rotation.z = sx * 0.22;
    head.add(eye, ear);
  }
  root.add(head);

  const tail = new Mesh(new OctahedronGeometry(0.12), runeGlow);
  tail.name = 'tail';
  tail.position.set(0, 0.44, -0.38);
  root.add(tail);
  return root;
}

function loadModel(url: string): Promise<Object3D> {
  if (url.startsWith('procedural:')) {
    return Promise.resolve(buildProceduralPet(url.slice('procedural:'.length)));
  }
  let model = models.get(url);
  if (!model) {
    model = new GLTFLoader().loadAsync(url).then((gltf) => gltf.scene);
    model.catch(() => models.delete(url));
    models.set(url, model);
  }
  return model.then((scene) => scene.clone(true));
}

/** Wraps a part in a pivot at `at` (in its parent's space), so it turns about that point instead of the origin. */
function pivot(part: Object3D, at: Vector3): Group {
  const parent = part.parent!;
  const group = new Group();
  group.position.copy(at);
  parent.add(group);
  part.position.sub(at);
  group.add(part);
  return group;
}

/** A part's bounds in its parent's space. */
function boundsInParent(part: Object3D): Box3 {
  part.updateWorldMatrix(true, true);
  const box = new Box3().setFromObject(part);
  const inverse = part.parent!.matrixWorld.clone().invert();
  return box.applyMatrix4(inverse);
}

function rig(model: Object3D): Rig {
  const root = new Group();
  const body = new Group();
  root.add(body);
  body.add(model);
  const named = (name: string) => model.getObjectByName(name) ?? null;

  const head = named('head');
  let tail: Group | null = null;
  const tailPart = named('tail');
  if (tailPart) {
    // the tail turns about its root: the end nearest the body's centre
    const box = boundsInParent(tailPart);
    const center = box.getCenter(new Vector3());
    const towardBody = Math.abs(box.max.z) < Math.abs(box.min.z) ? box.max.z : box.min.z;
    tail = pivot(tailPart, new Vector3(center.x, center.y, towardBody));
  }
  const ears: Group[] = [];
  for (const name of ['ear', 'ear~x']) {
    const ear = named(name);
    if (!ear) continue;
    const inner = named(`inner_${name}`);
    const box = boundsInParent(ear);
    const base = new Vector3(box.getCenter(new Vector3()).x, box.min.y, box.getCenter(new Vector3()).z);
    const group = pivot(ear, base);
    if (inner && inner.parent === group.parent) {
      inner.position.sub(base);
      group.add(inner);
    }
    ears.push(group);
  }
  const hat = named('hat');
  const pom = named('pom');
  return {
    root,
    body,
    head,
    headRest: { x: head?.rotation.x ?? 0, y: head?.rotation.y ?? 0, z: head?.rotation.z ?? 0 },
    tail,
    ears,
    hat,
    hatRest: { x: hat?.rotation.x ?? 0, z: hat?.rotation.z ?? 0 },
    pom,
    pomY: pom?.position.y ?? 0,
    aura: null,
  };
}

/** Bounds of an object expressed in its own local coordinate space. */
function localBounds(part: Object3D): Box3 {
  part.updateWorldMatrix(true, true);
  const box = new Box3().setFromObject(part);
  const inverse = part.matrixWorld.clone().invert();
  return box.applyMatrix4(inverse);
}

/**
 * Procedural low-poly accessories unlocked by Pet Bond Tier (Tiers 3-5): crown, shades, wizard-hat, mana-collar,
 * and astral-aura orbiting sparks. Scaled from the rig's measured head/body bounds so every 3D pet wears them cleanly.
 */
function attachStyle(rigged: Rig, loaded: Object3D, style: string | null | undefined, tier: number): void {
  const anchor = rigged.head ?? rigged.body;
  const headBox = localBounds(anchor);
  const headSize = headBox.getSize(new Vector3());
  const headCenter = headBox.getCenter(new Vector3());
  const unit = Math.max(0.15, Math.min(headSize.x, headSize.z));

  if (style === 'crown') {
    const crown = new Group();
    const gold = new MeshStandardMaterial({ color: 0xf4c95d, metalness: 0.65, roughness: 0.28, emissive: 0x452b00, emissiveIntensity: 0.25 });
    const gemMat = new MeshStandardMaterial({ color: 0xd9383a, metalness: 0.2, roughness: 0.2, emissive: 0x550808, emissiveIntensity: 0.35 });
    const band = new Mesh(new CylinderGeometry(unit * 0.34, unit * 0.3, unit * 0.14, 8), gold);
    crown.add(band);
    for (let i = 0; i < 4; i++) {
      const angle = (i / 4) * Math.PI * 2;
      const spike = new Mesh(new ConeGeometry(unit * 0.09, unit * 0.19, 4), gold);
      spike.position.set(Math.cos(angle) * unit * 0.28, unit * 0.14, Math.sin(angle) * unit * 0.28);
      crown.add(spike);
    }
    const gem = new Mesh(new OctahedronGeometry(unit * 0.065), gemMat);
    gem.position.set(0, unit * 0.02, unit * 0.33);
    crown.add(gem);
    crown.position.set(headCenter.x, headBox.max.y + unit * 0.04, headCenter.z);
    crown.rotation.z = -0.1;
    anchor.add(crown);
  } else if (style === 'wizard-hat') {
    const hat = new Group();
    const cloth = new MeshStandardMaterial({ color: 0x3b2c6e, roughness: 0.72, metalness: 0.12 });
    const trim = new MeshStandardMaterial({ color: 0xe6be5c, metalness: 0.6, roughness: 0.3, emissive: 0x3b2600, emissiveIntensity: 0.25 });
    const brim = new Mesh(new CylinderGeometry(unit * 0.54, unit * 0.58, unit * 0.05, 12), cloth);
    hat.add(brim);
    const band = new Mesh(new CylinderGeometry(unit * 0.32, unit * 0.35, unit * 0.08, 12), trim);
    band.position.y = unit * 0.06;
    hat.add(band);
    const cone = new Mesh(new ConeGeometry(unit * 0.33, unit * 0.62, 12), cloth);
    cone.position.y = unit * 0.36;
    cone.rotation.z = 0.12;
    hat.add(cone);
    const star = new Mesh(new OctahedronGeometry(unit * 0.07), trim);
    star.position.set(-unit * 0.06, unit * 0.66, 0);
    hat.add(star);
    hat.position.set(headCenter.x, headBox.max.y + unit * 0.02, headCenter.z);
    anchor.add(hat);
  } else if (style === 'shades') {
    const shades = new Group();
    const frame = new MeshStandardMaterial({ color: 0x16181f, roughness: 0.25, metalness: 0.7 });
    const lensMat = new MeshStandardMaterial({ color: 0x101828, roughness: 0.1, metalness: 0.9, emissive: 0x1d4ed8, emissiveIntensity: 0.25 });
    const left = new Mesh(new BoxGeometry(unit * 0.28, unit * 0.14, unit * 0.05), lensMat);
    left.position.set(-unit * 0.18, 0, 0);
    const right = new Mesh(new BoxGeometry(unit * 0.28, unit * 0.14, unit * 0.05), lensMat);
    right.position.set(unit * 0.18, 0, 0);
    const bridge = new Mesh(new BoxGeometry(unit * 0.12, unit * 0.04, unit * 0.05), frame);
    bridge.position.set(0, unit * 0.03, 0);
    shades.add(left, right, bridge);
    shades.position.set(headCenter.x, headCenter.y + headSize.y * 0.06, headBox.max.z + unit * 0.02);
    anchor.add(shades);
  } else if (style === 'mana-collar') {
    const collar = new Group();
    const band = new Mesh(
      new TorusGeometry(unit * 0.42, unit * 0.055, 8, 20),
      new MeshStandardMaterial({ color: 0xc9a35a, metalness: 0.7, roughness: 0.25 }),
    );
    band.rotation.x = Math.PI / 2;
    const gem = new Mesh(
      new OctahedronGeometry(unit * 0.1),
      new MeshStandardMaterial({ color: 0x38bdf8, emissive: 0x0ea5e9, emissiveIntensity: 0.65, roughness: 0.15 }),
    );
    gem.position.set(0, -unit * 0.04, unit * 0.44);
    collar.add(band, gem);
    collar.position.set(headCenter.x, headBox.min.y + unit * 0.04, headCenter.z);
    anchor.add(collar);
  }

  // Tier 5 (or explicit astral-aura style) spawns orbiting mana star sparks around the pet
  if (style === 'astral-aura' || tier >= 5) {
    const bodyBox = new Box3().setFromObject(loaded);
    const bodyCenter = bodyBox.getCenter(new Vector3());
    const radius = Math.max(bodyBox.getSize(new Vector3()).x, bodyBox.getSize(new Vector3()).z) * 0.68;
    const aura = new Group();
    aura.position.copy(bodyCenter);
    const count = style === 'astral-aura' ? 7 : 4;
    const colors = [0xfde047, 0x67e8f9, 0xf08a4b, 0xc084fc];
    for (let i = 0; i < count; i++) {
      const angle = (i / count) * Math.PI * 2;
      const color = colors[i % colors.length];
      const spark = new Mesh(
        new OctahedronGeometry(radius * (style === 'astral-aura' ? 0.095 : 0.07)),
        new MeshStandardMaterial({ color, emissive: color, emissiveIntensity: 0.85, roughness: 0.15 }),
      );
      spark.position.set(Math.cos(angle) * radius, Math.sin(angle * 2) * radius * 0.22, Math.sin(angle) * radius);
      aura.add(spark);
    }
    rigged.root.add(aura);
    rigged.aura = aura;
  }
}

/** crisp pixel textures, like the art was painted */
function sharpen(model: Object3D): void {
  model.traverse((node) => {
    const mesh = node as Mesh;
    if (!mesh.isMesh) return;
    for (const material of ([] as Material[]).concat(mesh.material)) {
      const map = (material as Material & { map?: Texture | null }).map;
      if (map) {
        map.magFilter = NearestFilter;
        map.colorSpace = SRGBColorSpace;
        map.needsUpdate = true;
      }
    }
  });
}

function reducedMotion(): boolean {
  return typeof window !== 'undefined' && !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
}

const ease = (t: number) => 1 - (1 - t) ** 3;

/** Poses the rig at `now`: idle, with a celebration that started at `cheeredAt` laid over it. */
function pose(rig: Rig, now: number, cheeredAt: number, still: boolean): void {
  const t = now / 1000;
  const since = now - cheeredAt;
  const party = since >= 0 && since < CELEBRATE_MS ? since / CELEBRATE_MS : -1;

  // idle: breathing, a lazy wag, a look about now and then, an ear flick
  const breathe = still ? 0 : Math.sin(t * 2.2) * 0.012;
  rig.body.scale.set(1 + breathe * 0.5, 1 + breathe, 1 + breathe * 0.5);
  if (rig.tail) rig.tail.rotation.y = still ? 0 : Math.sin(t * 5) * 0.35;
  if (rig.head) {
    const glance = still ? 0 : Math.sin(t * 0.7) * 0.18 + Math.sin(t * 0.23) * 0.1;
    rig.head.rotation.set(rig.headRest.x, rig.headRest.y + glance, rig.headRest.z + (still ? 0 : Math.sin(t * 0.9) * 0.05));
  }
  const flick = still ? 0 : Math.max(0, Math.sin(t * 1.3) ** 40) * 0.35;
  rig.ears.forEach((ear, index) => {
    ear.rotation.z = (index === 0 ? -1 : 1) * flick;
  });
  if (rig.hat) rig.hat.rotation.set(rig.hatRest.x, 0, rig.hatRest.z);
  if (rig.pom) rig.pom.position.y = rig.pomY;
  if (rig.aura) {
    rig.aura.rotation.y = still ? 0 : t * (party >= 0 ? 3.8 : 1.15);
    rig.aura.rotation.z = Math.sin(t * 0.8) * 0.15;
  }
  rig.root.position.y = 0;
  rig.root.rotation.y = 0;

  if (party < 0) return;
  // the hop: up and down twice, the first one high
  const hop = Math.abs(Math.sin(party * Math.PI * 2)) * (party < 0.5 ? 0.22 : 0.1) * (1 - party * 0.4);
  rig.root.position.y = still ? Math.sin(party * Math.PI) * 0.12 : hop;
  if (still) return;
  // a full spin during the first hop
  rig.root.rotation.y = ease(Math.min(1, party * 2)) * Math.PI * 2;
  if (rig.tail) rig.tail.rotation.y = Math.sin(t * 28) * 0.6;
  if (rig.head) rig.head.rotation.x = rig.headRest.x - Math.sin(party * Math.PI) * 0.35;
  rig.ears.forEach((ear, index) => {
    ear.rotation.z = (index === 0 ? -1 : 1) * Math.sin(party * Math.PI * 4) * 0.4;
  });
  const wobble = Math.sin(party * Math.PI * 6) * (1 - party);
  if (rig.hat) rig.hat.rotation.set(rig.hatRest.x + wobble * 0.25, 0, rig.hatRest.z + wobble * 0.35);
  if (rig.pom) rig.pom.position.y = rig.pomY + Math.abs(wobble) * 0.08;
}

export default function PetScene({ model, cheers, facing, style, tier = 1, onFail }: {
  model: string;
  cheers: number;
  facing: 1 | -1;
  style?: string | null;
  tier?: number;
  onFail(): void;
}) {
  const host = useRef<HTMLDivElement>(null);
  const cheer = useRef<() => void>(() => {});
  const failed = useRef(onFail);
  useEffect(() => {
    failed.current = onFail;
  }, [onFail]);

  useEffect(() => {
    const element = host.current;
    if (!element) return;
    let renderer: WebGLRenderer;
    try {
      renderer = new WebGLRenderer({ alpha: true, antialias: true, powerPreference: 'low-power' });
    } catch {
      failed.current();
      return;
    }
    renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    renderer.outputColorSpace = SRGBColorSpace;
    element.appendChild(renderer.domElement);

    const scene = new Scene();
    scene.add(new HemisphereLight(0xfff4e0, 0x2a3a30, 1.4));
    scene.add(new AmbientLight(0xffffff, 0.35));
    const sun = new DirectionalLight(0xffffff, 1.6);
    sun.position.set(2, 4, 3);
    scene.add(sun);
    const camera = new PerspectiveCamera(30, 1, 0.1, 50);

    let rigged: Rig | null = null;
    let disposed = false;
    let frame = 0;
    let last = 0;
    let cheeredAt = -Infinity;
    const still = reducedMotion();

    const resize = () => {
      const { width, height } = element.getBoundingClientRect();
      if (width === 0 || height === 0) return;
      renderer.setSize(width, height, false);
      camera.aspect = width / height;
      camera.updateProjectionMatrix();
    };
    const observer = new ResizeObserver(resize);
    observer.observe(element);
    resize();

    const draw = (now: number) => {
      if (!rigged) return;
      pose(rigged, now, cheeredAt, still);
      renderer.render(scene, camera);
    };
    const loop = (now: number) => {
      frame = 0;
      if (disposed) return;
      const celebrating = now - cheeredAt < CELEBRATE_MS;
      if (celebrating || now - last >= 1000 / IDLE_FPS) {
        last = now;
        draw(now);
      }
      // a still pet only draws while it hops
      if (!document.hidden && (!still || celebrating)) frame = requestAnimationFrame(loop);
    };
    const wake = () => {
      if (!frame && !disposed) frame = requestAnimationFrame(loop);
    };
    document.addEventListener('visibilitychange', wake);
    cheer.current = () => {
      cheeredAt = performance.now();
      wake();
    };

    loadModel(model).then((loaded) => {
      if (disposed) return;
      sharpen(loaded);
      rigged = rig(loaded);
      attachStyle(rigged, loaded, style, tier);
      // turned a little toward the middle of the table, and framed from its paws to the tip of its hat
      rigged.root.rotation.order = 'YXZ';
      const turn = new Group();
      turn.rotation.y = facing * 0.45;
      turn.add(rigged.root);
      scene.add(turn);
      const box = new Box3().setFromObject(loaded);
      const size = box.getSize(new Vector3());
      const center = box.getCenter(new Vector3());
      // headroom for the hop and the spin
      const tall = size.y * 1.2;
      const distance = tall / 2 / Math.tan((camera.fov * Math.PI) / 360);
      camera.position.set(center.x, center.y + tall * 0.22, center.z + distance);
      camera.lookAt(center.x, center.y + size.y * 0.04, center.z);
      draw(performance.now());
      wake();
    }).catch(() => {
      if (!disposed) failed.current();
    });

    return () => {
      disposed = true;
      if (frame) cancelAnimationFrame(frame);
      document.removeEventListener('visibilitychange', wake);
      observer.disconnect();
      scene.traverse((node) => {
        const mesh = node as Mesh;
        if (!mesh.isMesh) return;
        mesh.geometry.dispose();
        for (const material of ([] as Material[]).concat(mesh.material)) material.dispose();
      });
      renderer.dispose();
      renderer.domElement.remove();
    };
  }, [model, facing, style, tier]);

  useEffect(() => {
    if (cheers > 0) cheer.current();
  }, [cheers]);

  return <div ref={host} style={{ position: 'absolute', inset: 0 }} />;
}
