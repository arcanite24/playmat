import {
  Anvil, ArrowUpToLine, Castle, Eye, EyeOff, Feather, FlaskConical, Footprints, HeartPulse, Shield, ShieldCheck,
  ShieldHalf, Skull, Sword, Swords, Users, Zap, type LucideIcon,
} from 'lucide-react';
import { memo, useMemo, useRef, type CSSProperties, type PointerEvent as ReactPointerEvent } from 'react';
import { keywordEffects, keywordMarks, type KeywordEffect, type MarkedKeyword } from '../../core/game/keywords';
import type { PermanentView } from '../../protocol/generated/views';
import { useSettings } from '../stores/settings';
import { CardFace } from '../ui/CardFace';
import { cardKey, stackOffsetRatio, type PermanentGroup } from './boardModel';
import { useFlip } from './flip';
import { useMatchUi } from './matchUi';
import { useStage } from './stageContext';
import styles from './PermanentStack.module.css';

export interface PermanentStackProps {
  group: PermanentGroup;
  width: number;
  sleeve: string;
  clickable: ReadonlySet<string>;
  selected: ReadonlySet<string>;
  /** clickable but not worth a glow (lands while you hold priority: tapping for mana is always possible) */
  quiet?: ReadonlySet<string>;
  attacking: ReadonlySet<string>;
  blocking: ReadonlySet<string>;
  /** opponents' attackers slide toward the player, ours toward the opponent */
  forward: 1 | -1;
  /** a target is being chosen: the candidates stand out from merely playable cards */
  targeting?: boolean;
  onClick(id: string): void;
  /** the permanent just clicked, drawn pressed until the server answers */
  pending?: string | null;
  /** while blockers are declared: a creature dropped on an attacker blocks it (false when nothing was sent) */
  onBlockDrop?(blockerId: string, attackerId: string): boolean;
  /** what a dragged creature can be dropped on (attackers by default; planeswalkers and battles while attacking) */
  dropTargets?: ReadonlySet<string>;
}

const CARD_RATIO = 88 / 63;
const DRAG_THRESHOLD = 12;

/** The drop target (an attacking creature, a planeswalker or battle to attack) under a screen point, if any. */
function attackerAt(x: number, y: number, targets: ReadonlySet<string>, self: string): string | null {
  for (const element of document.elementsFromPoint(x, y)) {
    const id = element.closest('[data-object-id]')?.getAttribute('data-object-id');
    if (id && id !== self && targets.has(id)) return id;
  }
  return null;
}

const KEYWORD_ICONS: Record<MarkedKeyword, LucideIcon> = {
  Flying: Feather,
  Reach: ArrowUpToLine,
  Menace: Users,
  Defender: Castle,
  'First strike': Sword,
  'Double strike': Swords,
  Deathtouch: Skull,
  Trample: Footprints,
  Lifelink: HeartPulse,
  Infect: FlaskConical,
  Vigilance: Eye,
  Haste: Zap,
  Indestructible: Anvil,
  Hexproof: EyeOff,
  Shroud: EyeOff,
  Ward: ShieldHalf,
  Protection: ShieldCheck,
};

/** The layer each keyword effect draws (PermanentStack.module.css); float moves the card itself. */
const FX_CLASS: Record<KeywordEffect, string> = {
  float: 'fxFloat',
  death: 'fxDeath',
  infect: 'fxInfect',
  dust: 'fxDust',
  life: 'fxLife',
  strike: 'fxStrike',
  doubleStrike: 'fxDoubleStrike',
  gold: 'fxGold',
  ward: 'fxWard',
  menace: 'fxMenace',
  haste: 'fxHaste',
};

/** A permanent (or a stack of identical lands/tokens) placed on the mat, with its attachments tucked behind. */
export const PermanentStack = memo(function PermanentStack(props: PermanentStackProps) {
  const { group, width } = props;
  // members turn individually; the slot keeps room for a turned card so nothing shifts when one taps
  const anyTapped = group.members.some((member) => member.tapped);
  const height = width * CARD_RATIO;
  const stackOffset = width * stackOffsetRatio(group);
  const attachOffset = width * 0.18;
  const slotWidth = (anyTapped ? height : width) + (group.members.length - 1) * stackOffset + group.attachments.length * attachOffset;
  const slotHeight = height + group.attachments.length * attachOffset;

  return (
    <div className={styles.slot} style={{ width: slotWidth, height: slotHeight }}>
      {group.attachments.map((attachment, index) => (
        <PlacedCard
          key={cardKey(attachment)}
          permanent={attachment}
          {...props}
          left={(group.attachments.length - 1 - index) * attachOffset}
          top={0}
          depth={index}
        />
      ))}
      {[...group.members].reverse().map((permanent, index, reversed) => (
        <PlacedCard
          key={cardKey(permanent)}
          permanent={permanent}
          {...props}
          left={group.attachments.length * attachOffset + (reversed.length - 1 - index) * stackOffset}
          top={group.attachments.length * attachOffset}
          depth={group.attachments.length + index}
          count={index === reversed.length - 1 && group.members.length > 1 ? group.members.length : undefined}
        />
      ))}
    </div>
  );
});

interface PlacedCardProps extends PermanentStackProps {
  permanent: PermanentView;
  left: number;
  top: number;
  depth: number;
  count?: number;
}

function PlacedCard({ permanent, width, sleeve, clickable, selected, quiet, attacking, blocking, forward, targeting, onClick, onBlockDrop, dropTargets, pending, left, top, depth, count }: PlacedCardProps) {
  const ref = useRef<HTMLDivElement>(null);
  const id = permanent.id!;
  const tapped = !!permanent.tapped;
  const height = width * CARD_RATIO;
  const isClickable = clickable.has(id);
  const isSelected = selected.has(id);
  const isAttacking = attacking.has(id);
  const isBlocking = blocking.has(id);
  const setZoom = useMatchUi((state) => state.setZoom);
  const keywords = useMemo(() => keywordMarks(permanent), [permanent]);
  const effects = useMemo(() => keywordEffects(keywords), [keywords]);
  const motion = useSettings((state) => state.settings.animations);
  useFlip(cardKey(permanent), ref, { rotation: tapped ? 90 : 0 });
  const stage = useStage();
  const setBlockDrag = useMatchUi((state) => state.setBlockDrag);
  // drag a creature onto an attacker to block it; a press without moving stays a click
  const canDrag = !!onBlockDrop && isClickable && !isAttacking;
  const gesture = useRef<{ startX: number; startY: number; moved: boolean } | null>(null);
  const suppressClick = useRef(false);

  function onPointerDown(event: ReactPointerEvent) {
    suppressClick.current = false;
    if (!canDrag || event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    gesture.current = { startX: event.clientX, startY: event.clientY, moved: false };
  }

  function onPointerMove(event: ReactPointerEvent) {
    const current = gesture.current;
    if (!current) return;
    if (!current.moved && Math.hypot(event.clientX - current.startX, event.clientY - current.startY) / stage.scale > DRAG_THRESHOLD) {
      current.moved = true;
      setZoom(null);
    }
    if (current.moved) setBlockDrag({ blockerId: id, attackerId: attackerAt(event.clientX, event.clientY, dropTargets ?? attacking, id) });
  }

  function endDrag(event: ReactPointerEvent, drop: boolean) {
    const current = gesture.current;
    gesture.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    if (!current?.moved) return;
    setBlockDrag(null);
    // the click that ends a drag is not a click on the card
    suppressClick.current = true;
    const attacker = drop ? attackerAt(event.clientX, event.clientY, dropTargets ?? attacking, id) : null;
    if (attacker) onBlockDrop?.(id, attacker);
  }

  const damage = permanent.damage ?? 0;
  const counters = (permanent.counters ?? []).filter((counter) => (counter.count ?? 0) > 0);
  const isCreature = (permanent.cardTypes ?? []).includes('CREATURE');
  const loyalty = permanent.loyalty || permanent.defense;
  // summoning sick: it can't attack or tap yet, so it dozes (the server's flag ignores haste; a hasty creature is awake)
  const asleep = isCreature && !!permanent.summoningSickness && !keywords.some((mark) => mark.name === 'Haste');
  const flying = effects.includes('float');
  const advance = isAttacking ? forward * height * 0.16 : isBlocking ? forward * height * 0.1 : 0;

  const style: CSSProperties = {
    width,
    height,
    left: left + (tapped ? (height - width) / 2 : 0),
    // turned in place around its centre, which stays at the slot middle
    top,
    zIndex: depth + 1,
    transform: `translateY(${advance}px) rotate(${tapped ? 90 : 0}deg)`,
    // how high a flier hovers grows with the card, so it reads at every board size
    ['--lift' as string]: `${Math.max(8, Math.round(height * 0.08))}px`,
    // a crowded row's small cards shrink their badges too, so they don't bury the neighbours
    ['--badge' as string]: Math.min(1, Math.max(0.7, width / 110)).toFixed(2),
  };

  return (
    <div
      ref={ref}
      className={[
        styles.card,
        isClickable && !quiet?.has(id) ? styles.clickable : '',
        isClickable && targeting && !isSelected ? styles.target : '',
        isSelected ? styles.selected : '',
        isAttacking ? styles.attacking : '',
        pending === id ? styles.pending : '',
      ].join(' ')}
      style={style}
      role={isClickable ? 'button' : undefined}
      tabIndex={isClickable ? 0 : undefined}
      aria-label={describe(permanent, { tapped, asleep, isAttacking, isBlocking, isSelected, keywords: keywords.map((mark) => (mark.gained ? `${mark.name.toLowerCase()} (gained)` : mark.name.toLowerCase())) }) + (isClickable && targeting && !isSelected ? ', can be targeted' : '')}
      aria-pressed={isClickable ? isSelected : undefined}
      onClick={() => {
        if (suppressClick.current) {
          suppressClick.current = false;
          return;
        }
        if (isClickable) onClick(id);
      }}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={(event) => endDrag(event, true)}
      onPointerCancel={(event) => endDrag(event, false)}
      onKeyDown={(event) => {
        if (isClickable && event.key === 'Enter') {
          event.preventDefault();
          onClick(id);
        }
      }}
      onPointerEnter={(event) => !gesture.current?.moved && setZoom({ card: permanent, x: event.clientX, y: event.clientY, sleeve })}
      onPointerLeave={() => setZoom(null)}
      data-object-id={id}
    >
      {flying && <span className={styles.flightShadow} data-still={motion ? undefined : ''} aria-hidden="true" />}
      <div className={[styles.body, flying ? styles.floating : '', asleep ? styles.asleep : ''].join(' ')} data-still={motion ? undefined : ''}>
        {effects.map((effect) => <span key={effect} className={[styles.fx, styles[FX_CLASS[effect]]].join(' ')} aria-hidden="true" />)}
        <CardFace card={permanent} sleeve={sleeve} size={width > 140 ? 'normal' : 'small'} />
      </div>

      <div className={styles.marks} style={{ transform: tapped ? 'rotate(-90deg)' : undefined }}>
        {isAttacking && <span className={[styles.mark, styles.markAttack].join(' ')} title="Attacking"><Swords size={13} aria-hidden="true" /></span>}
        {isBlocking && <span className={styles.mark} title="Blocking"><Shield size={13} aria-hidden="true" /></span>}
      </div>

      {asleep && (
        <span className={styles.sleep} style={{ transform: tapped ? 'rotate(-90deg)' : undefined }} data-still={motion ? undefined : ''} title="Summoning sick: it can't attack or tap until your next turn" aria-hidden="true">
          <span>z</span><span>z</span><span>Z</span>
        </span>
      )}

      {keywords.length > 0 && (
        // what it can do in combat, readable without the picture; granted keywords print solid
        <div className={styles.keywords} style={{ transform: tapped ? 'rotate(-90deg)' : undefined }} aria-hidden="true">
          {keywords.map((mark) => {
            const Icon = KEYWORD_ICONS[mark.name as MarkedKeyword];
            return (
              <span key={mark.name} className={mark.gained ? styles.keywordGained : styles.keyword} title={mark.gained ? `${mark.name} (gained from an effect)` : mark.name}>
                <Icon size={15} strokeWidth={2.4} />
              </span>
            );
          })}
        </div>
      )}

      {counters.length > 0 && (
        <ul className={styles.counters} aria-label="Counters">
          {counters.map((counter) => (
            <li key={counter.name} title={`${counter.count} ${counter.name}`}>
              <span>{counterLabel(counter.name ?? '')}</span>
              {(counter.count ?? 0) > 1 && <b>{counter.count}</b>}
            </li>
          ))}
        </ul>
      )}

      {isCreature && permanent.power !== undefined && (
        <span className={[styles.pt, ptTone(permanent)].join(' ')}>
          {permanent.power}/{permanent.toughness}
          {damage > 0 && <span className={styles.damage}>−{damage}</span>}
        </span>
      )}
      {!isCreature && loyalty && <span className={[styles.pt, styles.loyalty].join(' ')}>{loyalty}</span>}
      {count !== undefined && <span className={styles.count}>×{count}</span>}
    </div>
  );
}

function counterLabel(name: string): string {
  if (/^[+-]\d+\/[+-]\d+$/.test(name)) return name;
  return name.replace(/counter$/i, '').trim().slice(0, 10);
}

function ptTone(permanent: PermanentView): string {
  const base = permanent.original;
  const power = Number(permanent.power);
  const toughness = Number(permanent.toughness);
  const basePower = Number(base?.power ?? permanent.power);
  const baseToughness = Number(base?.toughness ?? permanent.toughness);
  if (Number.isNaN(power) || Number.isNaN(basePower)) return '';
  if (power > basePower || toughness > baseToughness) return styles.up;
  if (power < basePower || toughness < baseToughness) return styles.down;
  return '';
}

function describe(permanent: PermanentView, state: { tapped: boolean; asleep: boolean; isAttacking: boolean; isBlocking: boolean; isSelected: boolean; keywords: string[] }): string {
  const parts = [permanent.name ?? 'Permanent'];
  if (permanent.power !== undefined && (permanent.cardTypes ?? []).includes('CREATURE')) parts.push(`${permanent.power}/${permanent.toughness}`);
  parts.push(...state.keywords);
  if (state.tapped) parts.push('tapped');
  if (state.asleep) parts.push('summoning sick');
  if (state.isAttacking) parts.push('attacking');
  if (state.isBlocking) parts.push('blocking');
  if (state.isSelected) parts.push('chosen');
  return parts.join(', ');
}
