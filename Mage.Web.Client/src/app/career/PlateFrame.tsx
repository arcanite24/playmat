import { useState, type ReactNode } from 'react';
import { useCatalogItem } from '../ui/cosmeticsCatalog';
import { artUrl, usePaintedArt } from './careerArt';
import { frameLook, frameShapes, type FrameShape } from './frameModel';
import styles from './LookParts.module.css';

function paint(shape: FrameShape, ring: string, accent: string): string {
  return shape.paint === 'ring' ? ring : accent;
}

/**
 * An earned plate frame (B296) round its child, an avatar disc; anything else (no frame, the supporter ring) draws
 * nothing. With painted art on it's the frame's painted ring (art/specs/plate-frame.json), else, or when the picture
 * won't load, the ring drawn here from its style.
 */
export function PlateFrame({ frame, children, className }: { frame: string | null | undefined; children: ReactNode; className?: string }) {
  const look = frameLook(useCatalogItem('frame', frame));
  const paintedArt = usePaintedArt();
  const [failed, setFailed] = useState<string | null>(null);
  if (!look || !frame) return <>{children}</>;
  if (paintedArt && failed !== frame) {
    return (
      <span className={[styles.frame, className ?? ''].join(' ')} data-frame={look.style}>
        {children}
        <img className={styles.framePainted} src={artUrl('plate-frame', frame)} alt="" draggable={false} onError={() => setFailed(frame)} />
      </span>
    );
  }
  return (
    <span className={[styles.frame, className ?? ''].join(' ')} data-frame={look.style}>
      {children}
      <svg className={styles.frameArt} viewBox="0 0 100 100" aria-hidden="true">
        {frameShapes(look.style).map((shape, index) => {
          const color = paint(shape, look.ring, look.accent);
          switch (shape.type) {
            case 'circle':
              return <circle key={index} cx="50" cy="50" r={shape.r} fill="none" stroke={color} strokeWidth={shape.width} strokeDasharray={shape.dash} />;
            case 'dot':
              return <circle key={index} cx={shape.x} cy={shape.y} r={shape.r} fill={color} />;
            case 'line':
              return <line key={index} x1={shape.x1} y1={shape.y1} x2={shape.x2} y2={shape.y2} stroke={color} strokeWidth={shape.width} strokeLinecap="round" />;
            default:
              return <path key={index} d={shape.d} fill={color} />;
          }
        })}
      </svg>
    </span>
  );
}
