import { Bot, Eye, Link2, Lock, MessageSquare, Plus, UserRound } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { api } from '../connection';
import { toWire } from '../decks/deckModel';
import { refreshTables, useServerState, useTables } from '../queries';
import { useDecks } from '../stores/decks';
import { useDeckRoster } from '../stores/useDeckRoster';
import { useSession } from '../stores/session';
import { notify } from '../stores/toasts';
import type { TableView } from '../../protocol/generated/views';
import { Button, IconButton } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import { Field } from '../ui/Field';
import { warmDeck } from '../ui/imageCache';
import { Zone } from '../ui/Zone';
import { formatMenu, formatRules, gameTypesFor, isCommanderFormat } from '../../core/decks/formats';
import { DEFAULT_ADVANCED, matchOptions, type TableAdvanced } from '../../core/game/tableSetup';
import { inviteLink } from '../../core/social/chatLine';
import { ChatPanel } from '../social/ChatPanel';
import { NameLevel } from '../social/LevelBadge';
import { useChatRoom } from '../social/useChatRoom';
import { useSocial } from '../stores/social';
import { joinTable } from './joinTable';
import { TableAdvancedFields } from './TableAdvancedFields';
import styles from './TablesScreen.module.css';
import { PUBLIC_ORIGIN } from '../brand';
import { registerMessages, useT, type MessageKey } from '../i18n';
import { RichText } from '../i18n/RichText';
import messages from '../i18n/en/tables';

registerMessages(messages);

const FREEFORM = 'Constructed - Freeform';

const STATES = new Set(['WAITING', 'READY_TO_START', 'STARTING', 'DRAFTING', 'CONSTRUCTING', 'DUELING', 'SIDEBOARDING', 'FINISHED']);

/** Every table on the server: join one with your selected deck, watch a game, or host your own. */
export function TablesScreen() {
  const t = useT();
  const location = useLocation();
  const { data: tables = [], isLoading } = useTables();
  const userName = useSession((state) => state.userName);
  const roomId = useSession((state) => state.roomId);
  const decks = useDecks();
  const roster = useDeckRoster();
  const selectedDeck = roster.find((deck) => deck.id === decks.selectedId) ?? roster[0] ?? null;
  const [hostOpen, setHostOpen] = useState(!!(location.state as { host?: string } | null)?.host);
  const [busyTable, setBusyTable] = useState<string | null>(null);
  // a passworded table asks for the password in a dialog before joining
  const [locked, setLocked] = useState<TableView | null>(null);
  const [tablePassword, setTablePassword] = useState('');

  useEffect(() => {
    if (!decks.loaded) void decks.refresh();
  }, [decks]);

  const games = tables.filter((table) => !table.isTournament);
  const mine = games.filter((table) => table.seats?.some((seat) => seat.playerName === userName) && table.tableState !== 'FINISHED');
  const open = games.filter((table) => table.tableState === 'WAITING' && !mine.includes(table));
  const running = games.filter((table) => table.tableState !== 'WAITING' && table.tableState !== 'FINISHED' && !mine.includes(table));

  async function join(table: TableView, password?: string) {
    if (!roomId || !table.tableId || !selectedDeck) return;
    if (table.passworded && password === undefined) {
      setTablePassword('');
      setLocked(table);
      return;
    }
    setBusyTable(table.tableId);
    try {
      await joinTable(table, selectedDeck.id, password ?? '');
    } catch (error) {
      notify(t('tables.joinFailed'), error instanceof Error ? error.message : String(error), 'error');
    } finally {
      setBusyTable(null);
    }
  }

  async function watch(table: TableView) {
    if (!roomId || !table.tableId) return;
    const ok = await api.roomWatchTable(roomId, table.tableId).catch(() => false);
    if (!ok) notify(t('tables.watchFailed'), t('tables.watchFailed.detail'), 'error');
  }

  return (
    <div className={styles.page}>
      <div className={styles.columns}>
        <Zone
          label={t('tables.open')}
          aside={<Button size="sm" variant="print" icon={<Plus size={16} />} onClick={() => setHostOpen(true)}>{t('tables.host')}</Button>}
          className={styles.list}
        >
          {mine.length > 0 && (
            <div className={styles.group}>
              <h3 className={styles.groupLabel}>{t('tables.yours')}</h3>
              {mine.map((table) => (
                <MyTable key={table.tableId} table={table} />
              ))}
            </div>
          )}
          <div className={styles.group}>
            {open.length === 0 && !isLoading && (
              <p className={styles.empty}>{t('tables.noneWaiting')}</p>
            )}
            {open.map((table) => (
              <TableRow key={table.tableId} table={table}>
                <Button size="sm" busy={busyTable === table.tableId} disabled={!selectedDeck} onClick={() => void join(table)}>
                  {t('tables.join')}
                </Button>
              </TableRow>
            ))}
          </div>
        </Zone>

        <Zone label={t('tables.running')} className={styles.list}>
          {running.length === 0 && <p className={styles.empty}>{t('tables.noneRunning')}</p>}
          {running.map((table) => (
            <TableRow key={table.tableId} table={table}>
              {table.spectatorsAllowed && (
                <Button size="sm" variant="quiet" icon={<Eye size={16} />} onClick={() => void watch(table)}>{t('tables.watch')}</Button>
              )}
            </TableRow>
          ))}
        </Zone>
      </div>
      <p className={styles.deckNote}>
        <RichText text={t('tables.joiningWith')} parts={{ deck: <strong>{selectedDeck?.name ?? t('tables.noDeck')}</strong> }} />
      </p>
      <Dialog
        open={!!locked}
        onOpenChange={(open) => !open && setLocked(null)}
        title={t('tables.password.title')}
        description={locked ? t('tables.password.private', { name: locked.tableName ?? '' }) : undefined}
        width="sm"
        footer={(
          <>
            <Button variant="quiet" onClick={() => setLocked(null)}>{t('tables.cancel')}</Button>
            <Button
              variant="decision"
              disabled={!tablePassword}
              onClick={() => {
                const table = locked!;
                setLocked(null);
                void join(table, tablePassword);
              }}
            >
              {t('tables.join')}
            </Button>
          </>
        )}
      >
        <form
          onSubmit={(event) => {
            event.preventDefault();
            if (!locked || !tablePassword) return;
            const table = locked;
            setLocked(null);
            void join(table, tablePassword);
          }}
        >
          <Field label={t('tables.password')} type="password" value={tablePassword} onChange={(event) => setTablePassword(event.target.value)} autoFocus />
        </form>
      </Dialog>
      <HostDialog open={hostOpen} onOpenChange={setHostOpen} deckId={selectedDeck?.id ?? null} />
    </div>
  );
}

function TableRow({ table, children }: { table: TableView; children?: React.ReactNode }) {
  const t = useT();
  const state = table.tableState ?? '';
  return (
    <article className={styles.row}>
      <div className={styles.rowMain}>
        <h3 className={styles.tableName}>
          {table.passworded && <Lock size={14} aria-label={t('tables.passwordRequired')} />}
          {table.tableName}
        </h3>
        <p className={styles.tableMeta}>
          {table.gameType} · {(table.deckType ?? '').replace(/^Constructed - /, '')} · {table.controllerName}
        </p>
      </div>
      <ul className={styles.seats} aria-label={t('tables.seats')}>
        {(table.seats ?? []).map((seat, index) => (
          <li key={index} className={seat.playerName ? styles.seatTaken : styles.seatOpen} title={seat.playerName || t('tables.openSeat')}>
            {seat.playerType && seat.playerType !== 'HUMAN' ? <Bot size={14} aria-hidden="true" /> : <UserRound size={14} aria-hidden="true" />}
            <span className={styles.seatName}>{seat.playerName || t('tables.seatOpen')}</span>
            {seat.playerName && (!seat.playerType || seat.playerType === 'HUMAN') && <NameLevel name={seat.playerName} className={styles.seatLevel} />}
          </li>
        ))}
      </ul>
      <span className={styles.state}>{STATES.has(state) ? t(`tables.state.${state}` as MessageKey) : table.tableStateText}</span>
      <div className={styles.rowActions}>{children}</div>
    </article>
  );
}

function MyTable({ table }: { table: TableView }) {
  const t = useT();
  const roomId = useSession((state) => state.roomId);
  const userName = useSession((state) => state.userName);
  const [chatOpen, setChatOpen] = useState(false);
  // the server lists the controller first, then the other seated players: "host, guest"
  const isOwner = (table.controllerName ?? '').split(', ')[0] === userName;
  const ready = table.tableState === 'READY_TO_START';
  const copyInvite = () => {
    if (!table.tableId) return;
    const link = inviteLink(PUBLIC_ORIGIN, table.tableId);
    navigator.clipboard.writeText(link)
      .then(() => notify(t('tables.inviteCopied'), t(table.passworded ? 'tables.inviteCopied.password' : 'tables.inviteCopied.open')))
      .catch(() => notify(t('tables.inviteLink'), link));
  };
  return (
    <>
      <TableRow table={table}>
        {table.tableState === 'WAITING' && (
          <Button size="sm" variant="quiet" icon={<Link2 size={16} />} onClick={copyInvite}>{t('tables.invite')}</Button>
        )}
        <IconButton label={t(chatOpen ? 'tables.chat.hide' : 'tables.chat')} icon={<MessageSquare size={16} />} pressed={chatOpen} onClick={() => setChatOpen(!chatOpen)} />
        {isOwner && table.tableState !== 'DUELING' && (
          <>
            <Button size="sm" variant="quiet" onClick={() => roomId && table.tableId && void api.tableRemove(roomId, table.tableId).finally(refreshTables)}>{t('tables.close')}</Button>
            <Button size="sm" variant={ready ? 'decision' : 'print'} disabled={!ready} onClick={() => roomId && table.tableId && void api.matchStart(roomId, table.tableId)}>
              {t('tables.start')}
            </Button>
          </>
        )}
        {!isOwner && table.tableState === 'WAITING' && (
          <Button size="sm" variant="quiet" onClick={() => roomId && table.tableId && void api.roomLeaveTableOrTournament(roomId, table.tableId).finally(refreshTables)}>{t('tables.leave')}</Button>
        )}
      </TableRow>
      {chatOpen && table.tableId && <TableChat tableId={table.tableId} />}
    </>
  );
}

/** The chat of a table you sit at: talk to the others while seats fill. */
function TableChat({ tableId }: { tableId: string }) {
  const t = useT();
  const chat = useChatRoom(tableId, (id) => api.chatFindByTable(id));
  return (
    <div className={styles.tableChat}>
      <ChatPanel label={t('tables.chat')} lines={chat.lines} ready={chat.ready} onSend={chat.send} empty={t('tables.chat.empty')} />
    </div>
  );
}

function HostDialog({ open, onOpenChange, deckId }: { open: boolean; onOpenChange(open: boolean): void; deckId: string | null }) {
  const t = useT();
  const { data: serverState } = useServerState();
  const roomId = useSession((state) => state.roomId);
  const userName = useSession((state) => state.userName);
  const loadForPlay = useDecks((state) => state.loadForPlay);
  const loadList = useDecks((state) => state.loadList);
  const [name, setName] = useState('');
  const [deckType, setDeckType] = useState(FREEFORM);
  const [gameType, setGameType] = useState('Two Player Duel');
  const [seats, setSeats] = useState(2);
  const [wins, setWins] = useState(2);
  const [aiSeats, setAiSeats] = useState(0);
  const [password, setPassword] = useState('');
  const [advanced, setAdvanced] = useState<TableAdvanced>(DEFAULT_ADVANCED);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // the table takes the format of the deck you join with
  useEffect(() => {
    if (!open || !deckId) return;
    let cancelled = false;
    void loadList(deckId).then((deck) => {
      if (!cancelled && deck.format) setDeckType(deck.format);
    }).catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [open, deckId, loadList]);

  const allTypes = useMemo(() => serverState?.gameTypes ?? [], [serverState]);
  const gameTypes = useMemo(() => gameTypesFor(deckType, allTypes.map((item) => item.name!).filter(Boolean)), [deckType, allTypes]);
  const chosenGame = gameTypes.includes(gameType) ? gameType : gameTypes[0] ?? gameType;
  const type = allTypes.find((item) => item.name === chosenGame);
  const minSeats = type?.minPlayers ?? 2;
  const maxSeats = Math.min(type?.maxPlayers ?? 2, 6);
  const seatCount = Math.min(maxSeats, Math.max(minSeats, seats));
  const commander = isCommanderFormat(deckType);
  const menu = formatMenu(serverState?.deckTypes ?? []);
  const listed = menu.some((group) => group.types.includes(deckType));

  async function host() {
    if (!roomId || !deckId) return;
    setBusy(true);
    setError(null);
    try {
      const { deck } = await loadForPlay(deckId);
      // the pictures download while the table fills, so the first hand lands with its art
      warmDeck(deck);
      const playerTypes = Array.from({ length: seatCount }, (_, index) => (index > 0 && index <= aiSeats ? 'COMPUTER_MAD' : 'HUMAN'));
      const table = await api.roomCreateTable(roomId, matchOptions({
        name: name.trim() || t('tables.host.defaultName', { name: userName }),
        gameType: chosenGame,
        deckType,
        // a free-for-all is one game
        winsNeeded: seatCount > 2 ? 1 : wins,
        password,
        playerTypes,
        advanced,
        // players you ignore can't sit at your table or watch it
        bannedUsers: useSocial.getState().ignored,
      }));
      if (!table?.tableId) throw new Error(t('tables.host.notCreated'));
      const wire = toWire(deck);
      if (!await api.roomJoinTable(roomId, table.tableId, userName, 'HUMAN', 1, wire, password)) {
        throw new Error(t('tables.host.illegal', { format: formatRules(deckType).label }));
      }
      if (aiSeats > 0) {
        // the AI brings the format's own decks, or starter decks of the right kind; a deck that doesn't fit gets a copy of yours
        const rivals = await (await import('../stores/aiDecks')).aiDecks([], aiSeats, commander, { format: deckType }).catch(() => []);
        rivals.forEach(warmDeck);
        for (let seat = 1; seat <= aiSeats; seat++) {
          const rival = rivals[seat - 1] ? toWire(rivals[seat - 1]) : wire;
          const joined = await api.roomJoinTable(roomId, table.tableId, `AI ${seat}`, 'COMPUTER_MAD', 4, rival, password);
          if (!joined && rival !== wire) await api.roomJoinTable(roomId, table.tableId, `AI ${seat}`, 'COMPUTER_MAD', 4, wire, password);
        }
      }
      refreshTables();
      onOpenChange(false);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('tables.host')}
      description={t('tables.host.description')}
      width="lg"
      footer={
        <>
          <Button variant="quiet" onClick={() => onOpenChange(false)}>{t('tables.cancel')}</Button>
          <Button variant="decision" busy={busy} disabled={!deckId} onClick={() => void host()}>{t('tables.host.submit')}</Button>
        </>
      }
    >
      <div className={styles.form}>
        <Field label={t('tables.host.name')} value={name} placeholder={t('tables.host.defaultName', { name: userName })} onChange={(event) => setName(event.target.value)} />
        <div className={styles.pair}>
          <label className={styles.selectField}>
            <span>{t('tables.host.format')}</span>
            <select value={deckType} onChange={(event) => setDeckType(event.target.value)}>
              {!listed && <option value={deckType}>{formatRules(deckType).label}</option>}
              {menu.map((group) => (
                <optgroup key={group.label} label={group.label}>
                  {group.types.map((item) => <option key={item} value={item}>{formatRules(item).label}</option>)}
                </optgroup>
              ))}
            </select>
          </label>
          <label className={styles.selectField}>
            <span>{t('tables.host.game')}</span>
            <select value={chosenGame} onChange={(event) => setGameType(event.target.value)}>
              {gameTypes.map((item) => <option key={item} value={item}>{item}</option>)}
            </select>
          </label>
        </div>
        <div className={styles.pair}>
          {maxSeats > 2 ? (
            <label className={styles.selectField}>
              <span>{t('tables.host.players')}</span>
              <select value={seatCount} onChange={(event) => setSeats(Number(event.target.value))}>
                {Array.from({ length: maxSeats - minSeats + 1 }, (_, index) => minSeats + index).map((count) => <option key={count} value={count}>{count}</option>)}
              </select>
            </label>
          ) : (
            <label className={styles.selectField}>
              <span>{t('tables.host.wins')}</span>
              <select value={wins} onChange={(event) => setWins(Number(event.target.value))}>
                <option value={1}>{t('tables.host.wins.1')}</option>
                <option value={2}>{t('tables.host.wins.2')}</option>
                <option value={3}>{t('tables.host.wins.3')}</option>
              </select>
            </label>
          )}
          <label className={styles.selectField}>
            <span>{t('tables.host.ai')}</span>
            <select value={Math.min(aiSeats, seatCount - 1)} onChange={(event) => setAiSeats(Number(event.target.value))}>
              {Array.from({ length: seatCount }, (_, index) => <option key={index} value={index}>{index === 0 ? t('tables.host.none') : index}</option>)}
            </select>
          </label>
        </div>
        <Field label={t('tables.password')} type="password" value={password} onChange={(event) => setPassword(event.target.value)} hint={t('tables.host.passwordHint')} />
        <TableAdvancedFields
          value={advanced}
          onChange={(patch) => setAdvanced((current) => ({ ...current, ...patch }))}
          multiplayer={seatCount > 2}
          commander={commander}
        />
        {error && <p className={styles.error} role="alert">{error}</p>}
      </div>
    </Dialog>
  );
}
