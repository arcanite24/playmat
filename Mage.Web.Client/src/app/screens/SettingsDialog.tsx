import * as Tabs from '@radix-ui/react-tabs';
import { BookmarkPlus } from 'lucide-react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useLocation } from 'react-router-dom';
import { lazy, Suspense, useEffect, useId, useMemo, useRef, useState } from 'react';
import { api } from '../connection';
import { MAT_CLOTHS, clothSwatch } from '../match/playmats';
import bookmarkletSource from '../decks/import/bookmarklet.source.js?raw';
import type { CareerState, SkipPrioritySteps } from '../../protocol/generated/views';
import { resetCommand, type AutoRule, type AutoRuleKind } from '../../core/game/autoAnswer';
import { notificationsSupported, requestNotifications } from '../stores/attention';
import { useAutoAnswers } from '../stores/autoAnswers';
import { useGames } from '../stores/games';
import { DEFAULT_SETTINGS, FULL_CONTROL, STREAMLINED, useSettings, type PlaySettings } from '../stores/settings';
import { APP_NAME, APP_VERSION, PUBLIC_ORIGIN } from '../brand';
import { clearPictureStore, DEVICE_PICTURES, pictureStoreStats } from '../nativePictures';
import { formatNumber, LOCALES, registerMessages, useT, type LocalePreference, type MessageKey, type Translate } from '../i18n';
import messages from '../i18n/en/settings';
import { browserLocale } from '../i18n/locales';
import { RichText } from '../i18n/RichText';
import { Button } from '../ui/Button';
import { CARD_BACKS, cardBackImage, type CardBackId } from '../ui/cardBack';
import { Dialog } from '../ui/Dialog';
import { useServerInfo } from '../queries';
import { AccountSettings } from './AccountSettings';
import { useSupportPerks } from '../support/supportData';
import { LockedSwatch } from '../support/SupporterPicks';
import { owns } from '../support/supportModel';
import styles from './SettingsDialog.module.css';

registerMessages(messages);

// the locker loads with the tab: it's Career's, and draws card backs and art
const CosmeticsSettings = lazy(() => import('../career/CosmeticsLocker'));

const STEPS: (keyof SkipPrioritySteps)[] = ['upkeep', 'draw', 'main1', 'beforeCombat', 'endOfCombat', 'main2', 'endOfTurn'];

const PLAY_STYLES = [
  ['settings.style.streamlined', STREAMLINED, 'settings.style.streamlined.detail'],
  ['settings.style.full', FULL_CONTROL, 'settings.style.full.detail'],
] as const;

export function SettingsDialog({ open, onOpenChange }: { open: boolean; onOpenChange(open: boolean): void }) {
  const settings = useSettings((state) => state.settings);
  const update = useSettings((state) => state.update);
  const t = useT();
  // the Account tab: servers with accounts only
  const accounts = useServerInfo().data?.accounts === true;
  const career = useServerInfo().data?.career === true;
  // progression shows or hides per account, on every device (B283, B312)
  const careerState = useQuery<CareerState>({ queryKey: ['career', 'state'], queryFn: () => api.careerState(), enabled: career && accounts });
  const showProgress = careerState.data?.profile ? careerState.data.profile.showProgress !== false : null;
  const queryClient = useQueryClient();
  async function setShowProgress(show: boolean) {
    const profile = await api.careerShowProgress(show);
    queryClient.setQueryData<CareerState>(['career', 'state'], (state) => ({ ...(state ?? { enabled: true }), profile }));
  }
  // a link inside (a locked cosmetic, to the support page) leaves for another screen: the dialog goes with it
  const path = useLocation().pathname;
  const shownAt = useRef(path);
  useEffect(() => {
    if (path === shownAt.current) return;
    shownAt.current = path;
    if (open) onOpenChange(false);
  }, [path, open, onOpenChange]);

  const toggle = (key: keyof PlaySettings) => (
    <Switch checked={!!settings[key]} onChange={(value) => update({ [key]: value } as Partial<PlaySettings>)} />
  );

  function setStop(turn: 'yourTurn' | 'opponentTurn', step: keyof SkipPrioritySteps, value: boolean) {
    update({ stops: { ...settings.stops, [turn]: { ...settings.stops[turn], [step]: value } } });
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange} title={t('settings.title')} width="lg"
      footer={(
        <>
          <span className={styles.version}>{t('settings.version', { app: APP_NAME, version: APP_VERSION })}</span>
          <Button variant="quiet" onClick={() => update({ ...DEFAULT_SETTINGS, avatarId: settings.avatarId, flag: settings.flag, locale: settings.locale, matCloth: settings.matCloth, matArt: settings.matArt, matCard: settings.matCard, cardBack: settings.cardBack })}>
            {t('settings.restore')}
          </Button>
        </>
      )}>
      <Tabs.Root defaultValue="play" className={styles.tabs} orientation="vertical">
        <Tabs.List className={styles.list} aria-label={t('settings.sections')}>
          <Tabs.Trigger value="play" className={styles.trigger}>{t('settings.tab.play')}</Tabs.Trigger>
          <Tabs.Trigger value="stops" className={styles.trigger}>{t('settings.tab.stops')}</Tabs.Trigger>
          <Tabs.Trigger value="answers" className={styles.trigger}>{t('settings.tab.answers')}</Tabs.Trigger>
          <Tabs.Trigger value="display" className={styles.trigger}>{t('settings.tab.display')}</Tabs.Trigger>
          <Tabs.Trigger value="playmat" className={styles.trigger}>{t('settings.tab.playmat')}</Tabs.Trigger>
          <Tabs.Trigger value="sound" className={styles.trigger}>{t('settings.tab.sound')}</Tabs.Trigger>
          <Tabs.Trigger value="alerts" className={styles.trigger}>{t('settings.tab.alerts')}</Tabs.Trigger>
          <Tabs.Trigger value="import" className={styles.trigger}>{t('settings.tab.import')}</Tabs.Trigger>
          <Tabs.Trigger value="language" className={styles.trigger}>{t('settings.tab.language')}</Tabs.Trigger>
          {accounts && <Tabs.Trigger value="cosmetics" className={styles.trigger}>{t('settings.tab.cosmetics')}</Tabs.Trigger>}
          {accounts && <Tabs.Trigger value="account" className={styles.trigger}>{t('settings.tab.account')}</Tabs.Trigger>}
        </Tabs.List>

        <Tabs.Content value="play" className={styles.panel}>
          <div className={styles.style}>
            <span className={styles.styleLabel}>{t('settings.style')}</span>
            <div className={styles.styleChoices} role="radiogroup" aria-label={t('settings.style')}>
              {PLAY_STYLES.map(([label, preset, detail]) => {
                const active = Object.entries(preset).every(([key, value]) => settings[key as keyof PlaySettings] === value);
                return (
                  <button key={label} type="button" role="radio" aria-checked={active} className={active ? styles.styleOn : styles.styleChoice} onClick={() => update(preset)}>
                    <strong>{t(label)}</strong>
                    <span>{t(detail)}</span>
                  </button>
                );
              })}
            </div>
          </div>
          <Row label={t('settings.autoPass')} detail={t('settings.autoPass.detail')}>{toggle('autoPass')}</Row>
          <Row label={t('settings.theirTurn')} detail={t('settings.theirTurn.detail')}>{toggle('abilitiesOnTheirTurn')}</Row>
          <Row label={t('settings.skipCombat')} detail={t('settings.skipCombat.detail')}>{toggle('autoSkipCombat')}</Row>
          <Row label={t('settings.passAfterCasting')} detail={t('settings.passAfterCasting.detail')}>{toggle('passAfterCasting')}</Row>
          <Row label={t('settings.autoPay')} detail={t('settings.autoPay.detail')}>{toggle('autoPayMana')}</Row>
          <Row label={t('settings.autoPayRestricted')} detail={t('settings.autoPayRestricted.detail')}>{toggle('autoPayRestricted')}</Row>
          <Row label={t('settings.emptyPool')} detail={t('settings.emptyPool.detail')}>{toggle('confirmEmptyManaPool')}</Row>
          <Row label={t('settings.orderTriggers')} detail={t('settings.orderTriggers.detail')}>{toggle('autoOrderTriggers')}</Row>
          <Row label={t('settings.autoTarget')} detail={t('settings.autoTarget.detail')}>
            <Switch checked={settings.autoTargetLevel > 0} onChange={(value) => update({ autoTargetLevel: value ? 1 : 0 })} />
          </Row>
          <Row label={t('settings.handRequests')} detail={t('settings.handRequests.detail')}>{toggle('allowHandRequests')}</Row>
          <h3 className={styles.groupHeading}>{t('settings.automation')}</h3>
          <p className={styles.intro}>{t('settings.automation.intro')}</p>
          <Row label={t('settings.autoPayHybrid')} detail={t('settings.autoPayHybrid.detail')}>{toggle('autoPayHybrid')}</Row>
          <Row label={t('settings.autoPayAnyColor')} detail={t('settings.autoPayAnyColor.detail')}>{toggle('autoPayAnyColor')}</Row>
          <Row label={t('settings.autoPayPainful')} detail={t('settings.autoPayPainful.detail')}>{toggle('autoPayPainful')}</Row>
          <Row label={t('settings.autoPayX')} detail={t('settings.autoPayX.detail')}>{toggle('autoPayX')}</Row>
          <Row label={t('settings.xDefaultsToMax')} detail={t('settings.xDefaultsToMax.detail')}>{toggle('xDefaultsToMax')}</Row>
          <Row label={t('settings.skipSingleChoice')} detail={t('settings.skipSingleChoice.detail')}>{toggle('skipSingleChoice')}</Row>
          <Row label={t('settings.pickIdentical')} detail={t('settings.pickIdentical.detail')}>{toggle('pickIdentical')}</Row>
          <Row label={t('settings.smartPreselect')} detail={t('settings.smartPreselect.detail')}>{toggle('smartPreselect')}</Row>
          <Row label={t('settings.autoCrew')} detail={t('settings.autoCrew.detail')}>{toggle('autoCrew')}</Row>
          <Row label={t('settings.attackPlayer')} detail={t('settings.attackPlayer.detail')}>{toggle('attackPlayerByDefault')}</Row>
          <Row label={t('settings.autoAssignDamage')} detail={t('settings.autoAssignDamage.detail')}>{toggle('autoAssignDamage')}</Row>
          <Row label={t('settings.autoAssignTrample')} detail={t('settings.autoAssignTrample.detail')}>{toggle('autoAssignTrample')}</Row>
        </Tabs.Content>

        <Tabs.Content value="stops" className={styles.panel}>
          <p className={styles.intro}>{t('settings.stops.intro')}</p>
          <table className={styles.stops}>
            <thead>
              <tr><th scope="col">{t('settings.stops.step')}</th><th scope="col">{t('settings.stops.yourTurn')}</th><th scope="col">{t('settings.stops.theirTurn')}</th></tr>
            </thead>
            <tbody>
              {STEPS.map((step) => {
                const label = t(`settings.step.${step}`);
                return (
                  <tr key={step}>
                    <th scope="row">{label}</th>
                    <td><Switch label={t('settings.stops.yourTurnOf', { step: label })} checked={!!settings.stops.yourTurn?.[step]} onChange={(value) => setStop('yourTurn', step, value)} /></td>
                    <td><Switch label={t('settings.stops.theirTurnOf', { step: label })} checked={!!settings.stops.opponentTurn?.[step]} onChange={(value) => setStop('opponentTurn', step, value)} /></td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          <Row label={t('settings.stops.attackers')}>
            <Switch checked={!!settings.stops.stopOnDeclareAttackers} onChange={(value) => update({ stops: { ...settings.stops, stopOnDeclareAttackers: value } })} />
          </Row>
          <Row label={t('settings.stops.stack')}>
            <Switch checked={!!settings.stops.stopOnStackNewObjects} onChange={(value) => update({ stops: { ...settings.stops, stopOnStackNewObjects: value } })} />
          </Row>
        </Tabs.Content>

        <Tabs.Content value="answers" className={styles.panel}>
          <AutoAnswers />
        </Tabs.Content>

        <Tabs.Content value="display" className={styles.panel}>
          <Row label={t('settings.hideStarterDecks')} detail={t('settings.hideStarterDecks.detail')}>{toggle('hideStarterDecks')}</Row>
          <Row label={t('settings.motion')} detail={t('settings.motion.detail')}>{toggle('animations')}</Row>
          <Row label={t('settings.sortHand')} detail={t('settings.sortHand.detail')}>{toggle('sortHand')}</Row>
          <Row label={t('settings.autoDismissReveals')} detail={t('settings.autoDismissReveals.detail')}>{toggle('autoDismissReveals')}</Row>
          <Row label={t('settings.compactBoard')} detail={t('settings.compactBoard.detail')}>{toggle('compactBoard')}</Row>
          <Row label={t('settings.muteEmotes')} detail={t('settings.muteEmotes.detail')}>{toggle('muteEmotes')}</Row>
          <Row label={t('settings.paintedArt')} detail={t('settings.paintedArt.detail')}>{toggle('paintedArt')}</Row>
          <Row label={t('settings.cardStyles')} detail={t('settings.cardStyles.detail')}>
            <select
              className={styles.select}
              value={settings.paintedArt ? settings.cardStyles : 'off'}
              disabled={!settings.paintedArt}
              aria-label={t('settings.cardStyles')}
              onChange={(event) => update({ cardStyles: event.target.value as PlaySettings['cardStyles'] })}
            >
              <option value="all">{t('settings.cardStyles.all')}</option>
              <option value="mine">{t('settings.cardStyles.mine')}</option>
              <option value="off">{t('settings.cardStyles.off')}</option>
            </select>
          </Row>
          {DEVICE_PICTURES && <PictureStoreRow />}
        </Tabs.Content>

        <Tabs.Content value="playmat" className={styles.panel}>
          <PlaymatSettings />
        </Tabs.Content>

        <Tabs.Content value="alerts" className={styles.panel}>
          <p className={styles.intro}>{t('settings.alerts.intro', { app: APP_NAME })}</p>
          <Row
            label={t('settings.notifications')}
            detail={notificationsSupported() ? t('settings.notifications.detail') : t('settings.notifications.unsupported')}
          >
            <Switch
              checked={settings.notifications && notificationsSupported()}
              onChange={(value) => {
                if (!value) {
                  update({ notifications: false });
                  return;
                }
                void requestNotifications().then((granted) => update({ notifications: granted }));
              }}
            />
          </Row>
        </Tabs.Content>

        <Tabs.Content value="sound" className={styles.panel}>
          <Row label={t('settings.sound')} detail={t('settings.sound.detail')}>{toggle('sound')}</Row>
          <Row label={t('settings.volume')}>
            <div className={styles.volume}>
              <input
                type="range"
                min={0}
                max={1}
                step={0.05}
                value={settings.volume}
                disabled={!settings.sound}
                onChange={(event) => update({ volume: Number(event.target.value) })}
                onPointerUp={previewTurnCue}
                onKeyUp={previewTurnCue}
                aria-label={t('settings.volume.label')}
                aria-valuetext={formatNumber(settings.volume, { style: 'percent' })}
              />
              <span aria-hidden="true">{formatNumber(settings.volume, { style: 'percent' })}</span>
            </div>
          </Row>
          <Row label={t('settings.importantCues')} detail={t('settings.importantCues.detail')}>{toggle('importantCuesOnly')}</Row>
          {showProgress !== null && (
            <Row label={t('settings.showProgress')} detail={t('settings.showProgress.detail')}>
              <Switch checked={showProgress} onChange={(value) => void setShowProgress(value)} />
            </Row>
          )}
          {settings.careerOptIn && (
            <>
              <Row label={t('settings.careerMusic')} detail={t('settings.careerMusic.detail')}>{toggle('careerMusic')}</Row>
              <Row label={t('settings.musicVolume')}>
                <div className={styles.volume}>
                  <input
                    type="range"
                    min={0}
                    max={1}
                    step={0.05}
                    value={settings.musicVolume}
                    disabled={!settings.sound || !settings.careerMusic}
                    onChange={(event) => update({ musicVolume: Number(event.target.value) })}
                    aria-label={t('settings.musicVolume.label')}
                    aria-valuetext={formatNumber(settings.musicVolume, { style: 'percent' })}
                  />
                  <span aria-hidden="true">{formatNumber(settings.musicVolume, { style: 'percent' })}</span>
                </div>
              </Row>
            </>
          )}
        </Tabs.Content>

        <Tabs.Content value="import" className={styles.panel}>
          <Bookmarklet />
        </Tabs.Content>

        <Tabs.Content value="language" className={styles.panel}>
          <Row label={t('settings.language')} detail={t('settings.language.detail')}>
            <select
              className={styles.select}
              value={settings.locale}
              aria-label={t('settings.language')}
              onChange={(event) => update({ locale: event.target.value as LocalePreference })}
            >
              <option value="auto">{t('settings.language.auto', { language: languageName(browserLocale(navigator.languages ?? [navigator.language])) })}</option>
              {LOCALES.map((locale) => <option key={locale.code} value={locale.code} lang={locale.code}>{locale.name}</option>)}
            </select>
          </Row>
        </Tabs.Content>

        {accounts && (
          <Tabs.Content value="cosmetics" className={styles.panel}>
            <Suspense fallback={null}>
              <CosmeticsSettings />
            </Suspense>
          </Tabs.Content>
        )}

        {accounts && (
          <Tabs.Content value="account" className={styles.panel}>
            <AccountSettings />
          </Tabs.Content>
        )}
      </Tabs.Root>
    </Dialog>
  );
}

const CHOICE_LABEL: Record<AutoRule['choice'], MessageKey> = { yes: 'settings.choice.yes', no: 'settings.choice.no', first: 'settings.choice.first', last: 'settings.choice.last' };

function languageName(code: string): string {
  return LOCALES.find((locale) => locale.code === code)?.name ?? code;
}

/** Standing answers set with "Always…" in the games in progress, and resetting them. */
function AutoAnswers() {
  const rules = useAutoAnswers((state) => state.rules);
  const forget = useAutoAnswers((state) => state.forget);
  const t = useT();
  const sessions = useGames((state) => state.sessions);
  const games = useMemo(() => Object.entries(sessions)
    .filter(([, session]) => session.getState().mode === 'play')
    .map(([gameId]) => ({ gameId, rules: rules.filter((rule) => rule.gameId === gameId) })), [sessions, rules]);

  const reset = (gameId: string, kind: AutoRuleKind) => {
    const session = useGames.getState().sessions[gameId];
    if (session) void session.respond(resetCommand(kind)).catch(() => undefined);
    forget(gameId, kind);
  };

  return (
    <>
      <p className={styles.intro}>
        {t('settings.answers.intro')}
      </p>
      {games.length === 0 && <p className={styles.rowDetail}>{t('settings.answers.none')}</p>}
      {games.map((game, index) => (
        <section key={game.gameId} className={styles.answers} aria-label={games.length > 1 ? t('settings.answers.game', { number: index + 1 }) : t('settings.answers.thisGame')}>
          {(['answer', 'trigger'] as const).map((kind) => {
            const list = game.rules.filter((rule) => rule.kind === kind);
            return (
              <div key={kind}>
                <div className={styles.row}>
                  <div>
                    <div className={styles.rowLabel}>{rowLabel(t, kind, games.length > 1 ? index + 1 : null)}</div>
                    <div className={styles.rowDetail}>
                      {list.length === 0 ? t('settings.answers.empty') : t('settings.answers.count', { count: list.length })}
                    </div>
                  </div>
                  <Button variant="print" size="sm" onClick={() => reset(game.gameId, kind)}>{t('settings.answers.reset')}</Button>
                </div>
                {list.length > 0 && (
                  <ul className={styles.answerList}>
                    {list.map((rule) => (
                      <li key={rule.label}>
                        <span>{rule.label}</span>
                        <b>{t(CHOICE_LABEL[rule.choice])}</b>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            );
          })}
        </section>
      ))}
    </>
  );
}

/** "Questions", or "Questions · game 2" when several games are in progress. */
function rowLabel(t: Translate, kind: 'answer' | 'trigger', game: number | null): string {
  const label = t(kind === 'answer' ? 'settings.answers.questions' : 'settings.answers.triggers');
  return game === null ? label : t('settings.answers.ofGame', { label, number: game });
}

/** "Send to Playmat": a bookmark that brings the deck on the page over in one click. */
function Bookmarklet() {
  const link = useRef<HTMLAnchorElement>(null);
  const t = useT();
  useEffect(() => {
    // React won't render javascript: links, so the bookmark's address is set by hand
    const code = bookmarkletSource
      .replace(/\/\*[\s\S]*?\*\//g, '')
      .replace(/^\s*\/\/.*$/gm, '')
      .replace(/\n\s+/g, '\n')
      .trim()
      .replaceAll('__PLAYMAT_ORIGIN__', PUBLIC_ORIGIN)
      .replaceAll('__APP_NAME__', APP_NAME.replace(/[\\'"]/g, ''));
    link.current?.setAttribute('href', `javascript:${encodeURIComponent(code)}`);
  }, []);
  return (
    <div className={styles.bookmarklet}>
      <p className={styles.intro}>
        {t('settings.bookmarklet.intro')}
      </p>
      <a ref={link} className={styles.bookmarkletLink} draggable onClick={(event) => event.preventDefault()} title={t('settings.bookmarklet.drag')}>
        <BookmarkPlus size={18} aria-hidden="true" /> {t('settings.bookmarklet.name', { app: APP_NAME })}
      </a>
      <ol className={styles.bookmarkletSteps}>
        <li><RichText text={t('settings.bookmarklet.step1')} parts={{ link: <b>{t('settings.bookmarklet.name', { app: APP_NAME })}</b> }} /></li>
        <li>{t('settings.bookmarklet.step2')}</li>
        <li>{t('settings.bookmarklet.step3')}</li>
      </ol>
      <p className={styles.rowDetail}>{t('settings.bookmarklet.privacy')}</p>
    </div>
  );
}

const MAT_ARTS = [
  ['deck', 'settings.playmat.art.deck'],
  ['card', 'settings.playmat.art.card'],
  ['none', 'settings.playmat.art.none'],
] as const;

/** Your playmat: the cloth the match is dyed in and the art printed into your half. */
function PlaymatSettings() {
  const t = useT();
  const settings = useSettings((state) => state.settings);
  const update = useSettings((state) => state.update);
  const [name, setName] = useState(settings.matCard?.name ?? '');
  const [problem, setProblem] = useState<string | null>(null);
  const listId = useId();
  // Career cloths show once unlocked (the level track lists the unlocks; the Career screens share this query)
  const levels = useQuery({ queryKey: ['career', 'levels'], queryFn: () => api.careerLevels(), enabled: settings.careerOptIn, staleTime: 30_000 });
  const unlocks = levels.data?.unlocks;
  // the Patron cloth is a thank-you for supporting the server: shown once owned, and as a quiet teaser where the server
  // takes support
  const accounts = useServerInfo().data?.accounts === true;
  const support = useServerInfo().data?.support === true;
  const perks = useSupportPerks();
  const owned = perks.data?.perks;
  const cloths = useMemo(
    () => MAT_CLOTHS.filter((cloth) => (cloth.career ? !!unlocks?.some((unlock) => unlock.kind === 'playmat' && unlock.id === cloth.id) : cloth.perk ? owns(owned, 'playmat', cloth.id) : true)),
    [unlocks, owned],
  );
  const teasers = accounts && support && perks.isSuccess ? MAT_CLOTHS.filter((cloth) => cloth.perk && !owns(owned, 'playmat', cloth.id)) : [];
  const typed = name.trim();
  const suggestions = useQuery({
    queryKey: ['playmat-names', typed.toLowerCase()],
    queryFn: () => api.searchCards({ nameContains: typed, uniqueNames: true, count: 12, start: 0 }),
    enabled: typed.length >= 3,
    staleTime: 60_000,
  });

  async function chooseCard() {
    setProblem(null);
    const found = await api.searchCards({ name: typed, uniqueNames: true, count: 1, start: 0 }).catch(() => []);
    const card = found[0];
    if (!card?.expansionSetCode || !card.cardNumber) {
      setProblem(t('settings.playmat.card.missing', { name: typed }));
      return;
    }
    update({ matArt: 'card', matCard: { name: card.name, setCode: card.expansionSetCode, cardNumber: card.cardNumber } });
  }

  return (
    <>
      <Row label={t('settings.playmat.cloth')} detail={t('settings.playmat.cloth.detail')}>
        <div className={styles.cloths}>
          <div className={styles.cloths} role="radiogroup" aria-label={t('settings.playmat.cloth')}>
            {cloths.map((cloth) => (
              <button
                key={cloth.id}
                type="button"
                role="radio"
                aria-checked={settings.matCloth === cloth.id}
                aria-label={t(`settings.playmat.cloth.${cloth.id}` as MessageKey)}
                title={cloth.career || cloth.perk
                  ? `${t(`settings.playmat.cloth.${cloth.id}` as MessageKey)} · ${t(cloth.career ? 'settings.playmat.cloth.career' : 'settings.playmat.cloth.perk')}`
                  : t(`settings.playmat.cloth.${cloth.id}` as MessageKey)}
                className={styles.cloth}
                style={clothSwatch(cloth)}
                onClick={() => update({ matCloth: cloth.id })}
              />
            ))}
          </div>
          {teasers.map((cloth) => (
            <LockedSwatch
              key={cloth.id}
              label={`${t(`settings.playmat.cloth.${cloth.id}` as MessageKey)} · ${t('settings.playmat.cloth.perk')}`}
              className={styles.cloth}
              style={clothSwatch(cloth)}
            />
          ))}
        </div>
      </Row>
      <Row label={t('settings.playmat.art')} detail={t('settings.playmat.art.detail')}>
        <select
          className={styles.select}
          value={settings.matArt === 'card' && !settings.matCard ? 'deck' : settings.matArt}
          aria-label={t('settings.playmat.art')}
          onChange={(event) => update({ matArt: event.target.value as PlaySettings['matArt'] })}
        >
          {MAT_ARTS.map(([value, key]) => (
            <option key={value} value={value} disabled={value === 'card' && !settings.matCard}>
              {value === 'card' && settings.matCard?.name ? `${t(key)}: ${settings.matCard.name}` : t(key)}
            </option>
          ))}
        </select>
      </Row>
      <form
        className={styles.matCard}
        onSubmit={(event) => {
          event.preventDefault();
          if (typed) void chooseCard();
        }}
      >
        <label className={styles.rowLabel} htmlFor={`${listId}-input`}>{t('settings.playmat.card')}</label>
        <div className={styles.matCardRow}>
          <input
            id={`${listId}-input`}
            className={styles.select}
            value={name}
            list={listId}
            autoComplete="off"
            placeholder={t('settings.playmat.card.placeholder')}
            onChange={(event) => setName(event.target.value)}
          />
          <datalist id={listId}>
            {(suggestions.data ?? []).map((card) => <option key={card.name} value={card.name} />)}
          </datalist>
          <Button type="submit" variant="print" disabled={!typed}>{t('settings.playmat.card.use')}</Button>
        </div>
        {problem && <p className={styles.rowDetail} role="alert">{problem}</p>}
      </form>
      <div className={styles.backs}>
        <div className={styles.rowLabel} id={`${listId}-backs`}>{t('settings.playmat.back')}</div>
        <div className={styles.rowDetail}>{t('settings.playmat.back.detail')}</div>
        <div className={styles.backGrid} role="radiogroup" aria-labelledby={`${listId}-backs`}>
          {([...CARD_BACKS, 'classic'] as CardBackId[]).map((id) => {
            const label = t(`settings.playmat.back.${id}` as MessageKey);
            return (
              <button
                key={id}
                type="button"
                role="radio"
                aria-checked={settings.cardBack === id}
                aria-label={label}
                title={label}
                className={styles.back}
                onClick={() => update({ cardBack: id })}
              >
                <img src={cardBackImage(id)} alt="" loading="lazy" decoding="async" draggable={false} />
              </button>
            );
          })}
        </div>
      </div>
    </>
  );
}

/** The Android app's stored card pictures: how much room they take, and a way to free it. */
function PictureStoreRow() {
  const t = useT();
  const stats = useQuery({ queryKey: ['pictureStore'], queryFn: pictureStoreStats });
  const [clearing, setClearing] = useState(false);
  const megabytes = Math.round((stats.data?.bytes ?? 0) / (1024 * 1024));
  return (
    <Row
      label={t('settings.pictureStore')}
      detail={stats.data ? t('settings.pictureStore.detail', { count: formatNumber(stats.data.files), size: formatNumber(megabytes) }) : undefined}
    >
      <Button
        variant="print"
        size="sm"
        busy={clearing}
        disabled={!stats.data?.files}
        onClick={() => {
          setClearing(true);
          void clearPictureStore().finally(() => {
            setClearing(false);
            void stats.refetch();
          });
        }}
      >
        {t('settings.pictureStore.clear')}
      </Button>
    </Row>
  );
}

function Row({ label, detail, children }: { label: string; detail?: string; children: React.ReactNode }) {
  return (
    <div className={styles.row}>
      <div>
        <div className={styles.rowLabel}>{label}</div>
        {detail && <div className={styles.rowDetail}>{detail}</div>}
      </div>
      {children}
    </div>
  );
}

function Switch({ checked, onChange, label }: { checked: boolean; onChange(value: boolean): void; label?: string }) {
  return (
    <button type="button" role="switch" aria-checked={checked} aria-label={label} className={styles.switch} onClick={() => onChange(!checked)}>
      <span className={styles.knob} />
    </button>
  );
}

/** the synthesizer belongs to the match screen; load it only when someone tries the volume */
function previewTurnCue() {
  void import('../match/sound').then(({ previewCue }) => previewCue('turn'));
}
