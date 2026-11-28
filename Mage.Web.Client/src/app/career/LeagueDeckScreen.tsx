import { Hammer } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams, useSearchParams } from 'react-router-dom';
import type { DeckCardLists } from '../../core/decks/types';
import type { LeagueDeck } from '../../protocol/generated/views';
import { api } from '../connection';
import { Collection } from '../decks/Collection';
import { DeckBuilder, type BinderProps, type BuilderMode } from '../decks/DeckBuilderScreen';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import league from '../i18n/en/league';
import { Button } from '../ui/Button';
import { OwnedBinder } from './CareerDeckScreen';
import { useCareerCollection } from './careerData';
import { isBasic, ownedByName } from './careerModel';
import { craftMissing, saveLeagueDeck, toBuilderDeck, useLeagueBuild } from './leagueBuild';
import { missingCards, PLAYSET } from './leagueModel';
import { leaguePath } from './leagueData';
import styles from './League.module.css';

registerMessages(messages);
registerMessages(league);

/** missing cards listed by name in the banner */
const SHOWN = 6;


/**
 * A League deck in the deck builder (#433): any card legal in the format, up to a playset, owned or not. The banner
 * says what the deck lacks and what crafting it takes, and crafts it all at once.
 */
export function LeagueDeckScreen() {
  const t = useT();
  const navigate = useNavigate();
  const { deckId = '' } = useParams();
  const [params] = useSearchParams();
  const [view, setView] = useState<LeagueDeck | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    useLeagueBuild.setState({ view: null, error: null });
    (async () => {
      try {
        if (deckId === 'new') {
          const rawFormat = params.get('format');
          const format =
            rawFormat === 'pioneer' || rawFormat === 'commander' || rawFormat === 'pauper' || rawFormat === 'brawl'
              ? rawFormat
              : 'standard';
          const made = await api.leagueSaveDeck(null, t('league.deck.newName'), format, { cards: [], sideboard: [] } as never);
          if (!cancelled) navigate(`/career/league/deck/${encodeURIComponent(made.id ?? '')}`, { replace: true });
          return;
        }
        const decks = await api.leagueDecks(null);
        const found = decks.find((deck) => deck.id === deckId) ?? null;
        if (cancelled) return;
        if (!found) setError(t('league.deck.gone'));
        setView(found);
        useLeagueBuild.setState({ view: found });
      } catch (failure) {
        if (!cancelled) setError(failure instanceof Error ? failure.message : String(failure));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [deckId, params, navigate, t]);

  const format = view?.format ?? 'standard';
  const id = view?.id ?? '';
  const limit = format === 'commander' || format === 'brawl' ? 1 : PLAYSET;
  const mode = useMemo<BuilderMode>(() => ({
    Binder: LeagueBinder,
    allowance: (name) => (isBasic(name) ? Infinity : limit),
    save: (deck) => void saveLeagueDeck(id, format, deck),
    doneLabel: t('league.title'),
    onDone: () => navigate(leaguePath(format)),
    onPlay: () => navigate(leaguePath(format)),
    Banner: CraftBanner,
  }), [id, format, limit, navigate, t]);
  const initial = useMemo<DeckCardLists | null>(() => (view ? toBuilderDeck(view) : null), [view]);

  if (error) return <p className={styles.screenNote} role="alert">{error}</p>;
  if (!initial) return <p className={styles.screenNote}>{t('career.loading')}</p>;
  return <DeckBuilder key={id} initial={initial} mode={mode} />;
}

/** owned cards, or every card legal in the format to plan with */
function LeagueBinder(props: BinderProps) {
  const t = useT();
  const [all, setAll] = useState(false);
  return (
    <div className={styles.binder}>
      <div className={styles.binderSwitch} role="group" aria-label={t('league.binder.show')}>
        <button type="button" aria-pressed={!all} className={!all ? styles.switchOn : styles.switchOff} onClick={() => setAll(false)}>
          {t('league.binder.owned')}
        </button>
        <button type="button" aria-pressed={all} className={all ? styles.switchOn : styles.switchOff} onClick={() => setAll(true)}>
          {t('league.binder.all')}
        </button>
      </div>
      {all ? <Collection {...props} /> : <OwnedBinder {...props} />}
    </div>
  );
}

/** what the deck on screen lacks against the collection, and what the server says crafting it takes */
function CraftBanner({ deck }: { deck: DeckCardLists }) {
  const t = useT();
  const view = useLeagueBuild((state) => state.view);
  const error = useLeagueBuild((state) => state.error);
  const crafting = useLeagueBuild((state) => state.crafting);
  const collection = useCareerCollection(true);
  const owned = useMemo(() => ownedByName(collection.data ?? []), [collection.data]);
  const copyLimit = view?.format === 'commander' || view?.format === 'brawl' ? 1 : PLAYSET;
  const short = useMemo(() => missingCards(deck, owned, copyLimit), [deck, owned, copyLimit]);
  const lacking = short.reduce((total, card) => total + card.inDeck - card.owned, 0);
  const craft = view?.craft;
  const cost = [
    craft?.mythic ? t('league.craft.mythic', { count: craft.mythic }) : null,
    craft?.rare ? t('league.craft.rare', { count: craft.rare }) : null,
    craft?.uncommon ? t('league.craft.uncommon', { count: craft.uncommon }) : null,
    craft?.common ? t('league.craft.common', { count: craft.common }) : null,
    craft?.coins ? t('league.craft.coins', { count: craft.coins }) : null,
  ].filter(Boolean).join(' · ');

  if (lacking === 0 && !error) {
    return <p className={[styles.banner, styles.bannerOk].join(' ')} role="status">{t('league.craft.allOwned')}</p>;
  }
  // the server's figures are for the last save; until it lands, the count here leads
  const current = craft && craft.copies === lacking;
  return (
    <div className={styles.banner} role="status">
      <p>
        <b>{t('league.craft.missing', { count: lacking })}</b>
        {current && cost ? ` · ${cost}` : ''}
      </p>
      <ul className={styles.shortList}>
        {short.slice(0, SHOWN).map((card) => (
          <li key={card.name}>{t('league.craft.card', { name: card.name, inDeck: card.inDeck, owned: card.owned })}</li>
        ))}
        {short.length > SHOWN && <li>{t('league.craft.more', { count: short.length - SHOWN })}</li>}
      </ul>
      {current && (craft?.unknown ?? []).length > 0 && <p className={styles.bannerNote}>{t('league.craft.unknown', { names: (craft?.unknown ?? []).join(', ') })}</p>}
      {current && !craft?.affordable && (craft?.unknown ?? []).length === 0 && <p className={styles.bannerNote}>{t('league.craft.short')}</p>}
      {error && <p className={styles.bannerError} role="alert">{error}</p>}
      <Button
        variant="decision"
        size="sm"
        icon={<Hammer size={14} />}
        disabled={!current || !craft?.affordable || crafting}
        onClick={() => view?.id && void craftMissing(view.id)}
      >
        {t('league.craft.go')}
      </Button>
    </div>
  );
}
