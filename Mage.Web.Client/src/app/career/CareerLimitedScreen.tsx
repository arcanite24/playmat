import { Coins, Flag, Layers, Swords } from 'lucide-react';
import { useMemo, useState } from 'react';
import { Navigate } from 'react-router-dom';
import type { CareerLimitedRun, CareerLimitedState, CareerPoolCard } from '../../protocol/generated/views';
import { api } from '../connection';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { usePlay } from '../stores/play';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { CardFace } from '../ui/CardFace';
import { useCareerShop, useCareerState } from './careerData';
import { abandonLimited, pickLimited, playMode, startLimited, useDraftCubes, useLimited } from './careerModesData';
import { LIMITED_LOSSES, LIMITED_WINS } from './careerModesModel';
import { runLimits, shapeRewards } from './ladderModel';
import { LimitedBuilder } from './CareerLimitedBuild';
import { DraftCoach } from './CareerLimitedCoach';
import { ModeResult } from './ModeParts';
import styles from './CareerModes.module.css';

registerMessages(messages);

type Kind = 'sealed' | 'draft' | 'cube';

const KIND_NAMES: Record<Kind, 'career.limited.sealed' | 'career.limited.draft' | 'career.limited.cube'> = {
  sealed: 'career.limited.sealed',
  draft: 'career.limited.draft',
  cube: 'career.limited.cube',
};

const KIND_TEXTS: Record<Kind, 'career.limited.sealedText' | 'career.limited.draftText' | 'career.limited.cubeText'> = {
  sealed: 'career.limited.sealedText',
  draft: 'career.limited.draftText',
  cube: 'career.limited.cubeText',
};

function message(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

/**
 * Solo sealed, draft and cube: open a set's packs (or draft them, or a server cube, against bots), build 40 cards, and
 * play the run's shape out: three wins or two losses, Arena's seven or three, or a league's five matches.
 */
export function CareerLimitedScreen() {
  const t = useT();
  const state = useCareerState();
  const profile = state.data?.profile;
  const limited = useLimited(!!profile);

  if (state.isPending) return <p className={styles.note}>{t('career.loading')}</p>;
  if (!profile) return <Navigate to="/career" replace />;
  const data = limited.data;
  const run = data?.run;

  return (
    <div className={styles.page}>
      <div className={styles.scroll}>
        <ModeResult kind="limited" />
        <header className={styles.modeHead}>
          <div>
            <h1 className={styles.modeTitle}>{run ? t('career.limited.runTitle', { set: run.setName ?? run.setCode ?? '', kind: t(run.kind === 'draft' ? 'career.limited.draft' : 'career.limited.sealed') }) : t('career.limited.title')}</h1>
            <p className={styles.lead}>
              {run ? data?.shapes?.find((shape) => shape.id === (run.shape || 'classic'))?.text ?? t('career.limited.lead', { wins: LIMITED_WINS, losses: LIMITED_LOSSES })
                : t('career.limited.lead', { wins: LIMITED_WINS, losses: LIMITED_LOSSES })}
            </p>
          </div>
        </header>
        {limited.isPending && <p className={styles.note}>{t('career.loading')}</p>}
        {limited.isError && <p className={styles.error} role="alert">{t('career.failed')}</p>}
        {data && !run && <Start data={data} coins={profile.coins ?? 0} />}
        {run && <Run run={run} data={data!} />}
        {data && <Recent runs={data.recent ?? []} />}
      </div>
    </div>
  );
}

function Start({ data, coins }: { data: CareerLimitedState; coins: number }) {
  const t = useT();
  const shop = useCareerShop(true);
  const sets = useMemo(() => (shop.data ?? []).filter((set) => !set.locked), [shop.data]);
  const [kind, setKind] = useState<Kind>('sealed');
  const [shape, setShape] = useState('classic');
  const [setCode, setSetCode] = useState('');
  const [cube, setCube] = useState('');
  const [busy, setBusy] = useState(false);
  const cubes = useDraftCubes(kind === 'cube');
  const chosenSet = setCode || sets[0]?.setCode || '';
  const chosenCube = cube || cubes.data?.[0] || '';
  const shapes = data.shapes ?? [];
  // a cube run is a draft on the server: it costs and pays as one
  const entryOf = (item: Kind) => (item === 'sealed' ? data.sealedEntry : data.draftEntry) ?? 0;
  const price = entryOf(kind);
  const rewards = shapeRewards(shapes, shape, kind === 'sealed' ? 'sealed' : 'draft', data);
  const short = coins < price;
  const source = kind === 'cube' ? chosenCube : chosenSet;

  async function start() {
    setBusy(true);
    try {
      await startLimited(kind, source, shapes.length > 0 ? shape : undefined);
    } catch (reason) {
      notify(t('career.limited.startFailed'), message(reason), 'error');
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className={styles.panel} aria-labelledby="limited-start">
      <h2 id="limited-start" className={styles.panelTitle}>{t('career.limited.newRun')}</h2>
      <div className={styles.kinds} role="radiogroup" aria-label={t('career.limited.kind')}>
        {(['sealed', 'draft', 'cube'] as const).map((item) => (
          <button
            key={item}
            type="button"
            role="radio"
            aria-checked={kind === item}
            className={[styles.kind, kind === item ? styles.kindOn : ''].join(' ')}
            onClick={() => setKind(item)}
          >
            <b>{t(KIND_NAMES[item])}</b>
            <small>{t(KIND_TEXTS[item])}</small>
            <small>{t('career.limited.entry', { price: entryOf(item) })}</small>
          </button>
        ))}
      </div>
      {shapes.length > 0 && (
        <div className={styles.kinds} role="radiogroup" aria-label={t('career.limited.shape')}>
          {shapes.map((item) => (
            <button
              key={item.id}
              type="button"
              role="radio"
              aria-checked={shape === item.id}
              className={[styles.kind, shape === item.id ? styles.kindOn : ''].join(' ')}
              onClick={() => setShape(item.id ?? 'classic')}
            >
              <b>{item.name}</b>
              <small>{item.text}</small>
            </button>
          ))}
        </div>
      )}
      {kind === 'cube' ? (
        <label className={styles.field}>
          <span>{t('career.limited.cubeName')}</span>
          <select value={chosenCube} onChange={(event) => setCube(event.target.value)} disabled={!cubes.data?.length}>
            {(cubes.data ?? []).map((name) => <option key={name} value={name}>{name}</option>)}
          </select>
        </label>
      ) : (
        <label className={styles.field}>
          <span>{t('career.limited.set')}</span>
          <select value={chosenSet} onChange={(event) => setSetCode(event.target.value)} disabled={sets.length === 0}>
            {sets.map((set) => <option key={set.setCode} value={set.setCode}>{set.name} ({set.setCode})</option>)}
          </select>
        </label>
      )}
      <p className={styles.small}>{t('career.limited.rewardsBy', { rewards: rewards.map((coinsFor, wins) => t('career.limited.rewardAt', { wins, coins: coinsFor })).join(' · ') })}</p>
      <p className={styles.small}>{t(kind === 'cube' ? 'career.limited.cubeLoan' : 'career.limited.keepCards')}</p>
      <div className={styles.actions}>
        <Button variant="decision" size="lg" icon={<Coins size={18} />} busy={busy} disabled={short || !source} aria-describedby={short ? 'limited-short' : undefined} onClick={() => void start()}>
          {t('career.limited.start', { price })}
        </Button>
        {short && <span id="limited-short" className={styles.warn}>{t('career.shop.short')}</span>}
      </div>
    </section>
  );
}

function Run({ run, data }: { run: CareerLimitedRun; data: CareerLimitedState }) {
  const [editing, setEditing] = useState(false);
  if (run.phase === 'draft') return <Draft run={run} />;
  if (run.phase === 'build' || editing) {
    return <LimitedBuilder key={run.id} run={run} onDone={() => setEditing(false)} onCancel={run.phase === 'play' ? () => setEditing(false) : undefined} />;
  }
  return <Games run={run} data={data} onEdit={() => setEditing(true)} />;
}

/** A draft pick: the pack in front of you; the bots pick theirs on the server. */
function Draft({ run }: { run: CareerLimitedRun }) {
  const t = useT();
  const [busy, setBusy] = useState<number | null>(null);
  const pack = run.pack ?? [];
  const pool = run.pool ?? [];

  async function pick(index: number) {
    setBusy(index);
    try {
      await pickLimited(index);
    } catch (reason) {
      notify(t('career.limited.pickFailed'), message(reason), 'error');
    } finally {
      setBusy(null);
    }
  }

  return (
    <section className={styles.panel} aria-labelledby="limited-draft">
      <header className={styles.panelHead}>
        <h2 id="limited-draft" className={styles.panelTitle}>{t('career.limited.pick', { round: run.round ?? 1, pick: run.pick ?? 1 })}</h2>
        <p className={styles.small}>{t('career.limited.pickHint', { count: pool.length })}</p>
      </header>
      <DraftCoach run={run} />
      <ul className={styles.packGrid}>
        {pack.map((card, index) => (
          <li key={`${index}-${card.name}`}>
            <button
              type="button"
              className={styles.poolTile}
              disabled={busy !== null}
              aria-busy={busy === index || undefined}
              aria-label={t('career.limited.take', { name: card.name ?? '' })}
              onClick={() => void pick(index)}
            >
              <CardFace card={{ name: card.name, setCode: card.setCode, cardNumber: card.cardNumber }} size="normal" />
            </button>
          </li>
        ))}
      </ul>
      {pool.length > 0 && <PoolSummary pool={pool} />}
    </section>
  );
}

function PoolSummary({ pool }: { pool: CareerPoolCard[] }) {
  const t = useT();
  return (
    <details className={styles.poolSummary}>
      <summary>{t('career.limited.picked', { count: pool.length })}</summary>
      <ul className={styles.names}>{pool.map((card, index) => <li key={index}>{card.name}</li>)}</ul>
    </details>
  );
}

/** The run's games: the record, what the next win pays, play, change the deck, or give up. */
function Games({ run, data, onEdit }: { run: CareerLimitedRun; data: CareerLimitedState; onEdit(): void }) {
  const t = useT();
  const playing = usePlay((play) => play.phase !== 'idle');
  const [starting, setStarting] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const wins = run.wins ?? 0;
  const losses = run.losses ?? 0;
  const limits = runLimits(run);
  const rewards = shapeRewards(data.shapes, run.shape, run.kind, data);
  const added = run.added ?? [];
  const deckSize = (run.deck ?? []).reduce((sum, card) => sum + (card.amount ?? 0), 0);

  async function play() {
    setStarting(true);
    setError(null);
    try {
      await playMode('limited', () => api.careerLimitedPlay());
    } catch (reason) {
      setError(message(reason));
      setStarting(false);
    }
  }

  async function abandon() {
    try {
      await abandonLimited();
      notify(t('career.limited.abandoned'), '');
    } catch (reason) {
      notify(t('career.gauntlet.abandonFailed'), message(reason), 'error');
    } finally {
      setConfirming(false);
    }
  }

  return (
    <section className={styles.panel} aria-labelledby="limited-games">
      <h2 id="limited-games" className={styles.panelTitle}>{t('career.limited.game', { number: wins + losses + 1 })}</h2>
      {limits.league ? (
        <div className={styles.record} role="img" aria-label={t('career.limited.leagueRecord', { played: wins + losses, games: limits.maxGames, wins, losses })}>
          <span className={styles.pips} aria-hidden="true">
            {Array.from({ length: limits.maxGames }, (_, index) => (
              <i key={index} className={index < wins ? styles.pipWin : index < wins + losses ? styles.pipLoss : styles.pip}>
                {index < wins ? 'W' : index < wins + losses ? 'L' : ''}
              </i>
            ))}
          </span>
          <span aria-hidden="true">{t('career.limited.leagueRecord', { played: wins + losses, games: limits.maxGames, wins, losses })}</span>
        </div>
      ) : (
        <div className={styles.record} role="img" aria-label={t('career.limited.record', { wins, losses, maxWins: limits.maxWins, maxLosses: limits.maxLosses })}>
          <span className={styles.pips} aria-hidden="true">
            {Array.from({ length: limits.maxWins }, (_, index) => <i key={index} className={index < wins ? styles.pipWin : styles.pip}>{index < wins ? 'W' : ''}</i>)}
          </span>
          <span className={styles.pips} aria-hidden="true">
            {Array.from({ length: limits.maxLosses }, (_, index) => <i key={index} className={index < losses ? styles.pipLoss : styles.pip}>{index < losses ? 'L' : ''}</i>)}
          </span>
          <span aria-hidden="true">{t('career.limited.record', { wins, losses, maxWins: limits.maxWins, maxLosses: limits.maxLosses })}</span>
        </div>
      )}
      <p className={styles.small}>
        {t('career.limited.nowPays', { coins: rewards[Math.min(wins, rewards.length - 1)] ?? 0 })} · {t('career.limited.nextPays', { coins: run.nextReward ?? 0 })}
      </p>
      <p className={styles.small}>{t('career.limited.yourDeck', { count: deckSize })}</p>
      {added.length > 0 && (
        <details className={styles.poolSummary} open>
          <summary>{t('career.limited.added', { count: added.length })}</summary>
          <ul className={styles.names}>{added.map((card, index) => <li key={index}>{card.name}</li>)}</ul>
        </details>
      )}
      {error && <p className={styles.error} role="alert">{t('career.playFailed')}: {error}</p>}
      <div className={styles.actions}>
        <Button variant="decision" size="lg" icon={<Swords size={18} />} busy={starting} disabled={playing} onClick={() => void play()}>
          {starting ? t('career.playing') : t('career.limited.play')}
        </Button>
        <Button variant="print" size="sm" icon={<Layers size={16} />} onClick={onEdit}>{t('career.limited.editDeck')}</Button>
        {confirming ? (
          <span className={styles.confirm} role="group" aria-label={t('career.gauntlet.abandon')}>
            <span className={styles.warn}>{t('career.limited.abandonSure')}</span>
            <Button variant="danger" size="sm" onClick={() => void abandon()}>{t('career.gauntlet.abandonYes')}</Button>
            <Button variant="quiet" size="sm" onClick={() => setConfirming(false)}>{t('career.keep')}</Button>
          </span>
        ) : (
          <Button variant="quiet" size="sm" icon={<Flag size={16} />} onClick={() => setConfirming(true)}>{t('career.gauntlet.abandon')}</Button>
        )}
      </div>
    </section>
  );
}

const RUN_END: Record<string, 'career.run.won' | 'career.run.lost' | 'career.run.abandoned'> = {
  won: 'career.run.won',
  lost: 'career.run.lost',
  abandoned: 'career.run.abandoned',
};

function Recent({ runs }: { runs: CareerLimitedRun[] }) {
  const t = useT();
  if (runs.length === 0) return null;
  return (
    <section className={styles.card} aria-labelledby="limited-recent">
      <h2 id="limited-recent" className={styles.cardLabel}>{t('career.limited.recent')}</h2>
      <ul className={styles.recentRuns}>
        {runs.slice(0, 6).map((run) => (
          <li key={run.id}>
            <span>
              {run.setName} {t(run.kind === 'draft' ? 'career.limited.draft' : 'career.limited.sealed')} · {t(RUN_END[run.state ?? run.phase ?? ''] ?? 'career.run.lost')} · {t('career.run.record', { wins: run.wins ?? 0, losses: run.losses ?? 0 })}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}
