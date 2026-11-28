import { CalendarDays, Check, Download, FileUp, Lightbulb, Pickaxe, Users } from 'lucide-react';
import { useRef, useState } from 'react';
import type { CareerMinedPuzzle } from '../../protocol/generated/views';
import { api } from '../connection';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { usePlay } from '../stores/play';
import { Button } from '../ui/Button';
import { Field } from '../ui/Field';
import { playMode } from './careerModesData';
import { Stars } from './ModeParts';
import { clock, downloadPuzzle, parsePuzzleFile, useDailyPuzzle, useMinedPuzzles } from './puzzleData';
import { ScoreBoard } from './ScoreBoard';
import styles from './CareerModes.module.css';
import extras from './PuzzleExtras.module.css';

registerMessages(messages);

function failure(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

/** Start a puzzle game, keeping the button busy and the server's reason when it can't start. */
function useStart() {
  const playing = usePlay((play) => play.phase !== 'idle');
  const [starting, setStarting] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  async function start(id: string, begin: () => ReturnType<typeof api.careerDailyPuzzlePlay>) {
    setStarting(id);
    setError(null);
    try {
      await playMode('puzzle', begin);
    } catch (reason) {
      setError(failure(reason));
      setStarting(null);
    }
  }
  return { playing, starting, error, setError, start };
}

/** Today's puzzle (S029): the same position for everyone, a board of the day's solves, coins for the first one. */
export function DailyPuzzleCard() {
  const t = useT();
  const daily = useDailyPuzzle(true);
  const { playing, starting, error, start } = useStart();
  const [hint, setHint] = useState(false);
  const data = daily.data;
  const puzzle = data?.puzzle;
  if (daily.isError) return <p className={styles.error} role="alert">{t('career.daily.failed')}</p>;
  if (!data || !puzzle) return null;

  return (
    <section className={extras.daily} aria-labelledby="daily-puzzle">
      <div className={[styles.card, extras.dailyBody].join(' ')}>
        <header className={extras.head}>
          <CalendarDays size={20} aria-hidden="true" />
          <h2 id="daily-puzzle" className={styles.cardLabel}>{t('career.daily.title')}</h2>
          <span className={styles.small}>{data.date}</span>
          {data.solved && <span className={[styles.stateTag, styles.tag_done].join(' ')}><Check size={12} aria-hidden="true" /> {t('career.daily.solved')}</span>}
        </header>
        <h3 className={extras.name}>{puzzle.name}</h3>
        <p className={styles.puzzleGoal}>{puzzle.text}</p>
        <p className={styles.small}>
          {data.solved
            ? t('career.daily.result', { count: data.tries ?? 0, time: clock(data.seconds ?? 0) })
            : t('career.puzzle.attempts', { count: data.attempts ?? 0 })}
          {' · '}<Users size={12} aria-hidden="true" /> {t('career.daily.solvers', { count: data.solvers ?? 0 })}
        </p>
        <p className={styles.small}>{t('career.daily.reward')}</p>
        {puzzle.hint && (
          <div className={styles.hint}>
            {hint ? (
              <p role="status"><Lightbulb size={16} aria-hidden="true" /> {puzzle.hint}</p>
            ) : (
              <Button variant="quiet" size="sm" icon={<Lightbulb size={16} />} onClick={() => setHint(true)}>{t('career.puzzle.showHint')}</Button>
            )}
          </div>
        )}
        {error && <p className={styles.error} role="alert">{t('career.playFailed')}: {error}</p>}
        <div className={styles.actions}>
          <Button variant="decision" busy={starting === 'daily'} disabled={playing} onClick={() => void start('daily', () => api.careerDailyPuzzlePlay())}>
            {starting === 'daily' ? t('career.playing') : t(data.solved ? 'career.daily.again' : 'career.daily.play')}
          </Button>
        </div>
      </div>
      {data.date && <ScoreBoard mode="daily-puzzle" board={data.date} title={t('career.daily.board')} />}
    </section>
  );
}

/** Positions mined from the AI's own games (S031), deepest first as the server sends them. */
export function MinedPuzzles() {
  const t = useT();
  const mined = useMinedPuzzles(true);
  const { playing, starting, error, start } = useStart();
  const list = mined.data ?? [];

  return (
    <section className={extras.section} aria-labelledby="mined-puzzles">
      <h2 id="mined-puzzles" className={styles.subhead}><Pickaxe size={14} aria-hidden="true" /> {t('career.mined.title')}</h2>
      {mined.isError && <p className={styles.error} role="alert">{t('career.failed')}</p>}
      {mined.data && list.length === 0 && <p className={styles.small}>{t('career.mined.empty')}</p>}
      {error && <p className={styles.error} role="alert">{t('career.playFailed')}: {error}</p>}
      {list.length > 0 && (
        <ul className={extras.mined}>
          {list.map((puzzle) => (
            <MinedRow key={puzzle.id} puzzle={puzzle} busy={starting === puzzle.id} disabled={playing}
              onPlay={() => void start(puzzle.id!, () => api.careerMinedPuzzlePlay(puzzle.id!))} />
          ))}
        </ul>
      )}
    </section>
  );
}

function MinedRow({ puzzle, busy, disabled, onPlay }: { puzzle: CareerMinedPuzzle; busy: boolean; disabled: boolean; onPlay(): void }) {
  const t = useT();
  return (
    <li className={extras.minedRow}>
      <div className={extras.minedText}>
        <strong>{puzzle.name}</strong>
        <span className={styles.small}>{puzzle.text}</span>
        <span className={styles.small}>
          {t('career.mined.depth', { count: Math.max(1, (puzzle.depth ?? 1) - 1) })}
          {(puzzle.attempts ?? 0) > 0 && <> · {t('career.puzzle.attempts', { count: puzzle.attempts ?? 0 })}</>}
        </span>
      </div>
      <Stars stars={puzzle.stars} size={12} />
      <Button variant="print" size="sm" busy={busy} disabled={disabled} onClick={onPlay}>
        {t((puzzle.stars ?? 0) > 0 ? 'career.puzzle.again' : 'career.puzzle.play')}
      </Button>
    </li>
  );
}

/** Play a puzzle from a .json file (S030); the server checks it and says what's wrong. */
export function PuzzleFileButton() {
  const t = useT();
  const input = useRef<HTMLInputElement>(null);
  const { playing, starting, error, setError, start } = useStart();

  async function chosen(file: File | undefined) {
    if (!file) return;
    let text: string;
    try {
      text = await file.text();
    } catch (reason) {
      setError(failure(reason));
      return;
    }
    let puzzle;
    try {
      puzzle = parsePuzzleFile(text);
    } catch {
      setError(t('career.puzzleFile.invalid'));
      return;
    }
    await start('file', () => api.careerPuzzleFilePlay(puzzle));
  }

  return (
    <div className={extras.file}>
      <input
        ref={input}
        type="file"
        accept=".json,application/json"
        hidden
        onChange={(event) => {
          const file = event.target.files?.[0];
          event.target.value = '';
          void chosen(file);
        }}
      />
      <Button variant="quiet" size="sm" icon={<FileUp size={16} />} busy={starting === 'file'} disabled={playing} onClick={() => input.current?.click()}>
        {t('career.puzzleFile.open')}
      </Button>
      {error && <p className={styles.error} role="alert">{t('career.puzzleFile.failed')}: {error}</p>}
    </div>
  );
}

/** Save the game's position as a puzzle file (S030); for a 2-player game against the computer. */
export function SavePuzzle({ gameId }: { gameId: string }) {
  const t = useT();
  const [name, setName] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [left, setLeft] = useState<string[] | null>(null);

  async function save() {
    setBusy(true);
    setError(null);
    setLeft(null);
    try {
      const typed = name.trim() || null;
      const exported = await api.gamePuzzleExport(gameId, typed, null, null);
      if (exported.puzzle) downloadPuzzle(exported.puzzle, exported.puzzle.name ?? typed ?? '');
      setLeft(exported.left ?? []);
    } catch (reason) {
      setError(failure(reason));
    } finally {
      setBusy(false);
    }
  }

  return (
    <form
      className={extras.save}
      onSubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      <div className={extras.saveRow}>
        <Field label={t('career.puzzleSave.name')} value={name} onChange={(event) => setName(event.target.value)} placeholder={t('career.puzzleSave.placeholder')} />
        <Button type="submit" variant="print" icon={<Download size={16} />} busy={busy}>{t('career.puzzleSave.action')}</Button>
      </div>
      {left && left.length === 0 && <p className={styles.small} role="status">{t('career.puzzleSave.saved')}</p>}
      {left && left.length > 0 && (
        <div className={styles.small} role="status">
          <p>{t('career.puzzleSave.left')}</p>
          <ul className={extras.left}>{left.map((item, index) => <li key={index}>{item}</li>)}</ul>
        </div>
      )}
      {error && <p className={styles.error} role="alert">{error}</p>}
    </form>
  );
}
