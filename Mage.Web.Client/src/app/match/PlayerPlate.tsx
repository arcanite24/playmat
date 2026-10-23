import { useEffect, useRef, useState, type CSSProperties } from 'react';
import { Heart, Swords } from 'lucide-react';
import { COMMANDER_DAMAGE_LETHAL } from '../../core/game/commander';
import { POOL_MANA, poolId } from '../../core/game/payment';
import { ROPE_SECONDS, ropeSecondsLeft, ropeVisible } from '../../core/game/rope';
import type { PlayerLook, PlayerView } from '../../protocol/generated/views';
import { Art } from '../career/Art';
import { ShowcaseBadges } from '../career/LookParts';
import { PlateFrame } from '../career/PlateFrame';
import { titleColor, titleName } from '../career/progressModel';
import { useCatalogItem } from '../ui/cosmeticsCatalog';
import { useT } from '../i18n';
import './matchMessages';
import { ManaCost } from '../ui/ManaCost';
import { sigilOf } from '../../core/social/identity';
import { LevelBadge } from '../social/LevelBadge';
import { usePresence } from './presence';
import { isMatchObjectKey, preventMatchObjectMouseFocus } from './keyboard';
import styles from './PlayerPlate.module.css';

export interface PlayerPlateProps {
  player: PlayerView;
  isMe: boolean;
  /** the player is a legal target right now */
  targetable: boolean;
  selected: boolean;
  /** the player must answer something right now */
  deciding: boolean;
  /** the player's sleeve color: their seat is dyed to match their cards */
  sleeve?: string;
  onClick(): void;
  /** while paying a cost: what is still owed */
  toPay?: string | null;
  /** board ids of the floating mana that can pay right now (see poolId) */
  payable?: ReadonlySet<string>;
  /** spend a kind of floating mana (its board id) */
  onPay?(id: string): void;
  /** combat damage taken from each commander (Commander games) */
  commanderDamage?: readonly { name: string; amount: number }[];
  /** outside your range of influence: you can't affect this player, nor they you */
  outOfRange?: boolean;
  /** the look the player chose (servers with accounts): their Career avatar and title, a supporter's ring and heart */
  look?: PlayerLook | null;
}

const ROPE_TICK_MS = 250;
/** a life change floats up and fades (see .delta); then it leaves the plate, so it doesn't linger as stale text */
const DELTA_MS = 1600;

/** "0:42": how long a player has been gone. */
function elapsed(since: number, now: number): string {
  const seconds = Math.max(0, Math.floor((now - since) / 1000));
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
}

/** Since when the server said this player dropped off (null while connected), and the clock to count it with. */
function useDisconnected(name: string | undefined): { since: number; now: number } | null {
  const since = usePresence((state) => (name ? state.offline[name] : undefined));
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (since === undefined) return;
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [since]);
  return since === undefined ? null : { since, now: Math.max(now, since) };
}

/** The player's priority time left, counted down locally between the server's reports. */
function useRopeSeconds(player: PlayerView): number {
  const reported = player.priorityTimeLeftSecs ?? 0;
  const running = !!player.timerActive && !!player.hasPriority && reported > 0;
  // a new report (a new value, or the same value saved again) restarts the local count from it
  const report = `${reported}|${player.priorityTimeSavedTimeMs ?? ''}`;
  const [clock, setClock] = useState<{ report: string; at: number; now: number } | null>(null);
  useEffect(() => {
    if (!running) return;
    const at = Date.now();
    const timer = setInterval(() => setClock({ report, at, now: Date.now() }), ROPE_TICK_MS);
    return () => clearInterval(timer);
  }, [report, running]);
  if (!running || !clock || clock.report !== report) return reported;
  return ropeSecondsLeft(reported, clock.at, clock.now);
}

/** A player's seat: who they are, their life, counters, floating mana and clock. */
export function PlayerPlate({ player, isMe, targetable, selected, deciding, sleeve, onClick, toPay, payable, onPay, commanderDamage = [], outOfRange = false, look }: PlayerPlateProps) {
  const t = useT();
  const life = player.life ?? 0;
  // a player out of the game (lost, conceded or left: leaving loses) has no life worth showing, however low it went
  const out = !!player.hasLeft;
  const previous = useRef(life);
  const [delta, setDelta] = useState<{ value: number; key: number } | null>(null);
  const disconnected = useDisconnected(out ? undefined : player.name);

  useEffect(() => {
    const change = life - previous.current;
    previous.current = life;
    if (change !== 0) setDelta({ value: change, key: Date.now() });
  }, [life]);
  useEffect(() => {
    if (!delta) return;
    const timer = setTimeout(() => setDelta(null), DELTA_MS);
    return () => clearTimeout(timer);
  }, [delta]);

  const counters = (player.counters ?? []).filter((counter) => (counter.count ?? 0) > 0);
  const pool = POOL_MANA.filter(({ key }) => (player.manaPool?.[key] ?? 0) > 0);
  const hasExtras = !!toPay || pool.length > 0 || commanderDamage.length > 0 || counters.length > 0 || !!player.monarch || !!player.initiative;
  const timeLeft = useRopeSeconds(player);
  const ticking = ropeVisible(player.timerActive, player.hasPriority, timeLeft);
  // the player's sigil from their profile, or their initial
  const initial = sigilOf(player.userData?.avatarId) ?? (player.name ?? '?').slice(0, 1).toUpperCase();
  // a Career avatar replaces the initial, with its drawn crest as the fallback
  const avatar = useCatalogItem('avatar', look?.avatar);
  const crest = avatar?.path ? { light: avatar.light ?? '#7a7466', dark: avatar.dark ?? '#22201b', path: avatar.path } : undefined;
  const drawnAvatar = crest ? (
    <svg className={styles.crest} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={crest.path} />
    </svg>
  ) : <span aria-hidden="true">{initial}</span>;
  // the supporter ring, when worn, is the plate's frame; otherwise an earned one (B296) is drawn round the disc
  const supporterRing = look?.frame === 'supporter-ring';

  return (
    <div
      className={[
        styles.plate,
        isMe ? styles.mine : styles.theirs,
        targetable ? styles.targetable : '',
        selected ? styles.selected : '',
        player.hasLeft ? styles.left : '',
        outOfRange ? styles.outOfRange : '',
        player.isActive ? styles.active : '',
        deciding ? styles.deciding : '',
      ].join(' ')}
      style={sleeve ? ({ '--sleeve': sleeve } as CSSProperties) : undefined}
      role={targetable ? 'button' : 'group'}
      tabIndex={targetable ? 0 : undefined}
      onMouseDown={preventMatchObjectMouseFocus}
      aria-label={out
        ? t('match.plate.aria.lost', { name: player.name ?? '' })
        : `${player.name}, ${life} life${disconnected ? `, ${t('match.plate.aria.disconnected')}` : ''}${outOfRange ? ', out of range' : ''}${targetable ? ', can be targeted' : ''}`}
      onClick={() => targetable && onClick()}
      onKeyDown={(event) => {
        if (targetable && isMatchObjectKey(event.nativeEvent)) {
          event.preventDefault();
          onClick();
        }
      }}
      data-object-id={player.playerId}
    >
      <PlateFrame frame={supporterRing ? null : look?.frame}>
        <div
          className={[styles.avatar, supporterRing ? styles.ring : ''].join(' ')}
          style={crest ? ({ '--crest-light': crest.light, '--crest-dark': crest.dark } as CSSProperties) : undefined}
          data-crest={crest ? '' : undefined}
        >
          {avatar?.id ? <Art kind="avatar" id={avatar.id} size={72} className={styles.avatarArt} fallback={drawnAvatar} /> : drawnAvatar}
          {ticking && (
            <svg className={styles.rope} viewBox="0 0 100 100" role="img" aria-label={`${Math.ceil(timeLeft)} seconds left`}>
              <circle cx="50" cy="50" r="47" pathLength={ROPE_SECONDS} strokeDasharray={`${timeLeft} ${ROPE_SECONDS}`} />
            </svg>
          )}
        </div>
      </PlateFrame>
      <div className={styles.info}>
        <div className={styles.name}>
          {player.name}
          <LevelBadge level={look?.level} />
          {look?.flair === 'heart' && <Heart className={styles.flair} size={14} fill="currentColor" role="img" aria-label={t('shell.supporter')} />}
          {out && <span className={styles.range}>{t('match.plate.lost')}</span>}
          {disconnected && (
            <span className={[styles.range, styles.offline].join(' ')} title={t('match.plate.disconnectedTitle')}>
              {t('match.plate.disconnected', { time: elapsed(disconnected.since, disconnected.now) })}
            </span>
          )}
          {outOfRange && !out && <span className={styles.range} title="Outside your range of influence">Out of range</span>}
        </div>
        {(look?.title || !!look?.badges?.length) && (
          // the title in its rarity's colour (B299), then the badges the player shows off
          <div className={styles.titleRow}>
            {look?.title && <span className={styles.title} style={{ color: titleColor(look.title) }}>{titleName(look.title)}</span>}
            <ShowcaseBadges badges={look?.badges} />
          </div>
        )}
        <div className={styles.lifeRow}>
          {out ? (
            <span className={[styles.life, styles.outLife].join(' ')}>{t('match.plate.out')}</span>
          ) : (
            <>
              <span key={delta?.key} className={[styles.life, delta ? (delta.value < 0 ? styles.hurt : styles.healed) : ''].join(' ')}>{life}</span>
              <span className={styles.unit} aria-hidden="true">life</span>
            </>
          )}
          {delta && !out && (
            <span key={`d${delta.key}`} className={[styles.delta, delta.value < 0 ? styles.deltaDown : styles.deltaUp].join(' ')}>
              {delta.value > 0 ? `+${delta.value}` : delta.value}
            </span>
          )}
        </div>
      </div>
      {hasExtras && (
        // what's on the seat besides life, in short columns beside it: a crowded seat grows sideways, not off the stage
        <div className={styles.extras}>
          {toPay && (
            <div className={styles.toPay}>
              <span className={styles.toPayLabel}>To pay</span>
              <ManaCost cost={toPay} size="lg" />
            </div>
          )}
          {pool.length > 0 && (
            <div className={styles.pool} aria-label="Mana pool">
              {pool.map(({ key, symbol, manaType }) => {
                const id = poolId(manaType);
                const content = (
                  <>
                    <i className={`ms ms-cost ms-${symbol}`} aria-hidden="true" />
                    <b>{player.manaPool?.[key]}</b>
                  </>
                );
                // floating mana pays the cost being paid: click it like a land
                return payable?.has(id) && onPay ? (
                  <button
                    key={key}
                    type="button"
                    className={[styles.poolItem, styles.poolPay].join(' ')}
                    onClick={(event) => {
                      event.stopPropagation();
                      onPay(id);
                    }}
                    aria-label={`Pay with ${key} mana from your pool, ${player.manaPool?.[key]} floating`}
                  >
                    {content}
                  </button>
                ) : (
                  <span key={key} className={styles.poolItem}>{content}</span>
                );
              })}
            </div>
          )}
          {commanderDamage.length > 0 && (
            // each hit is its own chip in the columns; the group only names them
            <div className={styles.commanderDamage} role="group" aria-label="Commander damage taken">
              {commanderDamage.map((hit) => (
                <span
                  key={hit.name}
                  className={[styles.counter, styles.hit, hit.amount >= COMMANDER_DAMAGE_LETHAL - 6 ? styles.hitDanger : ''].join(' ')}
                  title={`${hit.amount} combat damage from ${hit.name}; ${COMMANDER_DAMAGE_LETHAL} loses the game`}
                >
                  <Swords size={15} aria-hidden="true" /> <span className={styles.hitName}>{hit.name.split(',')[0]}</span> <b>{hit.amount}</b>
                </span>
              ))}
            </div>
          )}
          {counters.map((counter) => (
            <span key={counter.name} className={styles.counter} title={`${counter.count} ${counter.name}`}>
              {counter.count} {counter.name}
            </span>
          ))}
          {player.monarch && <span className={styles.counter}>Monarch</span>}
          {player.initiative && <span className={styles.counter}>Initiative</span>}
        </div>
      )}
    </div>
  );
}
