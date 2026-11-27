import { Shield, Skull } from 'lucide-react';
import { useMemo } from 'react';
import { combatPreview, type CombatPreview as Preview, type Fighter, type PairPreview } from '../../core/game/combatPreview';
import type { Interaction } from '../../core/game/interaction';
import type { GameView } from '../../protocol/generated/views';
import { useT, type Translate } from '../i18n';
import { useObjectBoxes } from './boardAnchors';
import { useMatchUi } from './matchUi';
import './matchMessages';
import styles from './CombatPreview.module.css';

/**
 * While you declare blockers, an estimate of how combat ends for the blocks as they stand (core/game/combatPreview):
 * each blocked combatant is marked dies or survives, and a small panel above the decision corner says what each
 * fight does, the damage that gets through and your life after combat. While you declare attackers, the panel says
 * what the attack deals if nothing blocks; when attacking several defenders, it also groups attackers per defender
 * with matching arrow colors and hover-to-isolate. Hidden at any other time.
 */

const MAX_ROWS = 6;

interface DefenderGroup {
  id: string;
  name: string;
  color: string | null;
  damage: number;
  attackers: string[];
}

/** Group attacking creatures by defender, collapsing identical name + P/T into "4× Goblin 1/1". */
function summarizeAttackers(t: Translate, preview: Preview, colors?: ReadonlyMap<string, string>): DefenderGroup[] {
  const byId = new Map<string, { name: string; damage: number; counts: Map<string, number> }>();
  for (const pair of preview.pairs) {
    let entry = byId.get(pair.defenderId);
    if (!entry) {
      entry = { name: pair.defenderName, damage: 0, counts: new Map() };
      byId.set(pair.defenderId, entry);
    }
    entry.damage += pair.result.toDefender;
    const key = withPt(t, pair.attacker);
    entry.counts.set(key, (entry.counts.get(key) ?? 0) + 1);
  }
  return [...byId.entries()].map(([id, entry]) => ({
    id,
    name: entry.name,
    color: colors?.get(id) ?? null,
    damage: entry.damage,
    attackers: [...entry.counts.entries()].map(([text, count]) => (count > 1 ? `${count}× ${text}` : text)),
  }));
}

function label(t: Translate, fighter: Fighter): string {
  return fighter.name || t('match.combat.faceDownName');
}

function names(t: Translate, fighters: readonly Fighter[]): string {
  return fighters.map((fighter) => label(t, fighter)).join(' + ');
}

function withPt(t: Translate, fighter: Fighter): string {
  return `${label(t, fighter)} ${fighter.power}/${fighter.toughness - fighter.marked}`;
}

function pairLine(t: Translate, pair: PairPreview, myId: string | undefined): string {
  const { attacker, blockers, result } = pair;
  const target = pair.defenderId === myId ? t('match.combat.you') : pair.defenderName;
  if (blockers.length === 0) return t('match.combat.unblocked', { attacker: withPt(t, attacker), damage: result.toDefender, target });
  const dying = [...(result.attackerDies ? [attacker] : []), ...blockers.filter((blocker) => result.deadBlockers.includes(blocker.id))];
  let outcome = dying.length === 0 ? t('match.combat.noneDie')
    : dying.length === blockers.length + 1 ? t(blockers.length === 1 ? 'match.combat.trade' : 'match.combat.allDie')
      : t('match.combat.dies', { count: dying.length, names: names(t, dying) });
  if (result.toDefender > 0) outcome = t('match.combat.trample', { outcome, damage: result.toDefender, target });
  return t('match.combat.fight', { attacker: withPt(t, attacker), blockers: blockers.map((blocker) => withPt(t, blocker)).join(' + '), outcome });
}

function lifeLines(t: Translate, preview: Preview, attacking: boolean): string[] {
  return preview.defenders.map((defender) => {
    if (!defender.isPlayer || defender.lifeAfter === null) return t('match.combat.damageTo', { damage: defender.damage, name: defender.name });
    let line = attacking && !defender.isMe
      ? t('match.combat.ifUnblocked', { damage: defender.damage, name: defender.name, life: defender.lifeAfter })
      : defender.isMe ? t('match.combat.lifeYou', { life: defender.lifeAfter }) : t('match.combat.lifeThem', { name: defender.name, life: defender.lifeAfter });
    if (defender.poison > 0) line = t('match.combat.poison', { line, poison: defender.poison });
    if (defender.lethal) line = t('match.combat.lethal', { line });
    return line;
  });
}

export function CombatPreview({ view, mode, defenderColors }: {
  view: GameView | null | undefined;
  mode: Interaction['mode'];
  defenderColors?: ReadonlyMap<string, string>;
}) {
  const t = useT();
  const setHoveredDefender = useMatchUi((state) => state.setHoveredDefender);
  const hoveredDefender = useMatchUi((state) => state.hoveredDefender);
  const active = mode === 'declareBlockers' || mode === 'declareAttackers';
  const preview = useMemo(() => (active ? combatPreview(view) : null), [active, view]);
  const blocking = mode === 'declareBlockers';
  // blocked attackers and their blockers: an unblocked attacker always survives combat, so it carries no mark
  const fates = useMemo(() => new Map(blocking && preview
    ? preview.pairs.filter((pair) => pair.blockers.length > 0)
      .flatMap((pair) => [pair.attacker, ...pair.blockers].map((fighter) => [fighter.id, preview.fate.get(fighter.id)!] as const))
    : []), [blocking, preview]);
  const ids = useMemo(() => [...fates.keys()], [fates]);
  const attackerIds = useMemo(() => new Set(preview?.pairs.map((pair) => pair.attacker.id)), [preview]);
  const boxes = useObjectBoxes(ids);
  if (!preview) return null;
  const myId = view?.myPlayerId;
  const rows = blocking ? preview.pairs.map((pair) => pairLine(t, pair, myId)) : [];
  const multiOpponent = (view?.players?.length ?? 0) > 2;
  const attackBreakdown = !blocking && (preview.defenders.length > 1 || (multiOpponent && preview.pairs.length > 0))
    ? summarizeAttackers(t, preview, preview.defenders.length > 1 ? defenderColors : undefined)
    : [];
  const gains = [...preview.gains].map(([id, life]) => (id === myId
    ? t('match.combat.gainYou', { life })
    : t('match.combat.gainThem', { life, name: view?.players?.find((player) => player.playerId === id)?.name ?? '' })));
  const notes = [
    ...preview.pairs.filter((pair) => pair.menaceAlone).map((pair) => t('match.combat.menace', { name: label(t, pair.attacker) })),
    ...preview.sharedBlockers.map((shared) => t('match.combat.sharedBlocker', { name: shared.name || t('match.combat.faceDownName'), count: shared.count })),
    ...(preview.faceDown ? [t('match.combat.faceDown')] : []),
  ];
  const title = t(blocking ? 'match.combat.title' : 'match.combat.attackTitle');

  return (
    <div className={styles.layer}>
      {[...fates].map(([id, fate]) => {
        const box = boxes.get(id);
        if (!box) return null;
        return (
          <div key={id}>
            {fate === 'dies' && <div className={styles.doomed} style={{ left: box.x, top: box.y, width: box.width, height: box.height }} />}
            <span
              // an attacker's mark on its top edge, clear of its blockers' marks
              className={[styles.mark, fate === 'dies' ? styles.dies : styles.survives, attackerIds.has(id) ? styles.top : ''].join(' ')}
              style={{ left: box.x + box.width / 2, top: attackerIds.has(id) ? box.y + 6 : box.y + box.height - 6 }}
            >
              {fate === 'dies' ? <Skull size={20} aria-hidden="true" /> : <Shield size={18} aria-hidden="true" />}
              {t(fate === 'dies' ? 'match.combat.markDies' : 'match.combat.markSurvives')}
            </span>
          </div>
        );
      })}
      <section className={styles.panel} aria-live="polite" aria-label={title}>
        <h3 className={styles.title}>{title}</h3>
        {attackBreakdown.length > 0 && (
          <ul className={styles.defenders}>
            {attackBreakdown.map((group) => (
              <li
                key={group.id}
                className={[styles.defenderRow, hoveredDefender === group.id ? styles.defenderActive : ''].join(' ')}
                onPointerEnter={() => setHoveredDefender(group.id)}
                onPointerLeave={() => setHoveredDefender(null)}
              >
                <span className={styles.defenderHead}>
                  {group.color && <span className={styles.swatch} style={{ background: group.color }} aria-hidden="true" />}
                  <b>{group.name}</b>
                  <span className={styles.defenderDmg}>{group.damage}</span>
                </span>
                <span className={styles.defenderList}>{group.attackers.join(', ')}</span>
              </li>
            ))}
          </ul>
        )}
        {rows.length > 0 && (
          <ul className={styles.rows}>
            {rows.slice(0, MAX_ROWS).map((row, index) => <li key={index}>{row}</li>)}
            {rows.length > MAX_ROWS && <li>{t('match.combat.more', { count: rows.length - MAX_ROWS })}</li>}
          </ul>
        )}
        {lifeLines(t, preview, !blocking).map((line, index) => <p key={index} className={styles.life}>{line}</p>)}
        {gains.map((line, index) => <p key={index} className={styles.gain}>{line}</p>)}
        {notes.map((note, index) => <p key={index} className={styles.warn}>{note}</p>)}
        <p className={styles.caveat}>{t(preview.assumedSplit ? 'match.combat.caveatSplit' : 'match.combat.caveat')}</p>
      </section>
    </div>
  );
}
