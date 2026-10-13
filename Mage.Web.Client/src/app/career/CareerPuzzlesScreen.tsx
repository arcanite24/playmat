import { Check, Heart, Lightbulb, Lock, Puzzle } from 'lucide-react';
import { useState } from 'react';
import { Navigate } from 'react-router-dom';
import type { CareerPuzzle, CareerPuzzleSide } from '../../protocol/generated/views';
import { api } from '../connection';
import { registerMessages, useT, type MessageKey } from '../i18n';
import messages from '../i18n/en/career';
import { usePlay } from '../stores/play';
import { Button } from '../ui/Button';
import { useCareerState } from './careerData';
import { playMode, usePuzzles } from './careerModesData';
import { puzzleZones } from './careerModesModel';
import { ModeResult, Stars } from './ModeParts';
import styles from './CareerModes.module.css';
import { PuzzleBoard } from './PuzzleBoard';

registerMessages(messages);

/** The puzzle book: fixed positions to win in one turn, solved in order, starred by how few tries they took. */
export function CareerPuzzlesScreen() {
  const t = useT();
  const state = useCareerState();
  const profile = state.data?.profile;
  const puzzles = usePuzzles(!!profile);
  const [chosenId, setChosenId] = useState<string | null>(null);

  if (state.isPending) return <p className={styles.note}>{t('career.loading')}</p>;
  if (!profile) return <Navigate to="/career" replace />;
  const list = puzzles.data ?? [];
  const chosen = list.find((puzzle) => puzzle.id === chosenId) ?? list.find((puzzle) => puzzle.open && !puzzle.solved) ?? list[0];
  const solved = list.filter((puzzle) => puzzle.solved).length;
  const stars = list.reduce((sum, puzzle) => sum + (puzzle.stars ?? 0), 0);
  // the book first, then each series (S021's stack drills) in the order they come
  const groups: { series: string; puzzles: CareerPuzzle[] }[] = [];
  for (const puzzle of list) {
    const series = puzzle.series ?? '';
    const group = groups.find((item) => item.series === series);
    if (group) group.puzzles.push(puzzle);
    else groups.push({ series, puzzles: [puzzle] });
  }

  return (
    <div className={styles.page}>
      <div className={styles.scroll}>
        <ModeResult kind="puzzle" />
        <header className={styles.modeHead}>
          <div>
            <h1 className={styles.modeTitle}>{t('career.puzzles.title')}</h1>
            <p className={styles.lead}>{t('career.puzzles.lead')}</p>
          </div>
          {list.length > 0 && <p className={styles.progressNote}>{t('career.puzzles.progress', { solved, total: list.length, stars, max: list.length * 3 })}</p>}
        </header>
        {puzzles.isPending && <p className={styles.note}>{t('career.loading')}</p>}
        {puzzles.isError && <p className={styles.error} role="alert">{t('career.failed')}</p>}
        <div className={styles.split}>
          <div className={styles.puzzleGroups}>
            {groups.map((group) => (
              <section key={group.series} aria-label={group.series || t('career.puzzles.book')}>
                {groups.length > 1 && <h2 className={styles.subhead}>{group.series || t('career.puzzles.book')}</h2>}
                <ol className={styles.puzzleList} aria-label={group.series || t('career.puzzles.title')}>
                  {group.puzzles.map((puzzle) => (
                    <li key={puzzle.id}>
                      <button
                        type="button"
                        className={[styles.puzzleTile, puzzle.id === chosen?.id ? styles.puzzleTileOn : '', puzzle.open ? '' : styles.puzzleLocked].join(' ')}
                        aria-pressed={puzzle.id === chosen?.id}
                        aria-label={[
                          t('career.puzzle.numbered', { number: puzzle.number ?? 0, name: puzzle.name ?? '' }),
                          t(puzzle.solved ? 'career.puzzle.solved' : puzzle.open ? 'career.node.open' : 'career.node.locked'),
                          t('career.puzzle.stars', { count: puzzle.stars ?? 0 }),
                        ].join(', ')}
                        onClick={() => setChosenId(puzzle.id ?? null)}
                      >
                        <span className={styles.puzzleNumber} aria-hidden="true">{puzzle.number}</span>
                        <span className={styles.puzzleName} aria-hidden="true">{puzzle.name}</span>
                        <span className={styles.puzzleMark} aria-hidden="true">
                          {!puzzle.open ? <Lock size={14} /> : puzzle.solved ? <Check size={14} /> : null}
                        </span>
                        <span aria-hidden="true"><Stars stars={puzzle.stars} size={12} /></span>
                      </button>
                    </li>
                  ))}
                </ol>
              </section>
            ))}
          </div>
          {chosen && <PuzzleDetail key={chosen.id} puzzle={chosen} />}
        </div>
      </div>
    </div>
  );
}

function PuzzleDetail({ puzzle }: { puzzle: CareerPuzzle }) {
  const t = useT();
  const [hint, setHint] = useState(false);
  const playing = usePlay((play) => play.phase !== 'idle');
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function play() {
    setStarting(true);
    setError(null);
    try {
      await playMode('puzzle', () => api.careerPuzzlePlay(puzzle.id!));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
      setStarting(false);
    }
  }

  return (
    <article className={styles.detail} aria-labelledby={`puzzle-${puzzle.id}`}>
      <div className={styles.detailBody}>
        <header className={styles.detailHead}>
          <Puzzle size={22} aria-hidden="true" />
          <h2 id={`puzzle-${puzzle.id}`}>{t('career.puzzle.numbered', { number: puzzle.number ?? 0, name: puzzle.name ?? '' })}</h2>
          <Stars stars={puzzle.stars} />
          <span className={[styles.stateTag, puzzle.solved ? styles.tag_done : puzzle.open ? styles.tag_open : styles.tag_locked].join(' ')}>
            {t(puzzle.solved ? 'career.puzzle.solved' : puzzle.open ? 'career.node.open' : 'career.node.locked')}
          </span>
        </header>
        <p className={styles.puzzleGoal}>{puzzle.text}</p>
        <p className={styles.small}>{objectiveLine(t, puzzle)}</p>
        <p className={styles.small}>
          {t('career.puzzle.attempts', { count: puzzle.attempts ?? 0 })} · {t('career.puzzle.starRule')}
        </p>
        {error && <p className={styles.error} role="alert">{t('career.playFailed')}: {error}</p>}

        <div className={styles.position}>
          <Side title={t('career.puzzle.opponent')} side={puzzle.opponent} />
          <Side title={t('career.puzzle.you')} side={puzzle.you} />
        </div>

        <div className={styles.hint}>
          {hint ? (
            <p role="status"><Lightbulb size={16} aria-hidden="true" /> {puzzle.hint}</p>
          ) : (
            <Button variant="quiet" size="sm" icon={<Lightbulb size={16} />} onClick={() => setHint(true)} disabled={!puzzle.hint}>{t('career.puzzle.showHint')}</Button>
          )}
        </div>

        <div className={styles.actions}>
          {puzzle.open ? (
            <Button variant="decision" busy={starting} disabled={playing} onClick={() => void play()}>
              {starting ? t('career.playing') : t(puzzle.solved ? 'career.puzzle.again' : 'career.puzzle.play')}
            </Button>
          ) : (
            <p className={styles.small}><Lock size={14} aria-hidden="true" /> {t('career.puzzle.lockedHint')}</p>
          )}
        </div>
        {puzzle.open && <PuzzleBoard puzzleId={puzzle.id!} />}
      </div>
    </article>
  );
}

function objectiveLine(t: ReturnType<typeof useT>, puzzle: CareerPuzzle): string {
  if (puzzle.objective === 'protect') return t('career.puzzle.objectiveProtect', { name: puzzle.protect ?? '' });
  if (puzzle.objective === 'survive') return t('career.puzzle.objectiveSurvive');
  return t('career.puzzle.objectiveWin');
}

const ZONE_KEYS: Record<'battlefield' | 'hand' | 'graveyard', MessageKey> = {
  battlefield: 'career.puzzle.battlefield',
  hand: 'career.puzzle.hand',
  graveyard: 'career.puzzle.graveyard',
};

/** One side of the starting position: life, then each zone's cards by name. */
function Side({ title, side }: { title: string; side: CareerPuzzleSide | undefined }) {
  const t = useT();
  const zones = puzzleZones(side);
  const library = side?.library?.length ?? 0;
  return (
    <section className={styles.side} aria-label={title}>
      <header className={styles.sideHead}>
        <h3>{title}</h3>
        <span className={styles.life}><Heart size={14} aria-hidden="true" /> {t('career.puzzle.life', { life: side?.life ?? 20 })}</span>
      </header>
      {zones.length === 0 && <p className={styles.small}>{t('career.puzzle.empty')}</p>}
      {zones.map((zone) => (
        <div key={zone.zone} className={styles.zone}>
          <h4 className={styles.subhead}>{t(ZONE_KEYS[zone.zone])}</h4>
          <ul className={styles.names}>
            {zone.cards.map((card) => (
              <li key={`${card.name}-${card.tapped}`}>
                {card.count > 1 ? `${card.count} × ${card.name}` : card.name}
                {card.tapped && <small> ({t('career.puzzle.tapped')})</small>}
              </li>
            ))}
          </ul>
        </div>
      ))}
      <p className={styles.small}>{t('career.puzzle.library', { count: library })}</p>
    </section>
  );
}
