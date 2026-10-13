import { Check, Crown, Flag, Swords } from 'lucide-react';
import { useMemo, useState } from 'react';
import { Navigate } from 'react-router-dom';
import type { CareerOffer, CareerOfferOption, CareerRun, CareerRunReward } from '../../protocol/generated/views';
import { api } from '../connection';
import { formatDate, registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { usePlay } from '../stores/play';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { useCareerState } from './careerData';
import { abandonGauntlet, pickGauntlet, playMode, useGauntlet } from './careerModesData';
import { GauntletEntry, RunMap, RunStatus } from './GauntletParts';
import { ScoreBoard } from './ScoreBoard';
import { mergeByName, rewardLines, runReward, toggleChoice, twistLines } from './careerModesModel';
import { useSceneArt } from './careerScene';
import { Crest } from './ModeArt';
import { CardArt } from './Portraits';
import { Versus } from './Versus';
import { ModeResult } from './ModeParts';
import styles from './CareerModes.module.css';

registerMessages(messages);

function message(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

/** The gauntlet: pay in, build a deck from starting packs, then cross the map (or the boss line); one loss ends the run. */
export function CareerGauntletScreen() {
  const t = useT();
  const state = useCareerState();
  const profile = state.data?.profile;
  const gauntlet = useGauntlet(!!profile);

  if (state.isPending) return <p className={styles.note}>{t('career.loading')}</p>;
  if (!profile) return <Navigate to="/career" replace />;
  const data = gauntlet.data;
  const run = data?.run;

  return (
    <div className={styles.page}>
      <div className={styles.scroll}>
        <ModeResult kind="gauntlet" />
        <header className={styles.modeHead}>
          <div>
            <h1 className={styles.modeTitle}>{t('career.gauntlet.title')}</h1>
            <p className={styles.lead}>{t('career.gauntlet.lead', { max: data?.maxWins ?? 8 })}</p>
          </div>
          {data && (data.bestWins ?? 0) > 0 && <p className={styles.progressNote}>{t('career.gauntlet.best', { wins: data.bestWins ?? 0 })}</p>}
        </header>
        {gauntlet.isPending && <p className={styles.note}>{t('career.loading')}</p>}
        {gauntlet.isError && <p className={styles.error} role="alert">{t('career.failed')}</p>}
        {data && (
          <div className={styles.split}>
            <div className={styles.mainCol}>
              {run?.offer ? <Offer offer={run.offer} run={run} /> : run ? <Ladder run={run} rewards={data.rewards ?? []} /> : <GauntletEntry data={data} coins={profile.coins ?? 0} />}
            </div>
            <aside className={styles.sideCol}>
              <Rewards rewards={data.rewards ?? []} wins={run?.wins ?? -1} />
              {run?.mode && <RunStatus run={run} />}
              {run && <RunDeck run={run} />}
              <Recent runs={data.recent ?? []} />
              {!run && <Boards recent={data.recent ?? []} />}
            </aside>
          </div>
        )}
      </div>
    </div>
  );
}

/** The waiting pick: starting packs or a bundle (choose, then confirm), or a draft pick, shop, rest, event or relic (one click each). */
function Offer({ offer, run }: { offer: CareerOffer; run: CareerRun }) {
  const t = useT();
  const pick = offer.pick ?? 1;
  const [chosen, setChosen] = useState<string[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const ready = chosen.length === pick;
  const direct = DIRECT.has(offer.kind ?? '');

  async function confirm(ids: string[]) {
    setBusy(ids[0] ?? null);
    try {
      await pickGauntlet(ids);
      setChosen([]);
    } catch (reason) {
      notify(t('career.gauntlet.pickFailed'), message(reason), 'error');
    } finally {
      setBusy(null);
    }
  }

  const title = offer.kind === 'start' && !offer.title
    ? t('career.gauntlet.pickStart', { count: pick, of: offer.options?.length ?? 0 })
    : offer.kind === 'bundle' && !offer.title
      ? t('career.gauntlet.pickBundle', { count: pick, of: offer.options?.length ?? 0 })
      : offer.kind === 'draft'
        ? t('career.gauntlet.draftPick', { round: offer.round ?? 1, pick: offer.draftPick ?? 1 })
        : offer.title ?? '';

  return (
    <section className={styles.panel} aria-labelledby="gauntlet-offer">
      <h2 id="gauntlet-offer" className={styles.panelTitle}>{title}</h2>
      {offer.text && <p className={styles.small}>{offer.text}</p>}
      {offer.kind === 'shop' && <p className={styles.small}>{t('career.gauntlet.runCoins', { coins: run.runCoins ?? 0 })}</p>}
      {offer.kind === 'draft' ? (
        <ul className={styles.draftGrid} aria-label={t('career.gauntlet.draftPack')}>
          {(offer.options ?? []).map((option) => (
            <li key={option.id}>
              <button type="button" className={styles.draftCard} disabled={busy !== null} aria-busy={busy === option.id || undefined} onClick={() => void confirm([option.id ?? ''])}>
                <CardArt card={option.cards?.[0] ? { name: option.cards[0].name, setCode: option.cards[0].setCode, cardNumber: option.cards[0].cardNumber } : undefined} name={option.name ?? ''} className={styles.draftArt} />
                <span>{option.name}</span>
              </button>
            </li>
          ))}
        </ul>
      ) : direct ? (
        <ul className={styles.choices}>
          {(offer.options ?? []).map((option) => {
            const price = option.price ?? 0;
            const short = price > (run.runCoins ?? 0);
            const cards = option.cards ?? [];
            return (
              <li key={option.id} className={styles.choice}>
                <Button variant={option.kind === 'leave' ? 'quiet' : 'print'} busy={busy === option.id} disabled={busy !== null || short}
                  onClick={() => void confirm([option.id ?? ''])}>
                  {option.name}{price > 0 ? ` · ${t('career.gauntlet.price', { price })}` : ''}
                </Button>
                {option.text && <small>{option.text}</small>}
                {cards.length > 0 && (
                  <ul className={styles.names} aria-label={t('career.gauntlet.contents', { name: option.name ?? '' })}>
                    {mergeByName(cards).map((card) => <li key={card.name}>{card.amount > 1 ? `${card.amount} × ${card.name}` : card.name}</li>)}
                  </ul>
                )}
              </li>
            );
          })}
        </ul>
      ) : (
        <>
          <p className={styles.small}>{t('career.gauntlet.chosen', { count: chosen.length, pick })}</p>
          <ul className={styles.offers}>
            {(offer.options ?? []).map((option) => (
              <li key={option.id}>
                <OfferOption option={option} chosen={chosen.includes(option.id ?? '')} onToggle={() => setChosen((current) => toggleChoice(current, option.id ?? '', pick))} />
              </li>
            ))}
          </ul>
          <div className={styles.actions}>
            <Button variant="decision" busy={busy !== null} disabled={!ready} onClick={() => void confirm(chosen)}>
              {t(pick === 1 ? 'career.gauntlet.takeOne' : 'career.gauntlet.take', { count: pick })}
            </Button>
          </div>
        </>
      )}
    </section>
  );
}

const DIRECT = new Set(['shop', 'rest', 'event', 'relic']);

function OfferOption({ option, chosen, onToggle }: { option: CareerOfferOption; chosen: boolean; onToggle(): void }) {
  const t = useT();
  const cards = option.cards ?? [];
  const count = cards.reduce((sum, card) => sum + (card.amount ?? 1), 0);
  return (
    <div className={[styles.offer, chosen ? styles.offerOn : ''].join(' ')}>
      <button type="button" className={styles.offerHead} aria-pressed={chosen} onClick={onToggle}>
        <Crest name={option.name ?? ''} colors={option.color} size={44} />
        <span className={styles.offerName}>
          <b>{option.name}</b>
          <small>{t('career.starter.cards', { count })}</small>
        </span>
        <span className={styles.offerCheck} aria-hidden="true">{chosen ? <Check size={18} /> : null}</span>
      </button>
      {option.text && <p className={styles.small}>{option.text}</p>}
      <ul className={styles.names} aria-label={t('career.gauntlet.contents', { name: option.name ?? '' })}>
        {mergeByName(cards).map((card) => (
          <li key={card.name}>{card.amount > 1 ? `${card.amount} × ${card.name}` : card.name}</li>
        ))}
      </ul>
    </div>
  );
}

/** The run in progress: the map (S022) or the line of bosses, the next fight, and the way out. */
function Ladder({ run, rewards }: { run: CareerRun; rewards: CareerRunReward[] }) {
  const t = useT();
  const playing = usePlay((play) => play.phase !== 'idle');
  const [versus, setVersus] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const wins = run.wins ?? 0;
  const now = rewardLines(runReward(rewards, wins));
  const next = run.next;
  const last = run.bosses?.length ?? 8;
  const map = run.mode === 'map';
  const endless = run.mode === 'endless';
  const boss = next?.type === 'boss' || (!map && !endless && next?.number === last);
  const heading = map
    ? t('career.map.title', { wins })
    : endless ? t('career.gauntlet.endlessRun', { wins }) : t('career.gauntlet.run', { wins, max: run.maxWins ?? 8 });
  useSceneArt(next?.cover);

  async function abandon() {
    try {
      await abandonGauntlet();
      notify(t('career.gauntlet.abandoned'), '');
    } catch (reason) {
      notify(t('career.gauntlet.abandonFailed'), message(reason), 'error');
    } finally {
      setConfirming(false);
    }
  }

  return (
    <>
      {map ? <RunMap run={run} onFight={() => !playing && setVersus(true)} /> : (
        <section className={styles.panel} aria-labelledby="gauntlet-run">
          <header className={styles.panelHead}>
            <h2 id="gauntlet-run" className={styles.panelTitle}>{heading}</h2>
            {now.length > 0 && <p className={styles.small}>{t('career.gauntlet.paysNow', { reward: now.map((line) => t(line.key, line.vars)).join(', ') })}</p>}
          </header>
          <ol className={styles.ladder} aria-label={t('career.gauntlet.ladder')}>
            {(run.bosses ?? []).map((item) => {
              const isNext = next?.number === item.number;
              const twists = twistLines(item.twists);
              return (
                <li key={item.number} className={[styles.rung, item.beaten ? styles.rungBeaten : '', isNext ? styles.rungNext : ''].join(' ')} aria-current={isNext ? 'step' : undefined}>
                  <span className={styles.rungNumber} aria-hidden="true">{item.number}</span>
                  <CardArt card={item.cover} name={item.name ?? ''} boss={!endless && (item.number ?? 0) >= last} className={styles.rungArt} />
                  <span className={styles.rungText}>
                    <b>{item.name}</b>
                    <small>
                      {t('career.node.skill', { skill: item.skill ?? 1 })}
                      {twists.length > 0 && ` · ${twists.map((line) => t(line.key, line.vars)).join(', ')}`}
                    </small>
                  </span>
                  <span className={styles.rungState}>
                    {item.beaten ? <><Check size={14} aria-hidden="true" /> {t('career.gauntlet.beaten')}</> : isNext ? t('career.gauntlet.next') : t('career.gauntlet.ahead')}
                  </span>
                </li>
              );
            })}
          </ol>
        </section>
      )}
      <div className={styles.actions}>
        {next && !map && (
          <Button variant="decision" size="lg" icon={boss ? <Crown size={18} /> : <Swords size={18} />} disabled={playing} onClick={() => setVersus(true)} data-nav>
            {t('career.gauntlet.play', { name: next.name ?? '' })}
          </Button>
        )}
        {confirming ? (
          <span className={styles.confirm} role="group" aria-label={t('career.gauntlet.abandon')}>
            <span className={styles.warn}>{t('career.gauntlet.abandonSure')}</span>
            <Button variant="danger" size="sm" onClick={() => void abandon()}>{t('career.gauntlet.abandonYes')}</Button>
            <Button variant="quiet" size="sm" onClick={() => setConfirming(false)}>{t('career.keep')}</Button>
          </span>
        ) : (
          <Button variant="quiet" size="sm" icon={<Flag size={16} />} onClick={() => setConfirming(true)}>{t('career.gauntlet.abandon')}</Button>
        )}
      </div>
      {versus && next && (
        <Versus
          kicker={heading}
          opponent={{ name: next.name ?? '', cover: next.cover, boss }}
          deck={{ name: t('career.gauntlet.title') }}
          stakes={[
            t('career.node.skill', { skill: next.skill ?? 1 }),
            ...twistLines(next.twists).map((line) => t(line.key, line.vars)),
            t('career.versus.oneLoss'),
          ]}
          onFight={() => playMode('gauntlet', () => api.careerGauntletPlay())}
          onClose={() => setVersus(false)}
        />
      )}
    </>
  );
}

function Rewards({ rewards, wins }: { rewards: CareerRunReward[]; wins: number }) {
  const t = useT();
  return (
    <section className={styles.card} aria-labelledby="gauntlet-rewards">
      <h2 id="gauntlet-rewards" className={styles.cardLabel}>{t('career.gauntlet.rewards')}</h2>
      <table className={styles.table}>
        <thead>
          <tr>
            <th scope="col">{t('career.gauntlet.wins')}</th>
            <th scope="col">{t('career.gauntlet.pays')}</th>
          </tr>
        </thead>
        <tbody>
          {rewards.map((reward) => (
            <tr key={reward.wins} className={reward.wins === wins ? styles.rowOn : undefined} aria-current={reward.wins === wins ? 'true' : undefined}>
              <th scope="row">{reward.wins}</th>
              <td>{rewardLines(reward).map((line) => t(line.key, line.vars)).join(', ')}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}

function RunDeck({ run }: { run: CareerRun }) {
  const t = useT();
  const cards = useMemo(() => mergeByName(run.deck).sort((a, b) => a.name.localeCompare(b.name)), [run.deck]);
  if (cards.length === 0) return null;
  return (
    <section className={styles.card} aria-labelledby="gauntlet-deck">
      <h2 id="gauntlet-deck" className={styles.cardLabel}>{t('career.gauntlet.deck', { count: run.deckSize ?? 0 })}</h2>
      <ul className={[styles.names, styles.namesScroll].join(' ')}>
        {cards.map((card) => <li key={card.name}>{card.amount > 1 ? `${card.amount} × ${card.name}` : card.name}</li>)}
      </ul>
    </section>
  );
}

const RUN_END: Record<string, 'career.run.won' | 'career.run.lost' | 'career.run.abandoned'> = {
  won: 'career.run.won',
  lost: 'career.run.lost',
  abandoned: 'career.run.abandoned',
};

/** The gauntlet's scoreboards (S033): wins at each ascension, the highest ascension, boss rush, endless, and the last seed. */
function Boards({ recent }: { recent: CareerRun[] }) {
  const t = useT();
  const seeded = recent.find((run) => run.seed);
  return (
    <>
      <ScoreBoard mode="gauntlet" board="a0" title={t('career.board.gauntlet')} />
      <ScoreBoard mode="ascension" board="all" title={t('career.board.ascension')} />
      <ScoreBoard mode="boss-rush" board="all" title={t('career.board.bossRush')} />
      <ScoreBoard mode="endless" board="all" title={t('career.board.endless')} />
      {seeded?.seed && <ScoreBoard mode="gauntlet-seed" board={seeded.seed} title={t('career.board.seed', { seed: seeded.seed })} />}
    </>
  );
}

function Recent({ runs }: { runs: CareerRun[] }) {
  const t = useT();
  if (runs.length === 0) return null;
  return (
    <section className={styles.card} aria-labelledby="gauntlet-recent">
      <h2 id="gauntlet-recent" className={styles.cardLabel}>{t('career.gauntlet.recent')}</h2>
      <ul className={styles.recentRuns}>
        {runs.slice(0, 6).map((run) => (
          <li key={run.id}>
            <span>{t(RUN_END[run.state ?? ''] ?? 'career.run.lost')} · {t('career.run.record', { wins: run.wins ?? 0, losses: run.losses ?? 0 })}{run.seed ? ` · ${run.seed}` : ''}</span>
            <small>{run.ended ? formatDate(run.ended, { dateStyle: 'medium' }) : ''}</small>
          </li>
        ))}
      </ul>
    </section>
  );
}
