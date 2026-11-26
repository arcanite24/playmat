import { Package, Plus, Swords, Trophy } from 'lucide-react';
import { useState } from 'react';
import { Link, Navigate, useNavigate, useSearchParams } from 'react-router-dom';
import type { LeagueDeck, LeagueFormat, LeaguePack, LeagueSet, LeagueState, LadderState } from '../../protocol/generated/views';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import league from '../i18n/en/league';
import { usePlay } from '../stores/play';
import { Button } from '../ui/Button';
import { useCareerState } from './careerData';
import { GameResult, OpponentCard, Recent, Standing } from './CareerRankedScreen';
import { PackReveal } from './PackReveal';
import { Pips } from './Portraits';
import { leagueFormat, startReady, toggleStarter } from './leagueModel';
import { openLeaguePack, playLeague, startLeague, useLeagueDecks, useLeagueSets, useLeagueState } from './leagueData';
import { useRankedOpponent, useRankedState } from './rankedData';
import { titleWords } from './rankedModel';
import { SeasonBoards } from './SeasonBoards';
import { Versus } from './Versus';
import modes from './CareerModes.module.css';
import ranked from './Ranked.module.css';
import styles from './League.module.css';

registerMessages(messages);
registerMessages(league);

/**
 * The Career League (#433): Ranked with the cards you own. Start a format with two of its starter decks and its packs,
 * climb its own ladder with League decks, and earn the format's packs and wildcards on the way up.
 */
export function CareerLeagueScreen() {
  const t = useT();
  const career = useCareerState();
  const [params, setParams] = useSearchParams();
  const state = useLeagueState(!!career.data?.enabled);
  const formats = state.data?.formats ?? [];
  const format = leagueFormat(formats, params.get('format'));
  const tableId = params.get('table');
  const current = formats.find((item) => item.format === format);

  if (career.isPending) return <p className={modes.note}>{t('career.loading')}</p>;
  if (!career.data?.enabled) return <Navigate to="/career" replace />;
  return (
    <div className={modes.page}>
      <div className={`${modes.scroll} ${modes.capped}`}>
        <header className={modes.modeHead}>
          <div>
            <p className={modes.kicker}><Trophy size={16} aria-hidden="true" /> {t('league.kicker')}</p>
            <h1 className={modes.modeTitle}>{t('league.title')}</h1>
            <p className={modes.lead}>{t('league.lead')}</p>
          </div>
          {formats.length > 1 && (
            <div className={ranked.formats} role="tablist" aria-label={t('career.ranked.formats')}>
              {formats.map((item) => (
                <button
                  key={item.format}
                  type="button"
                  role="tab"
                  aria-selected={item.format === format}
                  className={[ranked.format, item.format === format ? ranked.formatOn : ''].join(' ')}
                  onClick={() => setParams({ format: item.format ?? '' })}
                  data-nav
                >
                  {formatName(t, item.format)}
                </button>
              ))}
            </div>
          )}
        </header>
        {state.isPending && <p className={modes.note}>{t('career.loading')}</p>}
        {state.isError && <p className={modes.error} role="alert">{t('career.failed')}</p>}
        {state.data && formats.length === 0 && <p className={modes.note}>{t('league.none')}</p>}
        {state.data && current && !current.started && <StartLeague key={format} league={current} state={state.data} />}
        {state.data && current?.started && (
          <>
            {tableId && <GameResult tableId={tableId} />}
            <Started key={format} format={format} league={current} />
          </>
        )}
      </div>
    </div>
  );
}

type Translate = ReturnType<typeof useT>;

function formatName(t: Translate, format: string | undefined): string {
  return t(`career.ranked.format.${format === 'pioneer' ? 'pioneer' : 'standard'}`);
}

function StartLeague({ league, state }: { league: LeagueFormat; state: LeagueState }) {
  const t = useT();
  const [picked, setPicked] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const format = league.format ?? '';
  const wildcards = state.startWildcards;

  async function start() {
    setBusy(true);
    setError(null);
    try {
      await startLeague(format, picked);
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : String(failure));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className={styles.start} aria-labelledby="league-start">
      <h2 id="league-start" className={modes.subhead}>{t('league.start.title', { format: formatName(t, format) })}</h2>
      <p className={modes.small}>{t('league.start.lead')}</p>
      <ul className={styles.starters} aria-label={t('league.start.pick')}>
        {(league.starters ?? []).map((starter) => {
          const on = picked.includes(starter.id ?? '');
          return (
            <li key={starter.id}>
              <button
                type="button"
                className={[styles.starter, on ? styles.starterOn : ''].join(' ')}
                aria-pressed={on}
                onClick={() => setPicked((current) => toggleStarter(current, starter.id ?? ''))}
                data-nav
              >
                <Pips colors={starter.colors} size="sm" />
                <b>{starter.name}</b>
                <small className={modes.small}>{t('league.start.cards', { count: starter.cards ?? 0 })}</small>
              </button>
            </li>
          );
        })}
      </ul>
      <ul className={modes.lines} aria-label={t('league.start.gives')}>
        <li>{t('league.start.decks')}</li>
        <li>{t('league.start.packs', { count: league.startPacks ?? 0, format: formatName(t, format) })}</li>
        {state.first && wildcards && (
          <li>{t('league.start.wildcards', { common: wildcards.common ?? 0, uncommon: wildcards.uncommon ?? 0, rare: wildcards.rare ?? 0 })}</li>
        )}
      </ul>
      {error && <p className={modes.error} role="alert">{error}</p>}
      <div className={modes.actions}>
        <Button variant="decision" size="lg" disabled={!startReady(picked) || busy} onClick={() => void start()} data-nav>
          {t(startReady(picked) ? 'league.start.go' : 'league.start.pickTwo')}
        </Button>
      </div>
    </section>
  );
}

function Started({ format, league }: { format: string; league: LeagueFormat }) {
  const state = useRankedState(format, true, 'league');
  const t = useT();
  return (
    <>
      {state.isPending && <p className={modes.note}>{t('career.loading')}</p>}
      {state.isError && <p className={modes.error} role="alert">{t('career.failed')}</p>}
      {state.data && (
        <div className={modes.split}>
          <div className={ranked.main}>
            <Standing state={state.data} />
            <Packs format={format} league={league} />
            <Decks format={format} />
            <Recent state={state.data} />
            <SeasonBoards state={state.data} />
          </div>
          <Queue format={format} state={state.data} />
        </div>
      )}
    </>
  );
}

function Queue({ format, state }: { format: string; state: LadderState }) {
  const t = useT();
  const opponent = useRankedOpponent(format, true, 'league');
  const decks = useLeagueDecks(format);
  const busy = usePlay((play) => play.phase !== 'idle');
  const [deckId, setDeckId] = useState<string | null>(null);
  const [versus, setVersus] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const list = decks.data ?? [];
  const deck = list.find((item) => item.id === deckId) ?? list.find(playable) ?? null;
  const them = opponent.data;
  const ready = !!deck && playable(deck);

  return (
    <aside className={ranked.queue} aria-labelledby="league-next">
      <h2 id="league-next" className={modes.subhead}>{t('career.ranked.next')}</h2>
      {opponent.isPending && <p className={modes.note}>{t('career.loading')}</p>}
      {opponent.isError && <p className={modes.error} role="alert">{t('career.failed')}</p>}
      {them && <OpponentCard opponent={them} />}
      <label className={ranked.field}>
        <span>{t('league.deck')}</span>
        <select value={deck?.id ?? ''} onChange={(event) => setDeckId(event.target.value)} disabled={decks.isPending} data-nav>
          <option value="" disabled>{decks.isPending ? t('career.loading') : t('career.ranked.chooseDeck')}</option>
          {list.map((item) => (
            <option key={item.id} value={item.id}>{playable(item) ? item.name : t('league.deck.notReady', { name: item.name ?? '' })}</option>
          ))}
        </select>
      </label>
      {deck && !playable(deck) && <DeckProblems deck={deck} />}
      {error && <p className={modes.error} role="alert">{error}</p>}
      <div className={modes.actions}>
        <Button variant="decision" size="lg" icon={<Swords size={18} />} disabled={!ready || !them || busy || !!state.playing} onClick={() => setVersus(true)} data-nav>
          {t('league.play')}
        </Button>
      </div>
      {state.playing && <p className={modes.small}>{t('career.ranked.playingNow')}</p>}
      <ul className={modes.lines} aria-label={t('career.ranked.rules')}>
        <li>{t('league.rule.owned')}</li>
        <li>{t('career.ranked.ruleNoAssists')}</li>
        <li>{t('league.rule.pays')}</li>
      </ul>
      {versus && them && deck && (
        <Versus
          kicker={`${t('league.title')} · ${state.label ?? ''}`}
          opponent={{
            name: them.name ?? '',
            avatar: them.avatar,
            tagline: them.title ? titleWords(them.title) : undefined,
          }}
          deck={{ name: deck.name ?? '' }}
          stakes={[t('career.ranked.stakeWin'), t('league.stakeTier')]}
          onFight={async () => {
            setError(null);
            try {
              await playLeague(format, deck.id ?? '');
            } catch (failure) {
              setError(failure instanceof Error ? failure.message : String(failure));
              throw failure;
            }
          }}
          onClose={() => setVersus(false)}
        />
      )}
    </aside>
  );
}

function playable(deck: LeagueDeck): boolean {
  return (deck.missing ?? []).length === 0 && (deck.illegal ?? []).length === 0;
}

function DeckProblems({ deck }: { deck: LeagueDeck }) {
  const t = useT();
  const missing = deck.missing ?? [];
  const illegal = deck.illegal ?? [];
  return (
    <div className={styles.problems}>
      {missing.length > 0 && <p className={modes.small}>{t('league.deck.missing', { count: missing.length })}: {missing.slice(0, 3).join('; ')}</p>}
      {illegal.length > 0 && <p className={modes.small}>{t('league.deck.illegal')}: {illegal.join('; ')}</p>}
    </div>
  );
}

function Decks({ format }: { format: string }) {
  const t = useT();
  const navigate = useNavigate();
  const decks = useLeagueDecks(format);
  const list = decks.data ?? [];
  return (
    <section aria-labelledby="league-decks">
      <h2 id="league-decks" className={modes.subhead}>{t('league.decks')}</h2>
      <ul className={styles.decks}>
        {list.map((deck) => (
          <li key={deck.id}>
            <Link to={`/career/league/deck/${encodeURIComponent(deck.id ?? '')}`} aria-label={t('league.decks.edit', { name: deck.name ?? '' })} data-nav>
              <b>{deck.name}</b>
            </Link>
            <small className={modes.small}>
              {playable(deck) ? t('league.deck.ready', { count: deck.cards ?? 0 }) : t('league.deck.missing', { count: (deck.missing ?? []).length || (deck.illegal ?? []).length })}
            </small>
          </li>
        ))}
      </ul>
      <div className={styles.deckActions}>
        <Button variant="quiet" size="sm" icon={<Plus size={14} />} onClick={() => navigate(`/career/league/deck/new?format=${format}`)} data-nav>
          {t('league.decks.new')}
        </Button>
      </div>
    </section>
  );
}

function Packs({ format, league }: { format: string; league: LeagueFormat }) {
  const t = useT();
  const sets = useLeagueSets(format);
  const state = useLeagueState();
  const [setCode, setSetCode] = useState<string | null>(null);
  const [opening, setOpening] = useState(false);
  const [opened, setOpened] = useState<{ pack: LeaguePack; set: LeagueSet; serial: number } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const list = sets.data ?? [];
  const set = list.find((item) => item.setCode === setCode) ?? list[0] ?? null;
  const tokens = league.tokens ?? 0;
  const careerTokens = state.data?.packTokens ?? 0;
  const coins = state.data?.coins ?? 0;
  const price = state.data?.packPrice ?? 100;
  const canOpen = tokens > 0 || careerTokens > 0 || coins >= price;

  async function open(which: LeagueSet) {
    setOpening(true);
    setError(null);
    try {
      const pack = await openLeaguePack(format, which.setCode ?? '');
      setOpened({ pack, set: which, serial: Date.now() });
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : String(failure));
    } finally {
      setOpening(false);
    }
  }

  return (
    <section className={styles.packs} aria-labelledby="league-packs">
      <h2 id="league-packs" className={modes.subhead}><Package size={16} aria-hidden="true" /> {t('league.packs')}</h2>
      <p className={modes.small}>
        {tokens > 0
          ? t('league.packs.tokens', { count: tokens, format: formatName(t, format) })
          : careerTokens > 0 ? t('league.packs.career', { count: careerTokens }) : t('league.packs.coins', { price, coins })}
      </p>
      <div className={styles.packRow}>
        <label className={ranked.field}>
          <span>{t('league.packs.set')}</span>
          <select value={set?.setCode ?? ''} onChange={(event) => setSetCode(event.target.value)} disabled={sets.isPending} data-nav>
            {list.map((item) => <option key={item.setCode} value={item.setCode}>{item.setCode === 'mixed' ? t('league.packs.mixed', { format: formatName(t, format) }) : item.name}</option>)}
          </select>
        </label>
        <Button variant="decision" disabled={!set || !canOpen || opening} onClick={() => set && void open(set)} data-nav>
          {t('league.packs.open')}
        </Button>
      </div>
      {error && <p className={modes.error} role="alert">{error}</p>}
      {opened && (
        <PackReveal
          key={opened.serial}
          setName={opened.set.setCode === 'mixed' ? opened.pack.setCode ?? '' : opened.set.name ?? ''}
          setCode={opened.pack.setCode ?? ''}
          cover={opened.set.cover}
          cards={opened.pack.cards ?? []}
          bonus={opened.pack.wildcard === 'mythic' ? t('league.packs.mythic') : opened.pack.wildcard === 'rare' ? t('league.packs.rare') : undefined}
          canBuyAnother={(opened.pack.tokens ?? 0) > 0 && !opening}
          onAnother={() => void open(opened.set)}
          onClose={() => setOpened(null)}
        />
      )}
    </section>
  );
}
