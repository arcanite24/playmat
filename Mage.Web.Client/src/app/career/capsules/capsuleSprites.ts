import { CAPSULE_ART, CAPSULE_SEAM } from './cabinetArt';

/**
 * The painted capsule, tinted to each set's colour once and kept: the top half multiplied by the colour, the frosted
 * half left as painted. The globe draws dozens of these every frame, so each colour and size is one canvas.
 */
let image: HTMLImageElement | null = null;
let ready: Promise<HTMLImageElement> | null = null;
const tinted = new Map<string, HTMLCanvasElement>();

export function capsuleImage(): Promise<HTMLImageElement> {
  if (!ready) {
    ready = new Promise((resolve, reject) => {
      const img = new Image();
      img.decoding = 'async';
      img.onload = () => {
        image = img;
        resolve(img);
      };
      img.onerror = () => {
        ready = null;
        reject(new Error('capsule art failed to load'));
      };
      img.src = CAPSULE_ART;
    });
  }
  return ready;
}

/** The capsule in `color` at `size` device pixels; null until the painting has loaded. */
export function capsuleSprite(color: string, size: number): HTMLCanvasElement | null {
  if (!image) return null;
  const px = Math.max(8, Math.round(size));
  const key = `${color}|${px}`;
  const kept = tinted.get(key);
  if (kept) return kept;
  const canvas = document.createElement('canvas');
  canvas.width = px;
  canvas.height = px;
  const ctx = canvas.getContext('2d');
  if (!ctx) return null;
  ctx.drawImage(image, 0, 0, px, px);
  ctx.globalCompositeOperation = 'multiply';
  ctx.fillStyle = color;
  ctx.fillRect(0, 0, px, px * CAPSULE_SEAM);
  // the multiply filled the corners too: keep only the capsule's own shape
  ctx.globalCompositeOperation = 'destination-in';
  ctx.drawImage(image, 0, 0, px, px);
  tinted.set(key, canvas);
  return canvas;
}
