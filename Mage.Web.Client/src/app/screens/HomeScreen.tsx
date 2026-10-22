import { ChevronRight, SlidersHorizontal, Trophy } from 'lucide-react';
import { lazy, Suspense, useEffect, useMemo, useState, type CSSProperties } from 'react';
import { useNavigate } from 'react-router-dom';
import { META_PREFIX, rosterOf, sleeveFor, useDecks } from '../stores/decks';
import { useCoach } from '../stores/coach';
import { usePlay } from '../stores/play';
import { useT, type MessageKey } from '../i18n';
import { Button } from '../ui/Button';
import { DeckBox } from '../ui/DeckBox';
import { MatPrint } from '../ui/MatPrint';
import { storeCards } from '../ui/imageCache';
import { DEVICE_PICTURES } from '../nativePictures';
import { useImportDrop, useImportPaste } from '../decks/import/useImportShortcuts';
import { openImport } from '../stores/importSheet';
import { aiDeckLabel, describeAi } from './aiSetup';
import { opponentSource } from '../solo/freePlay';
import { useCareerMessages } from '../solo/careerMessages';
import { slotsFull, useEmailStatus, useGameSlots, useServerInfo } from '../queries';
import styles from './HomeScreen.module.css';

// the banner and its code form load only for a player whose address still needs confirming
const EmailBanner = lazy(() => import('./EmailConfirm').then((module) => ({ default: module.EmailBanner })));
// the AI settings load the first time they are opened, out of the entry chunk (they bring the solo messages too)
const AiSettingsDialog = lazy(() => import('./AiSettingsDialog').then((module) => ({ default: module.AiSettingsDialog })));
// saved games against the AI, with what resuming one needs, load with Home but out of the entry chunk
const SavedGames = lazy(() => import('../solo/SavedGames').then((module) => ({ default: module.SavedGames })));
// the month's server costs, on servers that take support
const SupportBar = lazy(() => import('../support/SupportBar').then((module) => ({ default: module.SupportBar })));

type Mode = 'ai' | 'practice' | 'friend';

// open tables and events have their own pages in the rail; here the only question is who you play
const MODES: { id: Mode; label: MessageKey }[] = [
  { id: 'ai', label: 'home.mode.ai' },
  { id: 'practice', label: 'home.mode.practice' },
  { id: 'friend', label: 'home.mode.friend' },
];

/** Home: your deck on your mat, one decision away from a game. */
export function HomeScreen() {
  const navigate = useNavigate();
  const t = useT();
  const decks = useDecks();
  const play = usePlay();
  const [mode, setMode] = useState<Mode>('ai');
  const [aiOpen, setAiOpen] = useState(false);
  // stays mounted once opened, so it can animate closed
  const [aiLoaded, setAiLoaded] = useState(false);
  if (aiOpen && !aiLoaded) setAiLoaded(true);
  const emailStatus = useEmailStatus().data;
  const emailUnconfirmed = !!emailStatus?.email && !emailStatus.verified && !!emailStatus.mail;
  const roster = useMemo(() => rosterOf(decks), [decks]);
  // a meta, ghost or mirror opponent, or adaptive skill, is described with the solo messages
  const soloReady = useCareerMessages(play.aiOptions.adaptive || opponentSource(play.aiOptions.opponentDeckId).kind !== 'roster');
  const selected = roster.find((deck) => deck.id === decks.selectedId) ?? roster[0] ?? null;
  const sleeve = sleeveFor(decks.sleeves, selected);
  const busy = play.phase !== 'idle';
  const firstGameDone = useCoach((state) => state.firstGameDone);
  const skipFirstGame = useCoach((state) => state.finish);
  // Career is offered by servers with accounts; it is opt-in, and its screen explains it first
  const serverInfo = useServerInfo().data;
  const career = serverInfo?.career === true;
  // what staff want everyone to know, e.g. that a contributors server's data may be reset
  const notice = serverInfo?.notice?.trim();
  // a small server runs a few games at once; past that a new game waits for a free slot
  const full = slotsFull(useGameSlots().data);
  const { dragging, dropProps } = useImportDrop();
  useImportPaste();

  useEffect(() => {
    if (!decks.loaded) void decks.refresh();
  }, [decks]);

  // the Android app keeps the chosen deck's pictures on the device, so its next game opens without the network
  const selectedId = selected?.id;
  useEffect(() => {
    if (!DEVICE_PICTURES || !selectedId) return;
    void useDecks.getState().loadList(selectedId)
      .then((deck) => storeCards([...deck.cards, ...deck.sideboard].map((card) => ({ name: card.cardName, setCode: card.setCode ?? undefined, cardNumber: card.cardNumber ?? undefined }))))
      .catch(() => undefined);
  }, [selectedId]);

  function start() {
    if (!selected) return;
    switch (mode) {
      case 'ai':
        void play.playVsAi(selected.id);
        break;
      case 'practice':
        void play.playVsAi(selected.id, { practice: true });
        break;
      case 'friend':
        navigate('/tables', { state: { host: selected.id } });
        break;
    }
  }

  return (
    <div className={styles.home}>
      <aside className={styles.roster} aria-label={t('home.yourDecks')}>
        <header className={styles.rosterHead}>
          <h2>{t('home.yourDecks')}</h2>
          <span className={styles.rosterActions}>
            <Button variant="quiet" size="sm" onClick={() => openImport()}>{t('home.import')}</Button>
            <Button variant="quiet" size="sm" onClick={() => navigate('/decks')}>{t('home.manage')}</Button>
          </span>
        </header>
        <div className={[styles.rosterList, dragging ? styles.dropping : ''].join(' ')} role="list" {...dropProps}>
          {roster.map((deck) => (
            <div role="listitem" key={deck.id}>
              <DeckBox
                deck={deck}
                sleeve={sleeveFor(decks.sleeves, deck)}
                selected={deck.id === selected?.id}
                onSelect={() => decks.select(deck.id)}
                onActivate={() => {
                  decks.select(deck.id);
                  if (mode === 'ai') void play.playVsAi(deck.id);
                  if (mode === 'practice') void play.playVsAi(deck.id, { practice: true });
                }}
              />
            </div>
          ))}
          {decks.loaded && roster.length === 0 && (
            <p className={styles.empty}>{t('home.empty')}</p>
          )}
        </div>
      </aside>

      <section className={styles.mat} style={{ '--sleeve': sleeve } as CSSProperties}>
        <MatPrint card={selected?.cover ?? null} />
        {notice && <p className={styles.serverNotice} role="note" aria-label={t('home.notice')}>{notice}</p>}
        <div className={styles.deckTitle}>
          <h1 className={styles.deckName}>{selected?.name ?? t('home.chooseDeck')}</h1>
          <p className={styles.deckNote}>{selected?.fromStarter ? t('decks.fromStarter', { count: selected.cardCount }) : selected?.note ?? ' '}</p>
        </div>

        <div className={styles.playZone} role="group" aria-labelledby="play-zone-label">
          <h2 id="play-zone-label" className={styles.zoneLabel}>{t('home.play')}</h2>
          {emailUnconfirmed && <Suspense fallback={null}><EmailBanner /></Suspense>}
          {!firstGameDone && (
            <div className={styles.firstGame}>
              <p className={styles.firstGameText}>{t('home.firstGame.text')}</p>
              <span className={styles.firstGameActions}>
                <Button variant="quiet" size="sm" onClick={skipFirstGame} disabled={busy}>{t('home.firstGame.skip')}</Button>
                <Button variant="print" size="sm" onClick={() => selected && void play.playVsAi(selected.id, { guided: true })} disabled={busy || !selected}>
                  {t('home.firstGame.start')}
                </Button>
              </span>
            </div>
          )}
          <div className={styles.modes} role="radiogroup" aria-label={t('home.opponent')}>
            {MODES.map((item) => (
              <button
                key={item.id}
                type="button"
                role="radio"
                aria-checked={mode === item.id}
                className={[styles.mode, mode === item.id ? styles.modeOn : ''].join(' ')}
                onClick={() => setMode(item.id)}
                disabled={busy}
              >
                {t(item.label)}
              </button>
            ))}
          </div>
          {mode === 'ai' && (
            <button type="button" className={styles.aiSetup} onClick={() => setAiOpen(true)} disabled={busy}>
              <SlidersHorizontal size={16} aria-hidden="true" />
              <span>{describeAi(
                t,
                play.aiOptions,
                aiDeckLabel(
                  t,
                  play.aiOptions,
                  roster.find((deck) => deck.id === play.aiOptions.opponentDeckId)?.name
                    ?? decks.metaDecks.find((deck) => META_PREFIX + deck.file === play.aiOptions.opponentDeckId)?.name
                    ?? null,
                  soloReady,
                ),
                soloReady ? t('solo.adaptive.summary') : undefined,
              )}</span>
              <span className={styles.aiSetupAction}>{t('home.change')}</span>
            </button>
          )}
          {mode === 'practice' && <p className={styles.modeNote}>{t('home.practiceNote')}</p>}
          {mode === 'friend' && <p className={styles.modeNote}>{t('home.friendNote')}</p>}
          {full && <p className={styles.modeNote} role="status">{t('home.serverFull')}</p>}
          {play.error && <p className={styles.error} role="alert">{play.error}</p>}
          <div className={styles.playRow}>
            {busy && (
              <Button variant="quiet" onClick={() => void play.cancel()}>{t('home.cancel')}</Button>
            )}
            <Button
              variant="decision"
              size="xl"
              busy={busy}
              disabled={!selected}
              onClick={start}
              className={styles.playButton}
            >
              {t(busy ? (play.phase === 'waitingForGame' ? 'home.shuffling' : 'home.settingUp') : mode === 'friend' ? 'home.continue' : 'home.play')}
            </Button>
          </div>
          <Suspense fallback={null}><SavedGames disabled={busy} /></Suspense>
          {career && (
            <button type="button" className={styles.career} onClick={() => navigate('/career')} disabled={busy}>
              <Trophy size={18} aria-hidden="true" />
              <span className={styles.careerText}>
                <b>{t('home.career')}</b>
                <small>{t('home.career.note')}</small>
              </span>
              <ChevronRight size={18} aria-hidden="true" />
            </button>
          )}
          {serverInfo?.support && <Suspense fallback={null}><SupportBar place="home" /></Suspense>}
        </div>
      </section>
      {aiLoaded && <Suspense fallback={null}><AiSettingsDialog open={aiOpen} onOpenChange={setAiOpen} /></Suspense>}
    </div>
  );
}
