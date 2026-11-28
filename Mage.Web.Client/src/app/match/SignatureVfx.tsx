import { useEffect, useRef, type CSSProperties } from 'react';
import { lightningPath, seeded, type Point, type SignatureEffect } from './signatureShapes';
import styles from './SignatureVfx.module.css';

/**
 * The signature effects' primitives (cardVfx.ts picks one per spell): plain elements and SVG with CSS animations,
 * transform and opacity only, over the board and never in the pointer's way. Positions are stage pixels
 * (signatureShapes.ts has the effect's shape and timings).
 */

const pathOf = (points: Point[]) => points.map((point, index) => `${index === 0 ? 'M' : 'L'}${point.x.toFixed(1)} ${point.y.toFixed(1)}`).join(' ');

const at = (point: Point): CSSProperties => ({ left: point.x, top: point.y });

const vars = (values: Record<string, string | number>) => values as CSSProperties;

function Lightning({ effect }: { effect: SignatureEffect }) {
  const targets = effect.targets.length > 0 ? effect.targets : effect.caster ? [effect.caster] : [];
  return (
    <>
      <svg className={styles.svg} width={effect.width} height={effect.height} viewBox={`0 0 ${effect.width} ${effect.height}`}>
        {targets.map((target, index) => {
          const main = lightningPath(effect.from, target, effect.key * 7 + index, effect.big ? 11 : 8, effect.big ? 44 : 30);
          // a branch splits off a third of the way and dies out
          const split = main[Math.floor(main.length / 3)];
          const branchEnd = { x: split.x + (target.x - effect.from.x) * 0.25 + 40, y: split.y + (target.y - effect.from.y) * 0.25 - 30 };
          const branch = lightningPath(split, branchEnd, effect.key * 13 + index, 4, 18);
          return (
            <g key={index} className={styles.bolt}>
              <path d={pathOf(main)} className={styles.boltGlow} />
              <path d={pathOf(main)} className={styles.boltCore} />
              <path d={pathOf(branch)} className={styles.boltBranch} />
            </g>
          );
        })}
      </svg>
      {targets.map((target, index) => <span key={index} className={styles.flash} style={at(target)} />)}
    </>
  );
}

/** A ball of fire thrown from the stack, bursting where it lands. */
function Fireball({ from, to, big }: { from: Point; to: Point; big: boolean }) {
  const ref = useRef<HTMLSpanElement>(null);
  useEffect(() => {
    ref.current?.animate(
      [{ translate: '0 0', scale: '0.5' }, { translate: `${to.x - from.x}px ${to.y - from.y}px`, scale: big ? '1.4' : '1' }],
      { duration: 420, easing: 'cubic-bezier(0.55, 0, 0.9, 0.5)', fill: 'forwards' },
    );
  }, [from.x, from.y, to.x, to.y, big]);
  return (
    <>
      <span ref={ref} className={styles.fireball} style={at(from)} />
      <span className={[styles.explosion, big ? styles.bigger : ''].join(' ')} style={at(to)}>
        {EMBERS.map((ember, index) => <span key={index} className={styles.ember} style={vars({ '--dx': `${ember.dx}px`, '--dy': `${ember.dy}px` })} />)}
      </span>
    </>
  );
}

const ring = (count: number, distance: number, wobble = 0.35) => Array.from({ length: count }, (_, i) => {
  const angle = (i / count) * Math.PI * 2 + (i % 2) * wobble;
  const reach = distance * (0.7 + ((i * 37) % 10) / 30);
  return { dx: Math.round(Math.cos(angle) * reach), dy: Math.round(Math.sin(angle) * reach), angle };
});

const EMBERS = ring(12, 90);
const SHARDS = ring(10, 120, 0.5);
const MOTES = ring(10, 70, 0.2);
const LEAVES = ring(8, 80, 0.6);

function Effect({ effect }: { effect: SignatureEffect }) {
  const ink = styles[`ink-${effect.ink}`];
  const big = effect.big ? styles.bigger : '';
  const where = effect.targets.length > 0 ? effect.targets : effect.caster ? [effect.caster] : [effect.from];
  switch (effect.fx) {
    case 'lightning':
      return <div className={ink}><Lightning effect={effect} /></div>;
    case 'fire':
      return <div className={ink}>{where.map((to, index) => <Fireball key={index} from={effect.from} to={to} big={effect.big} />)}</div>;
    case 'beam':
      return (
        <div className={ink}>
          {where.map((point, index) => (
            <span key={index} className={[styles.beam, big].join(' ')} style={{ left: point.x, top: point.y, height: point.y + 120 }}>
              <span className={styles.beamFoot} />
              {MOTES.slice(0, 6).map((mote, mi) => <span key={mi} className={styles.rise} style={vars({ '--dx': `${mote.dx * 0.6}px`, '--delay': `${mi * 60}ms` })} />)}
            </span>
          ))}
        </div>
      );
    case 'counter':
      return (
        <div className={ink}>
          {where.map((point, index) => (
            <span key={index} className={[styles.glyph, big].join(' ')} style={at(point)}>
              <svg viewBox="-60 -60 120 120" className={styles.glyphRing}>
                <circle r="50" className={styles.glyphOuter} />
                <circle r="38" className={styles.glyphInner} />
                <path d="M-26 -26 L26 26 M26 -26 L-26 26" className={styles.glyphCross} />
              </svg>
              {SHARDS.map((shard, si) => <span key={si} className={styles.shard} style={vars({ '--dx': `${shard.dx}px`, '--dy': `${shard.dy}px`, '--turn': `${shard.angle}rad` })} />)}
            </span>
          ))}
        </div>
      );
    case 'sweep':
      return (
        <div className={ink}>
          <span className={[styles.sweep, big].join(' ')} style={vars({ width: effect.width, height: effect.height * 0.45, '--travel': `${effect.height * 1.3}px` })} />
          <span className={styles.sweepVeil} style={{ width: effect.width, height: effect.height }} />
        </div>
      );
    case 'tendrils':
      return (
        <div className={ink}>
          {where.map((point, index) => (
            <span key={index} className={[styles.tendrils, big].join(' ')} style={at(point)}>
              <svg viewBox="-100 -100 200 200" className={styles.tendrilSvg}>
                {[0, 1, 2, 3, 4, 5].map((i) => {
                  const angle = (i / 6) * Math.PI * 2 + 0.4;
                  const x = Math.cos(angle) * 95;
                  const y = Math.sin(angle) * 95;
                  return <path key={i} d={`M${x.toFixed(0)} ${y.toFixed(0)} Q${(x * 0.2 - y * 0.4).toFixed(0)} ${(y * 0.2 + x * 0.4).toFixed(0)} 0 0`} className={styles.tendril} style={vars({ '--delay': `${i * 40}ms` })} />;
                })}
              </svg>
              <span className={styles.crush} />
            </span>
          ))}
        </div>
      );
    case 'growth':
      return (
        <div className={ink}>
          {where.map((point, index) => (
            <span key={index} className={[styles.growth, big].join(' ')} style={at(point)}>
              <span className={styles.growRing} />
              <span className={[styles.growRing, styles.later].join(' ')} />
              {LEAVES.map((leaf, li) => <span key={li} className={styles.leaf} style={vars({ '--dx': `${leaf.dx}px`, '--dy': `${leaf.dy - 30}px`, '--turn': `${leaf.angle * 2}rad` })} />)}
            </span>
          ))}
        </div>
      );
    case 'halo':
      return (
        <div className={ink}>
          {where.map((point, index) => (
            <span key={index} className={styles.halo} style={at(point)}>
              <span className={styles.haloRing} />
              {MOTES.slice(0, 8).map((mote, mi) => <span key={mi} className={styles.glint} style={vars({ '--dx': `${mote.dx}px`, '--dy': `${mote.dy * 0.4 - 70}px`, '--delay': `${300 + mi * 50}ms` })} />)}
            </span>
          ))}
        </div>
      );
    case 'bounce':
      return (
        <div className={ink}>
          {where.map((point, index) => (
            <span key={index} className={styles.whirl} style={at(point)}>
              {[0, 1, 2].map((i) => <span key={i} className={styles.whirlRing} style={vars({ '--delay': `${i * 90}ms`, '--size': `${100 + i * 50}px` })} />)}
            </span>
          ))}
        </div>
      );
    case 'insight': {
      const point = effect.caster ?? effect.from;
      return (
        <div className={ink}>
          <span className={[styles.orbit, big].join(' ')} style={at(point)}>
            {[0, 1, 2, 3, 4, 5].map((i) => <span key={i} className={styles.rune} style={vars({ '--start': `${i * 60}deg`, '--delay': `${i * 45}ms` })} />)}
          </span>
        </div>
      );
    }
    case 'mill': {
      const spills = effect.spills.length > 0 ? effect.spills : where.map((point) => ({ at: point, count: effect.big ? 8 : 4 }));
      return (
        <div className={ink}>
          {spills.map((spill, index) => (
            <span key={index} className={styles.spill} style={at(spill.at)}>
              {Array.from({ length: Math.min(12, Math.max(2, spill.count)) }, (_, i) => {
                const random = seeded(effect.key * 31 + index * 7 + i);
                return (
                  <span
                    key={i}
                    className={styles.sheet}
                    style={vars({ '--dx': `${Math.round((random() - 0.5) * 260)}px`, '--dy': `${Math.round(40 + random() * 120)}px`, '--turn': `${Math.round((random() - 0.5) * 540)}deg`, '--delay': `${i * 55}ms` })}
                  />
                );
              })}
            </span>
          ))}
        </div>
      );
    }
    case 'mind':
    case 'drain':
    case 'ritual': {
      // streams of motes: from the targets (or the stack) to the caster; a ritual's come from all around
      const sink = effect.caster ?? effect.from;
      const sources = effect.fx === 'ritual' ? MOTES.map((mote) => ({ x: sink.x + mote.dx * 2.4, y: sink.y + mote.dy * 2.4 })) : where.filter((point) => point !== sink);
      const list = sources.length > 0 ? sources : [effect.from];
      return (
        <div className={ink}>
          {list.flatMap((source, index) => [0, 1, 2, 3, 4, 5].map((i) => (
            <span key={`${index}-${i}`} className={styles.stream} style={vars({ left: source.x, top: source.y, '--dx': `${sink.x - source.x}px`, '--dy': `${sink.y - source.y}px`, '--delay': `${i * 70 + index * 30}ms` })} />
          )))}
          <span className={styles.gather} style={at(sink)} />
        </div>
      );
    }
    case 'summon': {
      const point = effect.targets[0] ?? effect.caster ?? effect.from;
      return (
        <div className={ink}>
          <span className={[styles.summon, big].join(' ')} style={at(point)}>
            {[0, 1, 2, 3, 4, 5, 6, 7].map((i) => <span key={i} className={styles.ray} style={vars({ '--turn': `${i * 45}deg` })} />)}
            {MOTES.map((mote, mi) => <span key={mi} className={styles.rise} style={vars({ '--dx': `${mote.dx}px`, '--delay': `${mi * 50}ms` })} />)}
          </span>
        </div>
      );
    }
    case 'claws':
      return (
        <div className={ink}>
          {where.map((point, index) => (
            <span key={index} className={styles.claws} style={at(point)}>
              {[-1, 0, 1].map((i) => <span key={i} className={styles.slash} style={vars({ '--offset': `${i * 26}px`, '--delay': `${(i + 1) * 50}ms` })} />)}
            </span>
          ))}
        </div>
      );
  }
}

/** The signature effects playing now, and the table's shake for the big ones. */
export function SignatureVfx({ effects }: { effects: readonly SignatureEffect[] }) {
  return <>{effects.map((effect) => <Effect key={effect.key} effect={effect} />)}</>;
}
