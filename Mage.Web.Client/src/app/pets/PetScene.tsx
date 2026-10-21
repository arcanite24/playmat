import { useEffect, useRef } from 'react';
import {
  AmbientLight,
  Box3,
  DirectionalLight,
  Group,
  HemisphereLight,
  type Material,
  type Mesh,
  NearestFilter,
  type Object3D,
  PerspectiveCamera,
  Scene,
  SRGBColorSpace,
  type Texture,
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
}

const models = new Map<string, Promise<Object3D>>();

function loadModel(url: string): Promise<Object3D> {
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
  };
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

export default function PetScene({ model, cheers, facing, onFail }: { model: string; cheers: number; facing: 1 | -1; onFail(): void }) {
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
  }, [model, facing]);

  useEffect(() => {
    if (cheers > 0) cheer.current();
  }, [cheers]);

  return <div ref={host} style={{ position: 'absolute', inset: 0 }} />;
}
