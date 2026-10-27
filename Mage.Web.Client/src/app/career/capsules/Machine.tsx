import { forwardRef, useCallback, useEffect, useImperativeHandle, useLayoutEffect, useRef, useState, type CSSProperties, type PointerEvent as ReactPointerEvent, type Ref } from 'react';
import { playCareerCue } from '../careerSound';
import { Cabinet } from './Cabinet';
import { cabinetArtOf, CAPSULE_ART, type CabinetArt } from './cabinetArt';
import { CapsuleHeap } from './capsulePhysics';
import { capsuleImage, capsuleSprite } from './capsuleSprites';
import { TokenIcon } from './TokenIcon';
import styles from './Machine.module.css';

export interface MachineHandle {
  /** the crank moved on a notch: the plate under the globe stirs the capsules */
  stir(strength: number): void;
  /** capsules out of the globe, down the chute, one after another; resolves where the last one came to rest */
  deliver(colors: readonly string[]): Promise<DOMRect | null>;
  /** the cabinet rocks on its feet */
  jolt(power?: number): void;
}

export interface MachineProps {
  /** the machine's id: its painting, and the seed for where its capsules lie */
  id: string;
  body: string;
  trim: string;
  /** one colour for each capsule still inside */
  capsules: readonly string[];
  /** alive: the heap settles now and then, and a tap on the glass knocks the capsules about */
  live?: boolean;
  /** reduced motion: the capsules lie still and everything happens at once */
  still?: boolean;
  /** the crank's turn, in degrees */
  crank?: number;
  /** the crank is waiting for the player: it nudges now and then */
  inviting?: boolean;
  /** a token drops into the slot */
  inserting?: boolean;
  onCrankDown?(event: ReactPointerEvent<HTMLElement>): void;
  crankRef?: Ref<HTMLElement>;
  className?: string;
  style?: CSSProperties;
  label: string;
}

const pct = (value: number, of: number) => `${(value / of) * 100}%`;
const wait = (ms: number) => new Promise((done) => setTimeout(done, ms));

/**
 * A capsule machine: the painted cabinet (art/cabinet.mjs) with its capsules alive in the globe. They're balls
 * (capsulePhysics.ts) drawn with the painted capsule tinted to each set's colour, under the painting's own glass,
 * which is laid over them again in screen so its reflections sit on top. The crank wheel turns on its mount, a
 * token drops in the slot, and what the machine gives rolls out of the chute. A machine without a painting is
 * drawn (Cabinet).
 */
export const Machine = forwardRef(function Machine(props: MachineProps, ref: Ref<MachineHandle>) {
  const art = cabinetArtOf(props.id);
  const [failed, setFailed] = useState(false);
  const drawn = !art || failed;
  const fail = useCallback(() => setFailed(true), []);
  useImperativeHandle(drawn ? ref : undefined, () => fallbackHandle, []);
  if (drawn) {
    return (
      <Cabinet seed={props.id} body={props.body} trim={props.trim} capsules={props.capsules} crank={props.crank} inserting={props.inserting}
        onCrankDown={props.onCrankDown as ((event: ReactPointerEvent<SVGGElement>) => void) | undefined}
        crankRef={props.crankRef as Ref<SVGGElement> | undefined} className={props.className} style={props.style} label={props.label} />
    );
  }
  return <PaintedMachine {...props} art={art} ref={ref} onFail={fail} />;
});

// the drawn cabinet has no globe to stir and no chute to roll down
const fallbackHandle: MachineHandle = {
  stir() {},
  deliver: () => wait(500).then(() => null),
  jolt() {},
};

interface Delivered {
  id: number;
  color: string;
  /** more follow: this one rolls out of the way */
  out: boolean;
}

const PaintedMachine = forwardRef(function PaintedMachine(
  { id, capsules, live = false, still = false, crank = 0, inviting, inserting, onCrankDown, crankRef, className, style, label, art, onFail }: MachineProps & { art: CabinetArt; onFail(): void },
  ref: Ref<MachineHandle>,
) {
  const root = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const heap = useRef<CapsuleHeap | null>(null);
  const frame = useRef(0);
  const visible = useRef(true);
  const [delivered, setDelivered] = useState<Delivered[]>([]);
  const chute = useRef<HTMLSpanElement>(null);
  const serial = useRef(0);
  const { globe, width: W, height: H } = art;
  const top = globe.y - globe.r;

  const draw = useCallback(() => {
    const el = canvas.current;
    const pile = heap.current;
    const ctx = el?.getContext('2d');
    if (!el || !pile || !ctx) return;
    const scale = el.width / (globe.r * 2);
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, el.width, el.height);
    ctx.setTransform(scale, 0, 0, scale, -(globe.x - globe.r) * scale, -top * scale);
    ctx.save();
    ctx.beginPath();
    ctx.arc(globe.x, globe.y, globe.r, 0, Math.PI * 2);
    ctx.clip();
    const r = pile.radius;
    const size = r * 2.08;
    for (const ball of pile.balls) {
      const sprite = capsuleSprite(ball.color, size * scale);
      if (!sprite) continue;
      ctx.save();
      ctx.translate(ball.x, ball.y);
      ctx.rotate(ball.a);
      ctx.drawImage(sprite, -size / 2, -size / 2, size, size);
      ctx.restore();
    }
    // the far side of the glass is darker, and so is the heap's bottom, under the collar's lip
    const depth = ctx.createRadialGradient(globe.x - globe.r * 0.2, globe.y - globe.r * 0.25, globe.r * 0.35, globe.x, globe.y, globe.r);
    depth.addColorStop(0, 'rgb(6 12 9 / 0)');
    depth.addColorStop(1, 'rgb(6 12 9 / 0.62)');
    ctx.fillStyle = depth;
    ctx.fillRect(globe.x - globe.r, top, globe.r * 2, globe.floor - top);
    const lip = ctx.createLinearGradient(0, globe.floor - r * 1.4, 0, globe.floor);
    lip.addColorStop(0, 'rgb(6 12 9 / 0)');
    lip.addColorStop(1, 'rgb(6 12 9 / 0.5)');
    ctx.fillStyle = lip;
    ctx.fillRect(globe.x - globe.r, globe.floor - r * 1.4, globe.r * 2, r * 1.4);
    ctx.restore();
  }, [globe, top]);

  const run = useCallback(() => {
    if (frame.current || still) return;
    let last = performance.now();
    const tick = (now: number) => {
      const pile = heap.current;
      // a long frame (a hidden tab) shouldn't throw the capsules through the glass
      const dt = Math.min(1 / 30, (now - last) / 1000);
      last = now;
      if (!pile || !visible.current) {
        frame.current = 0;
        return;
      }
      pile.step(dt);
      draw();
      frame.current = pile.asleep ? 0 : requestAnimationFrame(tick);
    };
    frame.current = requestAnimationFrame(tick);
  }, [draw, still]);

  // fill the globe: on first sight, and when the capsules left change for some other reason than our own deliveries
  const key = capsules.join(',');
  useEffect(() => {
    const pile = heap.current;
    if (pile && pile.balls.filter((ball) => !ball.leaving).length === capsules.length) return;
    const next = new CapsuleHeap(globe);
    next.fill(capsules, id);
    heap.current = next;
    draw();
    // capsules is read through key
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, id, globe, draw]);

  useEffect(() => {
    let gone = false;
    capsuleImage().then(() => !gone && draw(), () => !gone && onFail());
    return () => {
      gone = true;
    };
  }, [draw, onFail]);

  // the canvas's own pixels follow its size on screen
  useLayoutEffect(() => {
    const el = canvas.current;
    if (!el) return;
    const fit = () => {
      const ratio = Math.min(2, window.devicePixelRatio || 1);
      const width = Math.max(1, Math.round(el.clientWidth * ratio));
      if (el.width === width) return;
      el.width = width;
      el.height = Math.round((width * (globe.floor - top)) / (globe.r * 2));
      draw();
    };
    fit();
    const observer = new ResizeObserver(fit);
    observer.observe(el);
    return () => observer.disconnect();
  }, [draw, globe, top]);

  // nothing runs off screen or in a hidden tab
  useEffect(() => {
    const el = root.current;
    if (!el) return;
    const observer = new IntersectionObserver(([entry]) => {
      visible.current = entry.isIntersecting && !document.hidden;
      if (visible.current && heap.current && !heap.current.asleep) run();
    });
    observer.observe(el);
    const onVisibility = () => {
      visible.current = !document.hidden;
    };
    document.addEventListener('visibilitychange', onVisibility);
    return () => {
      observer.disconnect();
      document.removeEventListener('visibilitychange', onVisibility);
      cancelAnimationFrame(frame.current);
      frame.current = 0;
    };
  }, [run]);

  // a machine on its own: now and then a capsule shifts in the heap
  useEffect(() => {
    if (!live || still) return;
    let timer = 0;
    const later = () => {
      timer = window.setTimeout(() => {
        heap.current?.shift();
        run();
        later();
      }, 3500 + Math.random() * 5000);
    };
    later();
    return () => clearTimeout(timer);
  }, [live, still, run]);

  const jolt = useCallback((power = 1) => {
    if (still) return;
    root.current?.animate(
      [
        { transform: 'none' },
        { transform: `translateY(${2 * power}px) rotate(${-0.5 * power}deg)` },
        { transform: `translateY(${-1 * power}px) rotate(${0.35 * power}deg)` },
        { transform: 'none' },
      ],
      { duration: 260 + power * 60, easing: 'cubic-bezier(0.3, 0.7, 0.4, 1)' },
    );
  }, [still]);

  useImperativeHandle(ref, () => ({
    stir(strength) {
      if (still) return;
      heap.current?.stir(strength);
      run();
    },
    jolt,
    async deliver(colors) {
      const pile = heap.current;
      setDelivered([]);
      for (let at = 0; at < colors.length; at++) {
        pile?.release(colors[at]);
        run();
        if (!still) await wait(at === 0 ? 300 : 120);
        const one = { id: ++serial.current, color: colors[at], out: at < colors.length - 1 };
        setDelivered((before) => [...before, one]);
        if (!still) {
          window.setTimeout(() => {
            jolt(colors.length > 1 ? 0.6 : 1.2);
            playCareerCue('land');
          }, 330);
          await wait(at === colors.length - 1 ? 640 : 170);
        }
      }
      if (colors.length > 1 && !still) await wait(300);
      const last = chute.current?.lastElementChild as HTMLElement | null | undefined;
      return last?.getBoundingClientRect() ?? null;
    },
  }), [still, run, jolt]);

  const knockAt = (event: ReactPointerEvent<HTMLCanvasElement>) => {
    if (!live || still || !heap.current) return;
    const box = event.currentTarget.getBoundingClientRect();
    const scale = (globe.r * 2) / box.width;
    heap.current.knock({ x: globe.x - globe.r + (event.clientX - box.left) * scale, y: top + (event.clientY - box.top) * scale });
    playCareerCue('glassTap');
    run();
  };

  const { mount, chute: mouth, slot } = art;
  return (
    <div ref={root} className={[styles.machine, still ? styles.still : '', className ?? ''].join(' ')} style={{ ...style, aspectRatio: `${W} / ${H}` }} role="img" aria-label={label}>
      <img className={styles.body} src={art.body} alt="" draggable={false} onError={onFail} />
      <canvas
        ref={canvas}
        className={styles.globe}
        style={{ left: pct(globe.x - globe.r, W), top: pct(top, H), width: pct(globe.r * 2, W), height: pct(globe.floor - top, H) }}
        onPointerDown={knockAt}
        aria-hidden="true"
      />
      {/* the painting's glass again, over the capsules: only its reflections show */}
      <img
        className={styles.glass}
        src={art.body}
        alt=""
        draggable={false}
        style={{ clipPath: `ellipse(${pct(globe.r, W)} ${pct(globe.r, H)} at ${pct(globe.x, W)} ${pct(globe.y, H)})` }}
      />
      <span className={styles.slot} style={{ left: pct(slot.x, W), top: pct(slot.y, H) }} aria-hidden="true">
        {inserting && <span className={styles.coin}><TokenIcon size={64} /></span>}
      </span>
      <span
        ref={chute}
        className={styles.chute}
        style={{ left: pct(mouth.left, W), top: pct(mouth.top, H), width: pct(mouth.right - mouth.left, W), height: pct(mouth.bottom - mouth.top, H) }}
        aria-hidden="true"
      >
        {delivered.map((one) => (
          <span key={one.id} className={styles.delivered} data-out={one.out || undefined}>
            <PaintedCapsule color={one.color} />
          </span>
        ))}
      </span>
      <span
        ref={crankRef as Ref<HTMLSpanElement>}
        className={[styles.crank, onCrankDown ? styles.crankLive : '', inviting && !still ? styles.invite : ''].join(' ')}
        style={{ left: pct(mount.x - mount.r, W), top: pct(mount.y - mount.r, H), width: pct(mount.r * 2, W) }}
        onPointerDown={onCrankDown}
        aria-hidden="true"
      >
        <img src={art.crank} alt="" draggable={false} style={{ transform: `rotate(${crank}deg)` }} />
      </span>
    </div>
  );
});

/** One painted capsule, its top tinted to `color`; the halves can come apart (the pull sequence opens it). */
export function PaintedCapsule({ color, className, style }: { color: string; className?: string; style?: CSSProperties }) {
  return (
    <span className={[styles.capsule, className ?? ''].join(' ')} style={{ ...style, '--capsule': color, '--capsule-art': `url(${CAPSULE_ART})` } as CSSProperties} aria-hidden="true">
      <span className={styles.half} data-half="bottom" />
      <span className={styles.half} data-half="top" />
    </span>
  );
}
