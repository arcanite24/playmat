import { useQuery } from '@tanstack/react-query';
import { ChevronRight, Eye, Layers, Package, Shuffle, Swords, Trophy } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { eliminationRounds, fitPlayers, isCubeFromDeck, maxPoolSets, PACK_SOURCES, planEvent, playerCounts, TIMINGS, type EventChoice, type EventKind, type PackSource } from '../../core/events/eventPlan';
import type { ExpansionSetInfo, PlayerType, TableView, TimingOption } from '../../protocol/generated/views';
import type { WebTournamentOptions } from '../../protocol/options';
import { api } from '../connection';
import { toWire } from '../decks/deckModel';
import { registerMessages, t as translate, useT, type MessageKey } from '../i18n';
import messages from '../i18n/en/events';
import { useServerState, useTables } from '../queries';
import { useDecks } from '../stores/decks';
import { useDeckRoster } from '../stores/useDeckRoster';
import { useEvents } from '../stores/events';
import { useSession } from '../stores/session';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import { Field } from '../ui/Field';
import { Zone } from '../ui/Zone';
import styles from './EventsScreen.module.css';

registerMessages(messages);

const KINDS: { value: EventKind; icon: React.ReactNode }[] = [
  { value: 'draft', icon: <Layers size={22} /> },
  { value: 'sealed', icon: <Package size={22} /> },
  { value: 'jumpstart', icon: <Shuffle size={22} /> },
  { value: 'constructed', icon: <Swords size={22} /> },
];

/** The pack choices' words, per format (the plan in core/events keeps English ones for logs and tests). */
const SOURCE_TEXT: Record<Exclude<EventKind, 'constructed'>, Partial<Record<PackSource, [MessageKey, MessageKey]>>> = {
  draft: {
    set: ['events.source.set', 'events.source.set.draft'],
    cube: ['events.source.cube', 'events.source.cube.draft'],
    random: ['events.source.random', 'events.source.random.detail'],
    reshuffled: ['events.source.reshuffled', 'events.source.reshuffled.detail'],
    richMan: ['events.source.richMan', 'events.source.richMan.detail'],
    richManCube: ['events.source.richManCube', 'events.source.richManCube.detail'],
  },
  sealed: {
    set: ['events.source.set', 'events.source.set.sealed'],
    cube: ['events.source.cube', 'events.source.cube.sealed'],
  },
  jumpstart: {
    set: ['events.source.official', 'events.source.official.detail'],
    custom: ['events.source.custom', 'events.source.custom.detail'],
  },
};

const TIMING_TEXT: Partial<Record<TimingOption, MessageKey>> = {
  BEGINNER: 'events.timing.BEGINNER',
  REGULAR: 'events.timing.REGULAR',
  PROFESSIONAL: 'events.timing.PROFESSIONAL',
};

const EMPTY_DECK = { cards: [], sideboard: [] };

/** Drafts, sealed and constructed events: join one, follow yours, or host a new one. */
export function EventsScreen() {
  const { data: tables = [] } = useTables();
  const roomId = useSession((state) => state.roomId);
  const userName = useSession((state) => state.userName);
  const tournaments = useEvents((state) => state.tournaments);
  const mine = useMemo(() => Object.values(tournaments).filter((tournament) => !tournament.over), [tournaments]);
  const navigate = useNavigate();
  const t = useT();
  const roster = useDeckRoster();
  const [hosting, setHosting] = useState<EventKind | null>(null);
  const events = tables.filter((table) => table.isTournament && table.tableState !== 'FINISHED');
  const open = events.filter((table) => table.tableState === 'WAITING');
  const running = events.filter((table) => table.tableState !== 'WAITING');

  async function join(table: TableView) {
    if (!roomId || !table.tableId) return;
    // limited events need no deck to join; constructed events take the selected deck
    let deck = EMPTY_DECK as Parameters<typeof api.roomJoinTournament>[5];
    if (!table.limited) {
      const selected = roster.find((deck) => deck.id === useDecks.getState().selectedId)?.id ?? roster[0]?.id;
      if (!selected) {
        notify(translate('events.pickDeckFirst'), translate('events.pickDeckFirst.detail'), 'error');
        return;
      }
      deck = toWire((await useDecks.getState().loadForPlay(selected)).deck);
    }
    const ok = await api.roomJoinTournament(roomId, table.tableId, userName, 'HUMAN', 1, deck).catch((error) => {
      notify(translate('events.joinFailed'), error instanceof Error ? error.message : String(error), 'error');
      return null;
    });
    if (ok === false) notify(translate('events.joinFailed'), translate('events.seatRefused'), 'error');
    else if (ok) notify(translate('events.joined'), translate('events.joined.detail', { name: table.tableName ?? '' }));
  }

  return (
    <div className={styles.page}>
      <header className={styles.head}>
        <h1 className={styles.title}>{t('events.title')}</h1>
        <Button variant="print" icon={<Trophy size={18} />} onClick={() => setHosting('draft')}>{t('events.host')}</Button>
      </header>

      {mine.length > 0 && (
        <Zone label={t('events.yours')} tone="filled">
          <div className={styles.list}>
            {mine.map((tournament) => (
              <button key={tournament.tournamentId} type="button" className={styles.mine} onClick={() => navigate(`/event/${tournament.tournamentId}`)}>
                <Trophy size={18} aria-hidden="true" />
                <span>{tournament.view?.tournamentName ?? t('events.event')}</span>
                <small>{tournament.view?.runningInfo || tournament.view?.tournamentState || t('events.open')}</small>
              </button>
            ))}
          </div>
        </Zone>
      )}

      {open.length === 0 && running.length === 0 ? (
        <Zone label={t('events.startOne')} className={styles.starter}>
          <p className={styles.empty}>{t('events.nothing')}</p>
          <ul className={styles.starts}>
            {KINDS.map((option) => (
              <li key={option.value}>
                <button type="button" className={styles.start} onClick={() => setHosting(option.value)}>
                  <strong>{t(`events.kind.${option.value}`)}</strong>
                  <span>{t(`events.kind.${option.value}.detail`)}</span>
                  <ChevronRight size={20} aria-hidden="true" />
                </button>
              </li>
            ))}
          </ul>
        </Zone>
      ) : (
      <div className={styles.columns}>
        <Zone label={t('events.openToJoin')} className={styles.zone}>
          {open.length === 0 ? (
            <p className={styles.empty}>{t('events.noneWaiting')}</p>
          ) : (
            <div className={styles.list}>
              {open.map((table) => (
                <EventRow key={table.tableId} table={table} action={<Button size="sm" variant="print" onClick={() => void join(table)}>{t('events.join')}</Button>} />
              ))}
            </div>
          )}
        </Zone>
        <Zone label={t('events.underWay')} className={styles.zone}>
          {running.length === 0 ? (
            <p className={styles.empty}>{t('events.noneRunning')}</p>
          ) : (
            <div className={styles.list}>
              {running.map((table) => (
                <EventRow
                  key={table.tableId}
                  table={table}
                  action={<Button size="sm" variant="quiet" icon={<Eye size={15} />} onClick={() => table.tableId && api.roomWatchTournament(table.tableId).catch(() => undefined)}>{t('events.view')}</Button>}
                />
              ))}
            </div>
          )}
        </Zone>
      </div>
      )}

      {/* remounted per opening so it starts from the chosen format */}
      <HostDialog key={hosting ?? 'closed'} open={!!hosting} initialKind={hosting ?? 'draft'} onOpenChange={(open) => !open && setHosting(null)} />
    </div>
  );
}

function EventRow({ table, action }: { table: TableView; action: React.ReactNode }) {
  return (
    <article className={styles.row}>
      <div className={styles.rowMain}>
        <h3 className={styles.rowName}>{table.tableName}</h3>
        <p className={styles.rowMeta}>{table.gameType} · {table.additionalInfoShort || table.deckType}</p>
      </div>
      <span className={styles.rowState}>{table.seatsInfo}</span>
      <span className={styles.rowState}>{table.tableStateText}</span>
      {action}
    </article>
  );
}

function HostDialog({ open, initialKind, onOpenChange }: { open: boolean; initialKind: EventKind; onOpenChange(open: boolean): void }) {
  const roomId = useSession((state) => state.roomId);
  const userName = useSession((state) => state.userName);
  const t = useT();
  const server = useServerState();
  const sets = useQuery({
    queryKey: ['expansionSets'],
    queryFn: () => api.getExpansionSets(),
    staleTime: Infinity,
    enabled: open,
  });
  const boosterSets = useMemo(
    () => (sets.data ?? []).filter((set) => set.hasBoosters).sort((a, b) => (b.releaseDate ?? 0) - (a.releaseDate ?? 0)),
    [sets.data],
  );
  const decks = useDecks();
  const roster = useDeckRoster();
  const cubes = useMemo(() => [...(server.data?.draftCubes ?? [])].sort((a, b) => a.localeCompare(b)), [server.data?.draftCubes]);
  const types = useMemo(() => server.data?.tournamentTypes ?? [], [server.data?.tournamentTypes]);

  const [kind, setKind] = useState<EventKind>(initialKind);
  const [source, setSource] = useState<PackSource>('set');
  const [name, setName] = useState('');
  const [setCode, setSetCode] = useState('');
  const [pool, setPool] = useState<string[] | null>(null);
  const [cubeName, setCubeName] = useState('');
  const [cubeDeckId, setCubeDeckId] = useState<string>('');
  const [jumpstartPacks, setJumpstartPacks] = useState<{ name: string; text: string } | null>(null);
  const [seats, setSeats] = useState(initialKind === 'draft' ? 8 : 4);
  const [fillAi, setFillAi] = useState(true);
  const [swiss, setSwiss] = useState(true);
  const [rounds, setRounds] = useState(3);
  const [bestOf3, setBestOf3] = useState(false);
  const [constructionMinutes, setConstructionMinutes] = useState(20);
  const [timing, setTiming] = useState<TimingOption>('REGULAR');
  const [format, setFormat] = useState('Constructed - Standard');
  const [deckId, setDeckId] = useState<string | null>(decks.selectedId);
  const [busy, setBusy] = useState(false);

  const chosenDeckId = roster.find((deck) => deck.id === deckId)?.id ?? roster[0]?.id ?? null;
  const chosenSet = setCode || boosterSets[0]?.setCode || '';
  // the six newest sets until the host picks a pool
  const chosenPool = pool ?? boosterSets.slice(0, 6).map((set) => set.setCode ?? '');
  const chosenCube = cubeName || cubes.find((cube) => !isCubeFromDeck(cube)) || '';
  const formats = (server.data?.deckTypes ?? []).filter((type) => type.startsWith('Constructed'));
  const limited = kind !== 'constructed';
  const sources = kind === 'constructed' ? [] : PACK_SOURCES[kind];
  const usesCube = source === 'cube' || source === 'richManCube';
  const usesPool = source === 'random' || source === 'reshuffled' || source === 'richMan';
  // custom jumpstart only comes as single elimination
  const structureFixed = kind === 'jumpstart' && source === 'custom';
  // only the player counts the event type takes (a draft needs at least 4), keeping the host's pick when it fits
  const swissChosen = swiss && !structureFixed;
  const counts = playerCounts(kind, swissChosen, source, types);
  const players = fitPlayers(seats, counts);

  const choice: EventChoice = {
    kind,
    swiss: swissChosen,
    source,
    players,
    setCode: chosenSet,
    pool: chosenPool,
    cubeName: chosenCube,
    cubeFromDeck: cubeDeckId ? { pending: cubeDeckId } : undefined,
    jumpstartPacks: jumpstartPacks?.text,
    constructionMinutes,
    timing,
  };
  const check = sets.isPending || server.isPending ? 'Loading…' : planEvent(choice, types);
  const problem = typeof check === 'string' ? check : null;

  function chooseKind(next: EventKind) {
    setKind(next);
    setSource('set');
    setSeats(next === 'draft' ? 8 : 4);
  }

  async function readPacks(file: File | undefined) {
    if (!file) return;
    if (file.size > 300_000) {
      notify(translate('events.fileTooBig'), translate('events.fileTooBig.detail'), 'error');
      return;
    }
    setJumpstartPacks({ name: file.name, text: await file.text() });
  }

  async function create() {
    if (!roomId) return;
    setBusy(true);
    try {
      const cubeFromDeck = usesCube && isCubeFromDeck(chosenCube) && cubeDeckId
        ? toWire((await useDecks.getState().loadForPlay(cubeDeckId)).deck)
        : undefined;
      const plan = planEvent({ ...choice, cubeFromDeck }, types);
      if (typeof plan === 'string') throw new Error(plan);
      // the server's draft bot only drafts and concedes every game; this AI drafts, builds and plays
      const aiType: PlayerType = 'COMPUTER_MAD';
      const playerTypes: PlayerType[] = ['HUMAN', ...Array.from({ length: players - 1 }, () => (fillAi ? aiType : 'HUMAN' as PlayerType))];
      const options: WebTournamentOptions = {
        name: name.trim() || translate(`events.defaultName.${kind}`, { name: userName }),
        tournamentType: plan.tournamentType,
        numberRounds: choice.swiss ? rounds : 0,
        watchingAllowed: true,
        playerTypes,
        matchOptions: {
          gameType: 'Two Player Duel',
          deckType: limited ? 'Limited' : format,
          winsNeeded: bestOf3 ? 2 : 1,
          limited,
          spectatorsAllowed: true,
          skillLevel: 'CASUAL',
        },
        limitedOptions: plan.limitedOptions,
      };
      const table = await api.roomCreateTournament(roomId, options);
      const tableId = table?.tableId;
      if (!tableId) throw new Error(translate('events.notCreated'));

      let deck = EMPTY_DECK as Parameters<typeof api.roomJoinTournament>[5];
      if (!limited) {
        if (!chosenDeckId) throw new Error(translate('events.pickDeckToPlay'));
        deck = toWire((await useDecks.getState().loadForPlay(chosenDeckId)).deck);
      }
      if (!await api.roomJoinTournament(roomId, tableId, userName, 'HUMAN', 1, deck)) throw new Error(translate('events.yourSeatRefused'));
      if (fillAi) {
        for (let seat = 1; seat < players; seat++) {
          const joined = await api.roomJoinTournament(roomId, tableId, `Bot ${seat}`, playerTypes[seat], 4, deck);
          if (!joined) throw new Error(translate('events.aiSeatRefused'));
        }
        if (!await api.tournamentStart(roomId, tableId)) throw new Error(translate('events.couldNotStart'));
      }
      notify(translate(fillAi ? 'events.starting' : 'events.opened'), translate(fillAi ? 'events.starting.detail' : 'events.opened.detail'));
      onOpenChange(false);
    } catch (error) {
      notify(translate('events.hostFailed'), error instanceof Error ? error.message : String(error), 'error');
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('events.host')}
      width="lg"
      footer={(
        <>
          {problem && problem !== 'Loading…' && <p className={styles.problem} role="status">{problem}</p>}
          <Button variant="quiet" onClick={() => onOpenChange(false)}>{t('events.cancel')}</Button>
          <Button variant="decision" busy={busy} disabled={!!problem || (!limited && !chosenDeckId)} onClick={() => void create()}>
            {fillAi ? t('events.start') : t('events.openEvent')}
          </Button>
        </>
      )}
    >
      <div className={styles.form}>
        <div className={styles.kinds} role="radiogroup" aria-label={t('events.type')}>
          {KINDS.map((option) => (
            <button
              key={option.value}
              type="button"
              role="radio"
              aria-checked={kind === option.value}
              className={[styles.kind, kind === option.value ? styles.kindOn : ''].join(' ')}
              onClick={() => chooseKind(option.value)}
            >
              {option.icon}
              <strong>{t(`events.kind.${option.value}`)}</strong>
              <span>{t(`events.kind.${option.value}.detail`)}</span>
            </button>
          ))}
        </div>

        {sources.length > 1 && (
          <div className={styles.sources} role="radiogroup" aria-label={t('events.packs')}>
            {sources.map((option) => {
              const text = kind === 'constructed' ? undefined : SOURCE_TEXT[kind][option.value];
              return (
                <label key={option.value} className={[styles.source, source === option.value ? styles.sourceOn : ''].join(' ')}>
                  <input type="radio" name="source" value={option.value} checked={source === option.value} onChange={() => setSource(option.value)} />
                  <strong>{text ? t(text[0]) : option.label}</strong>
                  <span>{text ? t(text[1]) : option.detail}</span>
                </label>
              );
            })}
          </div>
        )}

        <div className={styles.grid}>
          <Field label={t('events.name')} value={name} onChange={(event) => setName(event.target.value)} placeholder={t(`events.defaultName.${kind}`, { name: userName })} maxLength={40} />
          {limited && kind !== 'jumpstart' && source === 'set' && (
            <label className={styles.control}>
              <span>{t('events.set')}</span>
              <select value={chosenSet} onChange={(event) => setSetCode(event.target.value)} disabled={sets.isPending}>
                {boosterSets.map((set) => <option key={set.setCode} value={set.setCode}>{set.name} ({set.setCode})</option>)}
              </select>
            </label>
          )}
          {usesCube && (
            <label className={styles.control}>
              <span>{t('events.cube')}</span>
              <select value={chosenCube} onChange={(event) => setCubeName(event.target.value)}>
                {cubes.map((cube) => <option key={cube} value={cube}>{isCubeFromDeck(cube) ? t('events.ownCube') : cube}</option>)}
              </select>
            </label>
          )}
          {usesCube && isCubeFromDeck(chosenCube) && (
            <label className={styles.control}>
              <span>{t('events.cubeList')}</span>
              <select value={cubeDeckId} onChange={(event) => setCubeDeckId(event.target.value)}>
                <option value="">{t('events.pickDeck')}</option>
                {roster.map((deck) => <option key={deck.id} value={deck.id}>{deck.name}</option>)}
              </select>
            </label>
          )}
          {!limited && (
            <>
              <label className={styles.control}>
                <span>{t('events.format')}</span>
                <select value={format} onChange={(event) => setFormat(event.target.value)}>
                  {(formats.length ? formats : [format]).map((type) => <option key={type} value={type}>{type.replace(/^Constructed - /, '')}</option>)}
                </select>
              </label>
              <label className={styles.control}>
                <span>{t('events.yourDeck')}</span>
                <select value={chosenDeckId ?? ''} onChange={(event) => setDeckId(event.target.value || null)}>
                  {roster.map((deck) => <option key={deck.id} value={deck.id}>{deck.name}</option>)}
                </select>
              </label>
            </>
          )}
          <label className={styles.control}>
            <span>{t('events.players')}</span>
            <select value={players} onChange={(event) => setSeats(Number(event.target.value))}>
              {(counts.length ? counts : [players]).map((count) => <option key={count} value={count}>{count}</option>)}
            </select>
          </label>
          {!structureFixed && (
            <label className={styles.control}>
              <span>{t('events.structure')}</span>
              <select value={swiss ? 'swiss' : 'elimination'} onChange={(event) => setSwiss(event.target.value === 'swiss')}>
                <option value="swiss">{t('events.swiss')}</option>
                <option value="elimination">{t('events.elimination')}</option>
              </select>
            </label>
          )}
          {choice.swiss ? (
            <label className={styles.control}>
              <span>{t('events.rounds')}</span>
              <select value={rounds} onChange={(event) => setRounds(Number(event.target.value))}>
                {[1, 2, 3, 4, 5].map((count) => <option key={count} value={count}>{count}</option>)}
              </select>
            </label>
          ) : (
            // elimination plays until one player is left: the round count follows from the players
            <div className={styles.control}>
              <span>{t('events.rounds')}</span>
              <output>{t('events.rounds.elimination', { count: eliminationRounds(players) })}</output>
            </div>
          )}
          <label className={styles.control}>
            <span>{t('events.matches')}</span>
            <select value={bestOf3 ? '3' : '1'} onChange={(event) => setBestOf3(event.target.value === '3')}>
              <option value="1">{t('events.bestOf1')}</option>
              <option value="3">{t('events.bestOf3')}</option>
            </select>
          </label>
          {kind === 'draft' && (
            <label className={styles.control}>
              <span>{t('events.pickTimer')}</span>
              <select value={timing} onChange={(event) => setTiming(event.target.value as TimingOption)}>
                {TIMINGS.map((option) => <option key={option.value} value={option.value}>{TIMING_TEXT[option.value] ? t(TIMING_TEXT[option.value]!) : option.label}</option>)}
              </select>
            </label>
          )}
          {limited && (
            <label className={styles.control}>
              <span>{t('events.deckBuilding')}</span>
              <select value={constructionMinutes} onChange={(event) => setConstructionMinutes(Number(event.target.value))}>
                {[10, 15, 20, 30, 45, 60].map((minutes) => <option key={minutes} value={minutes}>{t('events.minutes', { count: minutes })}</option>)}
              </select>
            </label>
          )}
        </div>

        {usesPool && (
          <SetPool
            sets={boosterSets}
            pool={chosenPool}
            onChange={setPool}
            limit={source === 'reshuffled' ? undefined : maxPoolSets(source, players)}
          />
        )}

        {kind === 'jumpstart' && source === 'custom' && (
          <label className={styles.upload}>
            <span>{t('events.packsFile')}</span>
            <input type="file" accept=".txt,.dck,text/plain" onChange={(event) => void readPacks(event.target.files?.[0])} />
            <small>{jumpstartPacks ? t('events.packsFile.chosen', { name: jumpstartPacks.name, count: jumpstartPacks.text.length }) : t('events.packsFile.hint')}</small>
          </label>
        )}

        {usesCube && isCubeFromDeck(chosenCube) && (
          <p className={styles.hint}>{t('events.cubeHint')}</p>
        )}

        <label className={styles.check}>
          <input type="checkbox" checked={fillAi} onChange={(event) => setFillAi(event.target.checked)} />
          <span>{t('events.fillAi')}</span>
        </label>
      </div>
    </Dialog>
  );
}

/** The sets random, reshuffled and rich man drafts open packs from. */
function SetPool({ sets, pool, onChange, limit }: {
  sets: ExpansionSetInfo[];
  pool: string[];
  onChange(pool: string[]): void;
  limit?: number;
}) {
  const t = useT();
  const [filter, setFilter] = useState('');
  const chosen = new Set(pool);
  const needle = filter.trim().toLowerCase();
  const shown = sets.filter((set) => !needle || set.name?.toLowerCase().includes(needle) || set.setCode?.toLowerCase().includes(needle));
  function toggle(code: string) {
    onChange(chosen.has(code) ? pool.filter((entry) => entry !== code) : [...pool, code]);
  }
  return (
    <fieldset className={styles.pool}>
      <legend>{limit && pool.length > limit ? t('events.pool.limit', { count: pool.length, limit }) : t('events.pool', { count: pool.length })}</legend>
      <div className={styles.poolTools}>
        <Field label={t('events.findSet')} value={filter} onChange={(event) => setFilter(event.target.value)} />
        <Button size="sm" variant="quiet" onClick={() => onChange([])}>{t('events.clear')}</Button>
      </div>
      <ul className={styles.poolList}>
        {shown.map((set) => (
          <li key={set.setCode}>
            <label>
              <input type="checkbox" checked={chosen.has(set.setCode ?? '')} onChange={() => toggle(set.setCode ?? '')} />
              {set.name} <small>{set.setCode}</small>
            </label>
          </li>
        ))}
      </ul>
    </fieldset>
  );
}
