import { useEffect, useRef } from 'react';
import styles from './Pull.module.css';

/** How big a capsule's opening is: an ordinary one, one that finishes a set, one that finishes the machine. */
export type Tier = 'common' | 'set' | 'machine';

interface Mote {
  x: number;
  y: number;
  vx: number;
  vy: number;
  life: number;
  age: number;
  size: number;
  color: string;
  /** a spark is drawn as a short streak along its way; a mote as a soft dot */
  spark: boolean;
}

const COUNT: Record<Tier, number> = { common: 22, set: 70, machine: 130 };

/**
 * Light spilling out of an opened capsule: motes in its colour and white sparks, thrown out from `at` and falling
 * back, more of them for a bigger opening. One canvas over the screen for about a second and a half, then nothing.
 */
export function Burst({ at, color, tier }: { at: { x: number; y: number }; color: string; tier: Tier }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const el = canvas.current;
    const ctx = el?.getContext('2d');
    if (!el || !ctx) return;
    const ratio = Math.min(2, window.devicePixelRatio || 1);
    el.width = Math.round(window.innerWidth * ratio);
    el.height = Math.round(window.innerHeight * ratio);
    const reach = Math.min(window.innerWidth, window.innerHeight) * (tier === 'common' ? 0.5 : tier === 'set' ? 0.85 : 1.1);
    const palette = tier === 'machine' ? ['#f3d27a', '#ffe9b0', '#ffffff', color] : [color, color, '#ffffff', '#fff3d6'];
    const motes: Mote[] = Array.from({ length: COUNT[tier] }, (_, i) => {
      const angle = Math.random() * Math.PI * 2;
      const speed = reach * (0.6 + Math.random() * 1.1);
      const vy = Math.sin(angle) * speed;
      return {
        x: at.x,
        y: at.y,
        vx: Math.cos(angle) * speed,
        // a capsule opens upward: less of it goes down
        vy: (vy > 0 ? vy * 0.4 : vy) - reach * 0.3,
        life: 0.7 + Math.random() * (tier === 'common' ? 0.5 : 0.9),
        age: 0,
        size: (tier === 'machine' ? 2.6 : 2) + Math.random() * 3.5,
        color: palette[i % palette.length],
        spark: i % 3 === 0,
      };
    });
    let last = performance.now();
    let frame = requestAnimationFrame(function tick(now) {
      const dt = Math.min(0.05, (now - last) / 1000);
      last = now;
      ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
      ctx.clearRect(0, 0, el.width, el.height);
      ctx.globalCompositeOperation = 'lighter';
      let alive = 0;
      for (const mote of motes) {
        mote.age += dt;
        if (mote.age >= mote.life) continue;
        alive++;
        mote.vx *= 1 - 2.6 * dt;
        mote.vy = mote.vy * (1 - 2.6 * dt) + 520 * dt;
        mote.x += mote.vx * dt;
        mote.y += mote.vy * dt;
        const fade = 1 - mote.age / mote.life;
        ctx.globalAlpha = fade * fade;
        ctx.strokeStyle = mote.color;
        ctx.fillStyle = mote.color;
        if (mote.spark) {
          ctx.lineWidth = mote.size * 0.55;
          ctx.lineCap = 'round';
          ctx.beginPath();
          ctx.moveTo(mote.x, mote.y);
          ctx.lineTo(mote.x - mote.vx * 0.035, mote.y - mote.vy * 0.035);
          ctx.stroke();
        } else {
          ctx.beginPath();
          ctx.arc(mote.x, mote.y, mote.size * (0.5 + fade * 0.5), 0, Math.PI * 2);
          ctx.fill();
        }
      }
      frame = alive ? requestAnimationFrame(tick) : 0;
      if (!alive) ctx.clearRect(0, 0, el.width, el.height);
    });
    return () => cancelAnimationFrame(frame);
  }, [at, color, tier]);
  return <canvas ref={canvas} className={styles.burst} aria-hidden="true" />;
}
