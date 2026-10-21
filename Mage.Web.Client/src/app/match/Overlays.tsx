import { useEffect, useMemo, useState, type ReactNode } from 'react';
import type { Command, Interaction } from '../../core/game/interaction';
import type { Prompt } from '../../core/game/prompt';
import { abilityChooserSource, stripMarkup } from '../../core/game/prompt';
import { isStackAbility } from '../../core/game/cards';
import { matchProgress } from '../../core/game/matchProgress';
import { gameResult, matchResult } from '../../core/game/matchResult';
import type { CardView, GameEndView } from '../../protocol/generated/views';
import { useT, type MessageKey } from '../i18n';
import { Button } from '../ui/Button';
import { AbilityCard } from '../ui/AbilityCard';
import { CardFace } from '../ui/CardFace';
import { PromptText } from '../ui/PromptText';
import { cleanText } from '../ui/text';
import './matchMessages';
import { useMatchUi } from './matchUi';
import styles from './Overlays.module.css';

/** Opening hand, shown large: keep it or mulligan. */
export function MulliganOverlay({ hand, interaction, sleeve, onCommand }: {
  hand: CardView[];
  interaction: Interaction;
  sleeve: string;
  onCommand(command: Command): void;
}) {
  const prompt = interaction.prompt;
  if (!prompt || prompt.kind !== 'ask') return null;
  return (
    <div className={styles.scrim} role="dialog" aria-modal="true" aria-labelledby="mulligan-title">
      <div className={styles.center}>
        <h2 id="mulligan-title" className={styles.bigTitle}>Opening hand</h2>
        <p className={styles.subtitle}>{prompt.text}</p>
        <div className={styles.handRow}>
          {hand.map((card) => (
            <div key={card.id} className={styles.handCard} data-card-id={card.id}>
              <CardFace card={card} sleeve={sleeve} size="normal" />
            </div>
          ))}
        </div>
        <div className={styles.buttons}>
          <Button variant="print" size="lg" onClick={() => onCommand({ type: 'boolean', value: true })}>{prompt.yesLabel}</Button>
          <Button variant="decision" size="xl" onClick={() => onCommand({ type: 'boolean', value: false })} autoFocus>{prompt.noLabel}</Button>
        </div>
      </div>
    </div>
  );
}

/**
 * Choose among cards: off-board cards (library search, revealed cards) or the hand during a London mulligan.
 * Each click toggles a card; the server answers with the updated choice. Only the cards that can be chosen are shown
 * (a library search shows the lands it allows, not the whole library); Show all brings back the rest, greyed.
 */
export function CardPicker({ title, cards, interaction, sleeve, footer, onCommand }: {
  title: string;
  cards: CardView[];
  interaction: Interaction;
  sleeve: string;
  /** more ways to answer, beside the main button */
  footer?: ReactNode;
  onCommand(command: Command): void;
}) {
  const [hidden, setHidden] = useState(false);
  const [showAll, setShowAll] = useState(false);
  const main = interaction.mainButton;
  const choosable = cards.filter((card) => interaction.clickable.has(card.id!) || interaction.selected.has(card.id!));
  const others = cards.length - choosable.length;
  const shown = showAll ? cards : choosable;
  if (hidden) {
    return (
      <div className={styles.peek}>
        <Button variant="decision" onClick={() => setHidden(false)}>Back to choice</Button>
      </div>
    );
  }
  return (
    <div className={styles.scrim} role="dialog" aria-modal="true" aria-label={title}>
      <div className={styles.panel}>
        <header className={styles.panelHead}>
          <h2 className={styles.panelTitle}>{cards.length > 0 && cards.every(isStackAbility) ? 'Choose an ability' : cleanText(title)}</h2>
          {others > 0 && (
            <Button variant="quiet" size="sm" aria-pressed={showAll} onClick={() => setShowAll(!showAll)}>
              {showAll ? 'Only what can be chosen' : `Show all (${cards.length})`}
            </Button>
          )}
          <Button variant="quiet" size="sm" onClick={() => setHidden(true)}>See the board</Button>
        </header>
        <p className={styles.panelText}><PromptText text={interaction.headline} /></p>
        {shown.length === 0 && <p className={styles.panelText}>None of these can be chosen.</p>}
        <div className={styles.grid}>
          {shown.map((card) => {
            const id = card.id!;
            const clickable = interaction.clickable.has(id);
            const selected = interaction.selected.has(id);
            const ability = isStackAbility(card) ? card : null;
            const name = ability ? `${ability.sourceCard?.name ?? 'Ability'}: ${(ability.rules ?? []).map(stripMarkup).join(' ')}` : card.name;
            return (
              <button
                key={id}
                type="button"
                className={[styles.pick, clickable ? styles.pickable : styles.unpickable, selected ? styles.picked : ''].join(' ')}
                disabled={!clickable}
                aria-pressed={selected}
                data-card-id={id}
                aria-label={`${name}${selected ? ', chosen' : ''}`}
                onClick={() => onCommand({ type: 'uuid', id })}
              >
                {ability ? <AbilityCard ability={ability} sleeve={sleeve} /> : <CardFace card={card} sleeve={sleeve} size="normal" />}
              </button>
            );
          })}
        </div>
        {(main || footer || interaction.secondaryButtons.length > 0) && (
          <footer className={styles.panelFoot}>
            {footer}
            {interaction.secondaryButtons.map((button) => (
              <Button key={button.label} variant="quiet" size="lg" onClick={() => onCommand(button.command)}>{button.label}</Button>
            ))}
            {main && <Button variant="decision" size="lg" onClick={() => onCommand(main.command)}>{main.label}</Button>}
          </footer>
        )}
      </div>
    </div>
  );
}

/** Choices that don't live on the board: modes, named choices, piles, numbers. */
export function ChoicePanel({ prompt, onCommand, xLimit = null, startAtLimit = false }: {
  prompt: Prompt;
  onCommand(command: Command): void;
  /** announcing X: the most the player could pay (see xLimit), when the server sets no real limit */
  xLimit?: number | null;
  /** Start X at the most you can pay: the chooser starts at xLimit instead of the minimum */
  startAtLimit?: boolean;
}) {
  const t = useT();
  switch (prompt.kind) {
    case 'chooseAbility': {
      // the server asks even with one ability, to confirm one that sacrifices its source: say so plainly
      const source = prompt.choices.length === 1 ? abilityChooserSource(prompt.text) : null;
      return (
        <PanelShell title={source ? t('match.ability.one', { name: source }) : prompt.text || 'Choose one'}>
          <div className={styles.options}>
            {prompt.choices.map((choice, index) => (
              <button key={choice.id} type="button" className={styles.option} onClick={() => onCommand({ type: 'uuid', id: choice.id })}>
                <span className={styles.optionKey}>{index + 1}</span>
                {/* the server numbers its choices; the key already shows the number */}
                <span><PromptText text={stripMarkup(choice.text).replace(/^\d+\.\s*/, '')} /></span>
              </button>
            ))}
          </div>
          <NumberKeys count={prompt.choices.length} onPick={(index) => onCommand({ type: 'uuid', id: prompt.choices[index].id })} />
          {/* backing out: nothing is cast or activated */}
          <EscapeKey onEscape={() => onCommand({ type: 'uuid', id: null })} />
          <footer className={styles.panelFoot}>
            <Button variant="quiet" onClick={() => onCommand({ type: 'uuid', id: null })}>{t('match.ability.cancel')}</Button>
          </footer>
        </PanelShell>
      );
    }
    case 'chooseChoice':
      return <ChoiceList prompt={prompt} onCommand={onCommand} />;
    case 'choosePile':
      return (
        <PanelShell title={prompt.text || 'Choose a pile'} wide>
          <div className={styles.piles}>
            {[prompt.pile1, prompt.pile2].map((pile, index) => (
              <button key={index} type="button" className={styles.pileChoice} onClick={() => onCommand({ type: 'boolean', value: index === 0 })}>
                <span className={styles.pileLabel}>Pile {index + 1} · {pile.length} {pile.length === 1 ? 'card' : 'cards'}</span>
                <div className={styles.pileCards}>
                  {pile.map((card) => <div key={card.id} className={styles.pileCard}><CardFace card={card} size="small" /></div>)}
                </div>
              </button>
            ))}
          </div>
        </PanelShell>
      );
    case 'amount':
      return <AmountChooser prompt={prompt} onCommand={onCommand} limit={xLimit} startAtLimit={startAtLimit} />;
    case 'multiAmount':
      return <MultiAmountChooser prompt={prompt} onCommand={onCommand} />;
    default:
      return null;
  }
}

function PanelShell({ title, children, wide }: { title: string; children: React.ReactNode; wide?: boolean }) {
  return (
    <div className={styles.scrim} role="dialog" aria-modal="true" aria-label={cleanText(title)}>
      <div className={[styles.panel, wide ? styles.panelWide : styles.panelNarrow].join(' ')}>
        <h2 className={styles.panelTitle}><PromptText text={title} /></h2>
        {children}
      </div>
    </div>
  );
}

function EscapeKey({ onEscape }: { onEscape(): void }) {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (useMatchUi.getState().detail) return;
      if (event.key === 'Escape') {
        event.preventDefault();
        onEscape();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onEscape]);
  return null;
}

function NumberKeys({ count, onPick }: { count: number; onPick(index: number): void }) {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (useMatchUi.getState().detail) return;
      if (event.target instanceof HTMLInputElement) return;
      const value = Number.parseInt(event.key, 10);
      if (value >= 1 && value <= Math.min(9, count)) {
        event.preventDefault();
        onPick(value - 1);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [count, onPick]);
  return null;
}

function ChoiceList({ prompt, onCommand }: { prompt: Extract<Prompt, { kind: 'chooseChoice' }>; onCommand(command: Command): void }) {
  const choice = prompt.choice;
  const keyed = Object.entries(choice.keyChoices ?? {});
  const entries = keyed.length > 0
    ? keyed.map(([key, label]) => ({ value: key, label }))
    : (choice.choices ?? []).map((label) => ({ value: label, label }));
  const sorted = choice.sortData
    ? [...entries].sort((a, b) => (choice.sortData![a.value] ?? 0) - (choice.sortData![b.value] ?? 0))
    : entries;
  const [query, setQuery] = useState('');
  const filtered = useMemo(
    () => sorted.filter((entry) => stripMarkup(entry.label).toLowerCase().includes(query.trim().toLowerCase())),
    [sorted, query],
  );
  const title = stripMarkup(choice.message) || prompt.text || 'Choose one';
  return (
    <PanelShell title={title}>
      {choice.subMessage && <p className={styles.panelText}>{stripMarkup(choice.subMessage)}</p>}
      {entries.length > 8 && (
        <input className={styles.search} placeholder="Search" value={query} onChange={(event) => setQuery(event.target.value)} autoFocus aria-label="Search choices" />
      )}
      <div className={[styles.options, entries.length > 8 ? styles.optionsScroll : ''].join(' ')}>
        {filtered.map((entry, index) => (
          <button key={entry.value} type="button" className={styles.option} onClick={() => onCommand({ type: 'string', value: entry.value })}>
            {entries.length <= 9 && <span className={styles.optionKey}>{index + 1}</span>}
            <span><PromptText text={stripMarkup(entry.label)} /></span>
          </button>
        ))}
      </div>
      {entries.length <= 9 && <NumberKeys count={filtered.length} onPick={(index) => onCommand({ type: 'string', value: filtered[index].value })} />}
      {!choice.required && (
        <footer className={styles.panelFoot}>
          <Button variant="quiet" onClick={() => onCommand({ type: 'string', value: '' })}>Skip</Button>
        </footer>
      )}
    </PanelShell>
  );
}

function AmountChooser({ prompt, onCommand, limit, startAtLimit }: {
  prompt: Extract<Prompt, { kind: 'amount' }>;
  onCommand(command: Command): void;
  limit: number | null;
  startAtLimit: boolean;
}) {
  const t = useT();
  const max = Math.min(prompt.max, 999);
  // X: the range shown stops at what the player could pay; typing or stepping past it stays possible (mana the
  // client can't count, cost reductions)
  const shown = limit === null ? max : Math.max(prompt.min, Math.min(max, limit));
  const clamp = (next: number) => Math.max(prompt.min, Math.min(max, next));
  const [value, setValue] = useState(() => (startAtLimit && limit !== null ? clamp(limit) : prompt.min));
  return (
    <PanelShell title={prompt.text || 'Choose a number'}>
      <div className={styles.amount}>
        <Button variant="print" size="lg" onClick={() => setValue((current) => clamp(current - 1))} aria-label="Less" disabled={value <= prompt.min}>−</Button>
        <input
          className={styles.amountValue}
          type="number"
          value={value}
          min={prompt.min}
          max={max}
          onChange={(event) => setValue(clamp(Number(event.target.value)))}
          onKeyDown={(event) => event.key === 'Enter' && onCommand({ type: 'integer', value })}
          aria-label="Amount"
          autoFocus
        />
        <Button variant="print" size="lg" onClick={() => setValue((current) => clamp(current + 1))} aria-label="More" disabled={value >= max}>+</Button>
      </div>
      {shown - prompt.min <= 20 && shown > prompt.min && (
        <input className={styles.slider} type="range" min={prompt.min} max={shown} value={Math.min(value, shown)} onChange={(event) => setValue(Number(event.target.value))} aria-label="Amount" />
      )}
      {limit !== null && <p className={styles.panelText}>{t('match.amount.mana', { count: limit })}</p>}
      <footer className={styles.panelFoot}>
        <span className={styles.range}>{prompt.min} to {shown}</span>
        <Button variant="decision" size="lg" onClick={() => onCommand({ type: 'integer', value })}>Choose {value}</Button>
      </footer>
    </PanelShell>
  );
}

function MultiAmountChooser({ prompt, onCommand }: { prompt: Extract<Prompt, { kind: 'multiAmount' }>; onCommand(command: Command): void }) {
  const [values, setValues] = useState(prompt.items.map((item) => item.defaultValue ?? item.min ?? 0));
  const total = values.reduce((sum, value) => sum + value, 0);
  const valid = total >= prompt.min && total <= prompt.max
    && prompt.items.every((item, index) => values[index] >= (item.min ?? 0) && values[index] <= (item.max ?? Infinity));
  return (
    <PanelShell title={prompt.title ?? (prompt.text || 'Divide')}>
      {prompt.header && <p className={styles.panelText}>{stripMarkup(prompt.header)}</p>}
      <div className={styles.split}>
        {prompt.items.map((item, index) => (
          <label key={index} className={styles.splitRow}>
            <span>{stripMarkup(item.message)}</span>
            <input
              type="number"
              className={styles.splitValue}
              min={item.min}
              max={item.max}
              value={values[index]}
              onChange={(event) => setValues((current) => current.map((value, i) => (i === index ? Number(event.target.value) : value)))}
            />
          </label>
        ))}
      </div>
      <footer className={styles.panelFoot}>
        <span className={styles.range}>Total {total} · needs {prompt.min === prompt.max ? prompt.min : `${prompt.min} to ${prompt.max}`}</span>
        {prompt.canCancel && <Button variant="quiet" onClick={() => onCommand({ type: 'string', value: '' })}>Cancel</Button>}
        <Button variant="decision" size="lg" disabled={!valid} onClick={() => onCommand({ type: 'string', value: values.join(' ') })}>Done</Button>
      </footer>
    </PanelShell>
  );
}

/** You won the roll: choose who takes the first turn. */
export function StartingPlayerOverlay({ me, opponents, onChoose }: {
  me: { id: string; name: string };
  opponents: { id: string; name: string }[];
  onChoose(playerId: string): void;
}) {
  return (
    <div className={styles.scrim} role="dialog" aria-modal="true" aria-labelledby="start-title">
      <div className={styles.center}>
        <h2 id="start-title" className={styles.bigTitle}>You choose who starts</h2>
        <p className={styles.subtitle}>The player who goes first skips their first draw.</p>
        <div className={styles.buttons}>
          {opponents.map((opponent) => (
            <Button key={opponent.id} variant="print" size="lg" onClick={() => onChoose(opponent.id)}>
              {opponents.length === 1 ? 'Draw first' : `${opponent.name} starts`}
            </Button>
          ))}
          <Button variant="decision" size="xl" onClick={() => onChoose(me.id)} autoFocus>Play first</Button>
        </div>
      </div>
    </div>
  );
}

const END_TITLE: Record<string, MessageKey> = { won: 'match.end.victory', lost: 'match.end.defeat', draw: 'match.end.draw' };
const END_GAME: Record<string, MessageKey> = { won: 'match.end.wonGame', lost: 'match.end.lostGame', draw: 'match.end.drawGame' };
const END_MATCH: Record<string, MessageKey> = { won: 'match.end.wonMatch', lost: 'match.end.lostMatch', draw: 'match.end.drawMatch' };

/** The end of a game: who won, the match score, and where to go next. */
export function GameOverOverlay({ message, endInfo, myName, onLeave, leaveLabel = 'Back to Play', onPlayAgain, onReview }: {
  message: string;
  endInfo: GameEndView | null;
  /** the player's name, to tell from the server's message who won when there is no end-of-game info */
  myName: string | null;
  onLeave(): void;
  leaveLabel?: string;
  onPlayAgain?(): void;
  /** look back over the game: its turning point and mistakes */
  onReview?(): void;
}) {
  const t = useT();
  // a match of several games ends on the match's result; a single game on the game's
  const matchEnd = matchResult(endInfo);
  const progress = matchEnd ? matchProgress(endInfo) : null;
  const result = matchEnd ?? gameResult(message, endInfo, myName);
  const line = matchEnd && progress
    ? t(END_MATCH[matchEnd], { wins: progress.wins, losses: progress.losses })
    : result ? t(END_GAME[result]) : stripMarkup(endInfo?.gameInfo) || message;
  // why a match ended early ("Bob has quit the match.")
  const extra = stripMarkup(endInfo?.additionalInfo);
  return (
    <div className={[styles.scrim, styles.endScrim].join(' ')} role="dialog" aria-modal="true" aria-labelledby="game-over-title">
      <div className={styles.center}>
        <h2 id="game-over-title" className={[styles.endTitle, result === 'won' ? styles.victory : result === 'lost' ? styles.defeat : ''].join(' ')}>
          {t(result ? END_TITLE[result] : 'match.end.over')}
        </h2>
        <p className={styles.subtitle}>{line}</p>
        {extra && <p className={styles.score}>{extra}</p>}
        <div className={styles.buttons}>
          <Button variant="print" size="lg" onClick={onLeave}>{leaveLabel}</Button>
          {onReview && <Button variant="quiet" size="lg" onClick={onReview}>Review the game</Button>}
          {onPlayAgain && <Button variant="decision" size="xl" onClick={onPlayAgain}>Play again</Button>}
        </div>
      </div>
    </div>
  );
}

/** A zone's cards laid out to read (graveyard, exile, revealed). */
export function ZoneViewer({ title, cards, onClose }: { title: string; cards: CardView[]; onClose(): void }) {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => event.key === 'Escape' && !useMatchUi.getState().detail && onClose();
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);
  return (
    <div className={styles.scrim} role="dialog" aria-modal="true" aria-label={title} onClick={onClose}>
      <div className={styles.panel} onClick={(event) => event.stopPropagation()}>
        <header className={styles.panelHead}>
          <h2 className={styles.panelTitle}>{title}</h2>
          <Button variant="quiet" size="sm" onClick={onClose}>Close</Button>
        </header>
        {cards.length === 0 ? <p className={styles.panelText}>Empty.</p> : (
          <div className={styles.grid}>
            {cards.map((card) => <div key={card.id} className={styles.pick} data-card-id={card.id}><CardFace card={card} size="normal" /></div>)}
          </div>
        )}
      </div>
    </div>
  );
}
