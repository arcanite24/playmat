import { Minus, Plus, WandSparkles } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import type { CareerLimitedRun } from '../../protocol/generated/views';
import { useT, type MessageKey } from '../i18n';
import { readJson, writeJson } from '../stores/persist';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { CardFace } from '../ui/CardFace';
import { DraftReview, PoolCoach } from './CareerLimitedCoach';
import { submitLimitedDeck } from './careerModesData';
import {
  BASIC_LANDS, LIMITED_DECK_MIN, buildDeck, buildFrom, buildProblems, buildSize, changeBasic, changePick, poolEntries, suggestBasics,
  type BasicLand, type LimitedBuild, type PoolEntry,
} from './careerModesModel';
import styles from './CareerModes.module.css';

const SYMBOL: Record<BasicLand, string> = { Plains: 'w', Island: 'u', Swamp: 'b', Mountain: 'r', Forest: 'g' };
const COLORS = ['W', 'U', 'B', 'R', 'G'] as const;
const COLOR_KEYS: Record<(typeof COLORS)[number], MessageKey> = {
  W: 'career.color.W', U: 'career.color.U', B: 'career.color.B', R: 'career.color.R', G: 'career.color.G',
};
const LAND_KEYS: Record<BasicLand, MessageKey> = {
  Plains: 'career.land.Plains', Island: 'career.land.Island', Swamp: 'career.land.Swamp', Mountain: 'career.land.Mountain', Forest: 'career.land.Forest',
};

function draftKey(runId: string): string {
  return `playmat.career.limited.${runId}`;
}

/**
 * Building a sealed or draft deck from the run's pool: click a card to put a copy in, take it out from the deck list,
 * add basic lands (free), and send it once it has 40 cards. The deck in progress stays in this browser until then.
 */
export function LimitedBuilder({ run, onDone, onCancel }: { run: CareerLimitedRun; onDone(): void; onCancel?: () => void }) {
  const t = useT();
  const entries = useMemo(() => poolEntries(run.pool), [run.pool]);
  const [build, setBuild] = useState<LimitedBuild>(() => readJson<LimitedBuild | null>(draftKey(run.id ?? ''), null) ?? buildFrom(run.deck));
  const [colors, setColors] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    writeJson(draftKey(run.id ?? ''), build);
  }, [build, run.id]);

  const size = buildSize(build);
  const problems = buildProblems(build, entries);
  const shown = useMemo(() => (colors.length === 0
    ? entries
    : entries.filter((entry) => {
      const cardColors = (entry.card.colors ?? '').toUpperCase();
      return cardColors === '' || [...cardColors].some((color) => colors.includes(color));
    })), [entries, colors]);
  const inDeck = entries.filter((entry) => (build.picks[entry.key] ?? 0) > 0);

  async function save() {
    setBusy(true);
    try {
      await submitLimitedDeck(buildDeck(build, entries, `${run.setName ?? run.setCode ?? ''} ${run.kind ?? ''}`.trim()));
      notify(t('career.limited.deckSaved'), t('career.deck.summary', { count: size }));
      onDone();
    } catch (reason) {
      notify(t('career.limited.deckFailed'), reason instanceof Error ? reason.message : String(reason), 'error');
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className={styles.builder} aria-labelledby="limited-build">
      <div className={styles.builderPool}>
        <header className={styles.panelHead}>
          <h2 id="limited-build" className={styles.panelTitle}>{t('career.limited.build', { count: run.pool?.length ?? 0 })}</h2>
          <div className={styles.colorFilter} role="group" aria-label={t('career.limited.filter')}>
            {COLORS.map((color) => (
              <button
                key={color}
                type="button"
                className={[styles.colorButton, colors.includes(color) ? styles.colorButtonOn : ''].join(' ')}
                aria-pressed={colors.includes(color)}
                aria-label={t(COLOR_KEYS[color])}
                onClick={() => setColors((current) => (current.includes(color) ? current.filter((item) => item !== color) : [...current, color]))}
              >
                <i className={`ms ms-cost ms-${color.toLowerCase()}`} aria-hidden="true" />
              </button>
            ))}
          </div>
        </header>
        <p className={styles.small}>{t('career.limited.buildHint')}</p>
        {run.kind === 'draft' && <DraftReview picks={run.picks ?? []} />}
        <PoolCoach
          run={run}
          onBuild={(pair) => {
            setBuild(buildFrom(pair.deck));
            notify(t('career.coach.built'), t('career.coach.builtText', { colors: pair.colors ?? '' }));
          }}
        />
        <ul className={styles.poolGrid}>
          {shown.map((entry) => (
            <li key={entry.key}>
              <PoolTile entry={entry} inDeck={build.picks[entry.key] ?? 0} onAdd={() => setBuild((current) => changePick(current, entry, 1))} />
            </li>
          ))}
        </ul>
      </div>

      <aside className={styles.builderDeck} aria-labelledby="limited-deck">
        <h2 id="limited-deck" className={styles.cardLabel}>{t('career.limited.deckCount', { count: size, min: LIMITED_DECK_MIN })}</h2>
        <div className={styles.meter} role="progressbar" aria-valuemin={0} aria-valuemax={LIMITED_DECK_MIN} aria-valuenow={Math.min(size, LIMITED_DECK_MIN)} aria-label={t('career.limited.deckCount', { count: size, min: LIMITED_DECK_MIN })}>
          <span style={{ width: `${Math.min(100, (size / LIMITED_DECK_MIN) * 100)}%` }} />
        </div>

        <h3 className={styles.subhead}>{t('career.limited.lands')}</h3>
        <ul className={styles.basics}>
          {BASIC_LANDS.map((land) => (
            <li key={land}>
              <i className={`ms ms-cost ms-${SYMBOL[land]}`} aria-hidden="true" />
              <span className={styles.basicName}>{t(LAND_KEYS[land])}</span>
              <button type="button" className={styles.stepper} aria-label={t('career.limited.less', { name: t(LAND_KEYS[land]) })} onClick={() => setBuild((current) => changeBasic(current, land, -1))} disabled={build.basics[land] === 0}><Minus size={14} /></button>
              <b className={styles.basicCount}>{build.basics[land]}</b>
              <button type="button" className={styles.stepper} aria-label={t('career.limited.more', { name: t(LAND_KEYS[land]) })} onClick={() => setBuild((current) => changeBasic(current, land, 1))}><Plus size={14} /></button>
            </li>
          ))}
        </ul>
        <Button variant="quiet" size="sm" icon={<WandSparkles size={16} />} disabled={inDeck.length === 0} onClick={() => setBuild((current) => ({ ...current, basics: suggestBasics(current, entries) }))}>
          {t('career.limited.suggestLands')}
        </Button>

        <h3 className={styles.subhead}>{t('career.limited.spells', { count: size - Object.values(build.basics).reduce((sum, count) => sum + count, 0) })}</h3>
        {inDeck.length === 0 && <p className={styles.small}>{t('career.limited.emptyDeck')}</p>}
        <ul className={styles.deckList}>
          {inDeck.map((entry) => (
            <li key={entry.key}>
              <span>{(build.picks[entry.key] ?? 0) > 1 ? `${build.picks[entry.key]} × ${entry.card.name}` : entry.card.name}</span>
              <button type="button" className={styles.stepper} aria-label={t('career.limited.remove', { name: entry.card.name ?? '' })} onClick={() => setBuild((current) => changePick(current, entry, -1))}><Minus size={14} /></button>
            </li>
          ))}
        </ul>

        {problems.map((problem, index) => (
          <p key={index} className={styles.warn}>
            {problem.kind === 'short'
              ? t('career.limited.short', { count: problem.count, min: problem.min })
              : t('career.limited.notInPool', { name: problem.name, count: problem.inDeck, pool: problem.inPool })}
          </p>
        ))}
        <div className={styles.actions}>
          <Button variant="decision" busy={busy} disabled={problems.length > 0} onClick={() => void save()}>{t('career.limited.saveDeck')}</Button>
          {onCancel && <Button variant="quiet" size="sm" onClick={onCancel}>{t('career.keep')}</Button>}
        </div>
      </aside>
    </section>
  );
}

function PoolTile({ entry, inDeck, onAdd }: { entry: PoolEntry; inDeck: number; onAdd(): void }) {
  const t = useT();
  const full = inDeck >= entry.count;
  return (
    <button
      type="button"
      className={[styles.poolTile, full ? styles.poolTileFull : ''].join(' ')}
      onClick={onAdd}
      disabled={full}
      aria-label={t('career.limited.tile', { name: entry.card.name ?? '', count: inDeck, pool: entry.count })}
    >
      <CardFace card={{ name: entry.card.name, setCode: entry.card.setCode, cardNumber: entry.card.cardNumber }} size="normal" />
      <span className={styles.poolCount} aria-hidden="true">{inDeck}/{entry.count}</span>
    </button>
  );
}
