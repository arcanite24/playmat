import { ChevronFirst, ChevronLast, GitBranch, Pause, Play, StepBack, StepForward } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams, useSearchParams } from 'react-router-dom';
import { useStore } from 'zustand';
import { GameSession } from '../../core/game/gameSession';
import { EventBus } from '../../core/rpc/EventBus';
import { appendEvents, EMPTY_TIMELINE, frameDelay, logsAt, turnJump, viewAt, type ReplayTimeline } from '../../core/replay/timeline';
import type { ReplayInfo } from '../../protocol/generated/views';
import { api } from '../connection';
import { MatchErrorBoundary } from '../match/MatchErrorBoundary';
import { MatchStage } from '../match/MatchStage';
import { playFromTurn, useSoloTurns } from '../solo/soloData';
import { useSession } from '../stores/session';
import { notify } from '../stores/toasts';
import { Button, IconButton } from '../ui/Button';
import styles from './ReplayScreen.module.css';

/** events asked for at a time: snapshots are large, so pages stay small and the replay starts early */
const PAGE_SIZE = 40;
const SPEEDS = [0.5, 1, 2, 4];

/** a replay has no live connection: its session is fed snapshots by hand */
const NO_EVENTS = {
  onEvent: () => () => undefined,
  onSessionStart: () => () => undefined,
  onStatus: () => () => undefined,
};

/** Plays a recorded game back on the match stage: play, pause, step, jump by turn, scrub and change speed. */
export function ReplayScreen() {
  const { gameId = '' } = useParams();
  const navigate = useNavigate();
  // a review links to a moment: open there, paused
  const [search] = useSearchParams();
  const startFrame = Number.parseInt(search.get('frame') ?? '', 10);
  const userName = useSession((state) => state.userName);
  const [timeline, setTimeline] = useState<ReplayTimeline>(EMPTY_TIMELINE);
  const [info, setInfo] = useState<ReplayInfo | null>(null);
  const [total, setTotal] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [index, setIndex] = useState(0);
  const [playing, setPlaying] = useState(!Number.isFinite(startFrame));
  const [speed, setSpeed] = useState(1);
  const session = useMemo(() => new GameSession(api, new EventBus(NO_EVENTS), { gameId, playerId: null, mode: 'replay' }), [gameId]);
  useEffect(() => () => session.dispose(), [session]);
  const state = useStore(session.store);

  // pages come in one after the other; playback starts with the first
  useEffect(() => {
    let cancelled = false;
    setTimeline(EMPTY_TIMELINE);
    setIndex(Number.isFinite(startFrame) ? Math.max(0, startFrame) : 0);
    void (async () => {
      let current = EMPTY_TIMELINE;
      for (;;) {
        const page = await api.replayPage(gameId, current.read, PAGE_SIZE);
        if (cancelled) return;
        if (!page) {
          setError('This replay is not on the server any more.');
          return;
        }
        current = appendEvents(current, page.events ?? []);
        setInfo(page.info);
        setTotal(page.total);
        setTimeline(current);
        if ((page.events ?? []).length === 0 || current.read >= page.total) return;
      }
    })().catch((reason) => {
      if (!cancelled) setError(reason instanceof Error ? reason.message : String(reason));
    });
    return () => {
      cancelled = true;
    };
    // the starting frame only matters when the replay opens
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [gameId]);

  const frames = timeline.frames;
  const loaded = total !== null && timeline.read >= total;
  const frame = frames[Math.min(index, frames.length - 1)] ?? null;
  const ownSeat = useMemo(
    () => frames[0]?.view.players?.find((player) => player.name === userName)?.playerId ?? null,
    [frames, userName],
  );

  // a player watches their own game from their seat, hand and all
  useEffect(() => {
    if (ownSeat && session.getState().playerId === null) session.setPerspective(ownSeat);
  }, [ownSeat, session]);

  useEffect(() => {
    if (frame) session.showFrame(viewAt(frame, !!ownSeat && state.playerId === ownSeat));
  }, [frame, session, ownSeat, state.playerId]);

  // playback: wait out the frame, then move on; at the end, wait for more pages or stop
  useEffect(() => {
    if (!playing || frames.length === 0) return;
    if (index >= frames.length - 1) {
      if (loaded) setPlaying(false);
      return;
    }
    const timer = setTimeout(() => setIndex((current) => current + 1), frameDelay(timeline, index, speed));
    return () => clearTimeout(timer);
  }, [playing, index, speed, timeline, frames.length, loaded]);

  const last = Math.max(0, frames.length - 1);
  const go = (next: number) => setIndex(Math.max(0, Math.min(last, next)));

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target && (target.tagName === 'INPUT' && (target as HTMLInputElement).type !== 'range' || target.tagName === 'SELECT' || target.tagName === 'TEXTAREA')) return;
      if (event.key === ' ') {
        event.preventDefault();
        setPlaying((current) => !current);
      } else if (event.key === 'ArrowRight') {
        event.preventDefault();
        setPlaying(false);
        setIndex((current) => (event.shiftKey ? turnJump(timeline, current, 1) : Math.min(last, current + 1)));
      } else if (event.key === 'ArrowLeft') {
        event.preventDefault();
        setPlaying(false);
        setIndex((current) => (event.shiftKey ? turnJump(timeline, current, -1) : Math.max(0, current - 1)));
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [timeline, last]);

  // your own games against the computer keep every turn: take over from the start of one (S014)
  const turns = useSoloTurns(gameId);
  const kept = useMemo(() => new Set((turns.data ?? []).map((entry) => entry.turn)), [turns.data]);
  async function takeOver(turn: number) {
    setPlaying(false);
    try {
      await playFromTurn(gameId, turn);
    } catch (failure) {
      notify('Play from here', failure instanceof Error ? failure.message : String(failure));
    }
  }

  const replayLog = useMemo(() => logsAt(timeline, Math.min(index, last)), [timeline, index, last]);
  const leave = () => navigate('/history');

  if (error) {
    return (
      <div className={styles.missing}>
        <h1>Can't play this replay</h1>
        <p>{error}</p>
        <Button variant="print" onClick={leave}>Back to History</Button>
      </div>
    );
  }
  if (!frame) {
    return (
      <div className={styles.missing}>
        <p role="status">Loading the replay…</p>
      </div>
    );
  }

  const turn = frame.view.turn ?? 0;
  return (
    <MatchErrorBoundary gameId={gameId} backLabel="Back to History" onBack={leave}>
      <MatchStage session={session} state={state} replayLog={replayLog} onLeave={leave} />
      <div className={styles.transport} role="toolbar" aria-label="Replay controls">
        <IconButton label="Previous turn" icon={<ChevronFirst size={18} />} onClick={() => { setPlaying(false); go(turnJump(timeline, index, -1)); }} />
        <IconButton label="Step back" icon={<StepBack size={18} />} onClick={() => { setPlaying(false); go(index - 1); }} disabled={index === 0} />
        <IconButton
          label={playing ? 'Pause' : 'Play'}
          icon={playing ? <Pause size={18} /> : <Play size={18} />}
          pressed={playing}
          onClick={() => {
            if (!playing && index >= last && loaded) setIndex(0);
            setPlaying(!playing);
          }}
        />
        <IconButton label="Step forward" icon={<StepForward size={18} />} onClick={() => { setPlaying(false); go(index + 1); }} disabled={index >= last} />
        <IconButton label="Next turn" icon={<ChevronLast size={18} />} onClick={() => { setPlaying(false); go(turnJump(timeline, index, 1)); }} />
        <input
          className={styles.scrub}
          type="range"
          min={0}
          max={last}
          value={Math.min(index, last)}
          onChange={(event) => go(Number(event.target.value))}
          aria-label="Position in the replay"
          aria-valuetext={`Turn ${turn}, moment ${index + 1} of ${frames.length}`}
        />
        <span className={styles.where}>
          Turn {turn}{frame.view.activePlayerName ? ` · ${frame.view.activePlayerName}` : ''}
          {!loaded && total ? <span className={styles.loading}> · loading {Math.round((timeline.read / total) * 100)}%</span> : null}
        </span>
        <label className={styles.speed}>
          <span className="visually-hidden">Speed</span>
          <select value={speed} onChange={(event) => setSpeed(Number(event.target.value))}>
            {SPEEDS.map((value) => <option key={value} value={value}>{value}×</option>)}
          </select>
        </label>
        {kept.has(turn) && (
          <Button size="sm" variant="print" icon={<GitBranch size={15} />} onClick={() => void takeOver(turn)} title={`Play on from the start of turn ${turn} against the computer`}>
            Play from turn {turn}
          </Button>
        )}
        {info?.truncated && <span className={styles.loading} title="The recording stopped early">partial</span>}
      </div>
    </MatchErrorBoundary>
  );
}
