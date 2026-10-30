import {
  AlertTriangle, ArrowDownToLine, HelpCircle, Ban, BellRing, BookOpen, Dot, Flag, Flame, Heart, HeartCrack, MessageCircle,
  MessageSquare, Mountain, Shield, Shuffle, Skull, Sparkles, Swords, Trophy, Archive, Zap, X, type LucideIcon,
} from 'lucide-react';
import { memo, useEffect, useMemo, useRef, useState } from 'react';
import { DelayedRelay } from '../../core/game/broadcastDelay';
import { explainLog } from '../../core/game/logRules';
import type { GameNotice } from '../../core/game/gameSession';
import {
  buildLogRows, cardInk, collectCards, EMOTES, findLogCard, isEmote, noticeEntry, parseChatMessage, presenceChange,
  type LogEntry, type LogIcon, type LogSegment,
} from '../../core/game/gameLog';
import type { CardView, ChatMessage, GameView } from '../../protocol/generated/views';
import { api, events } from '../connection';
import { useSession } from '../stores/session';
import { useSocial } from '../stores/social';
import { IconButton } from '../ui/Button';
import { PromptText } from '../ui/PromptText';
import { useEmotes } from './emotes';
import { useMatchUi } from './matchUi';
import { usePresence } from './presence';
import styles from './GameLog.module.css';

const MAX_LINES = 300;

const ICONS: Record<LogIcon, LucideIcon> = {
  turn: Dot,
  cast: Sparkles,
  land: Mountain,
  ability: Zap,
  trigger: BellRing,
  attack: Swords,
  block: Shield,
  damage: Flame,
  lifeGain: Heart,
  lifeLoss: HeartCrack,
  draw: BookOpen,
  discard: ArrowDownToLine,
  counter: Ban,
  destroy: Skull,
  exile: Archive,
  mulligan: Shuffle,
  win: Trophy,
  lose: Flag,
  chat: MessageCircle,
  error: AlertTriangle,
  info: Dot,
};

/** Game chat and the game's running log, joined once per game (not for replays, which bring their own log). */
function useGameChat(gameId: string, view: GameView | null, live: boolean, delayMs: number) {
  // the latest view, read when a line arrives: it says whose turn the line belongs to
  const latest = useRef(view);
  useEffect(() => {
    latest.current = view;
  }, [view]);
  const [chatId, setChatId] = useState<string | null>(null);
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const [unread, setUnread] = useState(0);
  // watching with a broadcast delay: the log keeps pace with the delayed board, or it would give the game away
  const [relay] = useState(() => new DelayedRelay<() => void>((show) => show()));
  useEffect(() => relay.setDelay(delayMs), [relay, delayMs]);
  useEffect(() => () => relay.dispose(), [relay]);

  useEffect(() => {
    if (!live) return;
    let cancelled = false;
    let joined: string | null = null;
    api.chatFindByGame(gameId)
      .then(async (id) => {
        if (cancelled || !id) return;
        await api.chatJoin(id, useSession.getState().userName);
        joined = id;
        setChatId(id);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
      useEmotes.getState().clear();
      usePresence.getState().clear();
      if (joined) api.chatLeave(joined).catch(() => undefined);
    };
  }, [gameId, live]);

  useEffect(() => {
    if (!chatId) return;
    let seq = 0;
    return events.on('CHATMESSAGE', (message: ChatMessage, event) => {
      if (event.objectId !== chatId) return;
      const parsed = parseChatMessage(message, `c${seq++}`);
      if (!parsed) return;
      const me = useSession.getState().userName;
      const fromOther = parsed.tone !== 'chat' || parsed.who !== me;
      const show = () => {
        const current = latest.current;
        const entry = parsed.turn !== null && parsed.turn === current?.turn && current.activePlayerName
          ? { ...parsed, turnOwner: current.activePlayerName }
          : parsed;
        setEntries((current) => [...current, entry].slice(-MAX_LINES));
        const presence = message.messageType === 'STATUS' ? presenceChange(entry.text) : null;
        if (presence) usePresence.getState().set(presence.name, presence.online);
        if (entry.tone !== 'chat') return;
        const muted = !!useEmotes.getState().muted[gameId];
        if (fromOther && muted) return;
        if (entry.who && isEmote(entry.text)) useEmotes.getState().show(entry.who, entry.text);
        if (fromOther) setUnread((count) => count + 1);
      };
      // one's own words show at once
      if (fromOther) relay.push(show);
      else show();
    });
  }, [chatId, gameId, relay]);

  const send = (text: string) => {
    if (chatId && text.trim()) api.chatSendMessage(chatId, useSession.getState().userName, text.trim()).catch(() => undefined);
  };
  return { entries, send, ready: !!chatId, unread, clearUnread: () => setUnread(0) };
}

/** Recorded log lines, parsed once each however often the replay passes them. */
const parsedReplayLines = new WeakMap<ChatMessage, LogEntry | null>();
let replaySeq = 0;

function replayEntries(log: readonly ChatMessage[]): LogEntry[] {
  const entries: LogEntry[] = [];
  for (const message of log) {
    let entry = parsedReplayLines.get(message);
    if (entry === undefined) {
      entry = parseChatMessage(message, `r${replaySeq++}`);
      parsedReplayLines.set(message, entry);
    }
    if (entry) entries.push(entry);
  }
  return entries.slice(-MAX_LINES);
}

export function GameLog({ gameId, notices, canChat, view, replayLog, delayMs = 0 }: {
  gameId: string;
  notices: GameNotice[];
  canChat: boolean;
  view: GameView | null;
  /** replays: the log up to the moment shown */
  replayLog?: readonly ChatMessage[];
  /** watching: the broadcast delay, which the log keeps too */
  delayMs?: number;
}) {
  const open = useMatchUi((state) => state.logOpen);
  const toggle = useMatchUi((state) => state.toggleLog);
  const muted = useEmotes((state) => !!state.muted[gameId]);
  const toggleMute = useEmotes((state) => state.toggleMute);
  const myName = useSession((state) => state.userName);
  const ignored = useSocial((state) => state.ignored);
  const live = useGameChat(gameId, view, !replayLog, delayMs);
  const recorded = useMemo(() => (replayLog ? replayEntries(replayLog) : null), [replayLog]);
  const chat = recorded ? { ...live, entries: recorded } : live;
  const [draft, setDraft] = useState('');
  const list = useRef<HTMLOListElement>(null);
  // every card seen this game, so lines about cards that left the table can still show them
  const cards = useRef(new Map<string, CardView>());
  useEffect(() => {
    collectCards(view, cards.current);
  }, [view]);

  const rows = useMemo(() => {
    // the server's errors, and its messages to this player alone (why a block was refused, for one)
    const errors = notices.filter((notice) => notice.kind === 'error' || notice.kind === 'personal').map((notice) => noticeEntry(`n${notice.id}`, notice.text, notice.at));
    const blocked = new Set(ignored.map((name) => name.toLowerCase()));
    const heard = blocked.size > 0 ? chat.entries.filter((entry) => entry.tone !== 'chat' || !entry.who || !blocked.has(entry.who.toLowerCase())) : chat.entries;
    const visible = muted ? heard.filter((entry) => entry.tone !== 'chat' || entry.who === myName) : heard;
    // stable sort: lines from the same moment keep the order the server sent them in
    const entries = [...visible, ...errors].sort((a, b) => a.at - b.at);
    return buildLogRows(entries, myName);
  }, [chat.entries, notices, muted, myName, ignored]);

  useEffect(() => {
    if (open) chat.clearUnread();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, chat.unread]);

  useEffect(() => {
    list.current?.scrollTo({ top: list.current.scrollHeight });
  }, [rows.length, open]);

  return (
    <>
      <div className={styles.toggle}>
        <IconButton label={open ? 'Close log and chat' : 'Open log and chat'} icon={<MessageSquare size={22} />} pressed={open} onClick={() => toggle()} />
        {chat.unread > 0 && !open && <span className={styles.badge}>{chat.unread}</span>}
      </div>
      {open && (
        <aside className={styles.drawer} aria-label="Game log and chat">
          <header className={styles.head}>
            <h2>Log</h2>
            <div className={styles.headActions}>
              {canChat && (
                <button type="button" className={styles.mute} aria-pressed={muted} onClick={() => toggleMute(gameId)}>
                  {muted ? 'Unmute opponent' : 'Mute opponent'}
                </button>
              )}
              <IconButton label="Close log" icon={<X size={20} />} onClick={() => toggle(false)} />
            </div>
          </header>
          <ol ref={list} className={styles.lines}>
            {rows.length === 0 && <li className={styles.empty}>The game's events will show here.</li>}
            {rows.map((row) => (row.kind === 'separator'
              ? (
                <li key={row.key} className={[styles.separator, row.mine ? styles.separatorMine : ''].join(' ')}>
                  <span>Turn {row.turn}{row.player ? ` · ${row.mine ? 'You' : row.player}` : ''}</span>
                </li>
              )
              : <LogLine key={row.key} entry={row.entry} cards={cards} />))}
          </ol>
          {canChat && (
            <footer className={styles.foot}>
              <div className={styles.emotes}>
                {EMOTES.map((emote) => (
                  <button key={emote} type="button" className={styles.emote} onClick={() => chat.send(emote)} disabled={!chat.ready}>{emote}</button>
                ))}
              </div>
              <form
                className={styles.form}
                onSubmit={(event) => {
                  event.preventDefault();
                  chat.send(draft);
                  setDraft('');
                }}
              >
                <input
                  className={styles.input}
                  value={draft}
                  onChange={(event) => setDraft(event.target.value)}
                  placeholder={chat.ready ? 'Say something' : 'Joining chat…'}
                  disabled={!chat.ready}
                  aria-label="Chat message"
                  maxLength={400}
                />
              </form>
            </footer>
          )}
        </aside>
      )}
    </>
  );
}

const LogLine = memo(function LogLine({ entry, cards }: { entry: LogEntry; cards: { current: Map<string, CardView> } }) {
  const Icon = ICONS[entry.icon];
  // the rule behind a game line, on request (S009)
  const notes = useMemo(() => (entry.tone === 'game' ? explainLog(entry.text) : []), [entry.tone, entry.text]);
  const [open, setOpen] = useState(false);
  return (
    <li className={[styles.line, styles[entry.tone]].join(' ')}>
      <Icon className={[styles.icon, styles[`icon_${entry.icon}`] ?? ''].join(' ')} size={16} aria-hidden="true" />
      <span className={styles.body}>
        {entry.who && <b>{entry.who}: </b>}
        {entry.segments.map((segment, index) => <Segment key={index} segment={segment} cards={cards} />)}
        {notes.length > 0 && (
          <button
            type="button"
            className={styles.why}
            aria-expanded={open}
            aria-label={open ? 'Hide the rule behind this' : 'Why did that happen?'}
            title="Why did that happen?"
            onClick={() => setOpen((value) => !value)}
          >
            <HelpCircle size={13} aria-hidden="true" />
          </button>
        )}
        {open && notes.map((note) => (
          <span key={note.id} className={styles.rule}>
            <b>{note.title}</b>
            {note.rule !== '—' && <small> · rule {note.rule}</small>}
            <span>{note.text}</span>
          </span>
        ))}
      </span>
    </li>
  );
});

function Segment({ segment, cards }: { segment: LogSegment; cards: { current: Map<string, CardView> } }) {
  const setZoom = useMatchUi((state) => state.setZoom);
  const openDetail = useMatchUi((state) => state.openDetail);
  if (segment.kind === 'text') return <PromptText text={segment.text} />;
  if (segment.kind === 'player') return <span className={styles.player}>{segment.name}</span>;
  // looked up when used: the card may have moved since the line was written
  const find = () => findLogCard(cards.current, segment);
  return (
    <button
      type="button"
      className={[styles.card, styles[`ink_${cardInk(segment.color) ?? 'none'}`] ?? ''].join(' ')}
      onPointerEnter={(event) => {
        const card = find();
        if (card) setZoom({ card, x: event.clientX, y: event.clientY, anchor: event.currentTarget });
      }}
      onPointerLeave={() => setZoom(null)}
      onClick={() => {
        const card = find();
        if (card?.id) openDetail({ id: card.id, card });
      }}
      aria-label={`${segment.name}, show card`}
    >
      {segment.name}
    </button>
  );
}
