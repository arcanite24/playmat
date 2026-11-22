import { BookOpen, Check, Crown, GraduationCap, Lightbulb, ListChecks, Lock, Puzzle, Signpost, Swords, Users } from 'lucide-react';
import { useMemo, useState, type ReactNode } from 'react';
import { Link, Navigate } from 'react-router-dom';
import type { CareerCampaign, CareerChapter, CareerLesson, CareerNode, CareerOption } from '../../protocol/generated/views';
import { api } from '../connection';
import { registerMessages, useT } from '../i18n';
import messages from '../i18n/en/career';
import { useCoach } from '../stores/coach';
import { usePlay } from '../stores/play';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { useCareerState } from './careerData';
import { MODE_PATHS, chooseOption, playMode, playPrologue, useCampaigns } from './careerModesData';
import {
  chapterDone, chapterPath, focusChapter, focusNode, graduated, legCurve, nodeState, rewardLines, schoolsOf, storiesOf, twistLines,
  type NodeState, type PathStop,
} from './careerModesModel';
import { CardFace } from '../ui/CardFace';
import { DeckListDialog } from './DeckListDialog';
import { useSceneArt } from './careerScene';
import { Crest } from './ModeArt';
import { PortraitCard } from './Portraits';
import { Versus } from './Versus';
import { LineList, ModeResult } from './ModeParts';
import styles from './CareerModes.module.css';

registerMessages(messages);

/** Campaigns: chapters of duels along a path, each chapter with its own deck, a boss at the end and a set it opens. */
export function CareerCampaignScreen() {
  const t = useT();
  const state = useCareerState();
  const profile = state.data?.profile;
  const campaigns = useCampaigns(!!profile);

  if (state.isPending) return <p className={styles.note}>{t('career.loading')}</p>;
  if (!profile) return <Navigate to="/career" replace />;
  return (
    <div className={styles.page}>
      <div className={styles.scroll}>
        <ModeResult kind="campaign" />
        {campaigns.isPending && <p className={styles.note}>{t('career.loading')}</p>}
        {campaigns.isError && <p className={styles.error} role="alert">{t('career.failed')}</p>}
        {campaigns.data?.length === 0 && <p className={styles.note}>{t('career.campaign.none')}</p>}
        {storiesOf(campaigns.data).map((campaign) => <Campaign key={campaign.id} campaign={campaign} back={MODE_PATHS.campaign} />)}
        {schoolsOf(campaigns.data).length > 0 && <AcademyDoor schools={schoolsOf(campaigns.data)} />}
      </div>
    </div>
  );
}

/** The way from the story to the format schools. */
function AcademyDoor({ schools }: { schools: CareerCampaign[] }) {
  const t = useT();
  const open = schools.filter((school) => school.open).length;
  const done = schools.filter(graduated).length;
  return (
    <Link to="/career/academy" className={styles.academyDoor} data-nav>
      <GraduationCap size={34} aria-hidden="true" />
      <span>
        <b>{t('career.academy.title')}</b>
        <small>{t('career.academy.door', { open, total: schools.length, done })}</small>
      </span>
    </Link>
  );
}

/** A campaign's chapters and path; {@code back} is where its games come back to. */
export function Campaign({ campaign, back, children }: { campaign: CareerCampaign; back: string; children?: ReactNode }) {
  const t = useT();
  const chapters = useMemo(() => campaign.chapters ?? [], [campaign.chapters]);
  const [chosenId, setChosenId] = useState<string | null>(null);
  const chapter = chapters.find((item) => item.id === chosenId) ?? focusChapter(chapters);

  return (
    <section className={styles.campaign} aria-labelledby={`campaign-${campaign.id}`}>
      <header className={styles.modeHead}>
        <div>
          <h1 id={`campaign-${campaign.id}`} className={styles.modeTitle}>{campaign.name}</h1>
          <p className={styles.lead}>{campaign.text}</p>
        </div>
        <p className={styles.progressNote}>{t('career.campaign.progress', { done: campaign.done ?? 0, total: campaign.total ?? 0 })}</p>
      </header>
      {children}

      {campaign.prologue === 'guided' && <Prologue />}

      <div className={styles.chapterTabs} role="group" aria-label={t('career.campaign.chapters')}>
        {chapters.map((item) => {
          const progress = chapterDone(item);
          return (
            <button
              key={item.id}
              type="button"
              className={[styles.chapterTab, item.id === chapter?.id ? styles.chapterTabOn : ''].join(' ')}
              aria-pressed={item.id === chapter?.id}
              onClick={() => setChosenId(item.id ?? null)}
            >
              <Crest name={item.name ?? ''} colors={item.color} size={40} />
              <span className={styles.chapterTabText}>
                <b>{item.name}</b>
                <small>{item.done ? t('career.campaign.chapterDone') : t('career.campaign.chapterProgress', { done: progress.done, total: progress.total })}</small>
              </span>
              {item.done && <Check size={16} aria-hidden="true" className={styles.doneMark} />}
            </button>
          );
        })}
      </div>

      {chapter && <Chapter key={chapter.id} campaignId={campaign.id ?? ''} school={!!campaign.school} chapter={chapter} back={back} />}
    </section>
  );
}

/** The guided first game, as the campaign's prologue. */
function Prologue() {
  const t = useT();
  const played = useCoach((coach) => coach.firstGameDone);
  const playing = usePlay((play) => play.phase !== 'idle');
  const [busy, setBusy] = useState(false);

  async function start() {
    setBusy(true);
    try {
      await playPrologue();
    } catch (reason) {
      notify(t('career.playFailed'), reason instanceof Error ? reason.message : String(reason), 'error');
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className={styles.prologue} aria-labelledby="campaign-prologue">
      <BookOpen size={22} aria-hidden="true" className={styles.prologueIcon} />
      <div className={styles.prologueText}>
        <h2 id="campaign-prologue">{t('career.campaign.prologue')}</h2>
        <p>{t(played ? 'career.campaign.prologuePlayed' : 'career.campaign.prologueText')}</p>
      </div>
      <Button variant={played ? 'quiet' : 'print'} size="sm" busy={busy} disabled={playing} onClick={() => void start()}>
        {t(played ? 'career.campaign.prologueAgain' : 'career.campaign.prologuePlay')}
      </Button>
    </section>
  );
}

function Chapter({ campaignId, school, chapter, back }: { campaignId: string; school: boolean; chapter: CareerChapter; back: string }) {
  const t = useT();
  const nodes = useMemo(() => chapter.nodes ?? [], [chapter.nodes]);
  const path = useMemo(() => chapterPath(nodes), [nodes]);
  const [chosenId, setChosenId] = useState<string | null>(null);
  const node = nodes.find((item) => item.id === chosenId) ?? focusNode(chapter);
  const boss = nodes.find((item) => item.boss);
  useSceneArt(boss?.cover);

  return (
    <section className={styles.chapter} aria-labelledby={`chapter-${chapter.id}`}>
      <header className={styles.chapterHead}>
        <Crest name={chapter.name ?? ''} colors={chapter.color} size={72} />
        <div className={styles.chapterHeadText}>
          <h2 id={`chapter-${chapter.id}`} className={styles.chapterTitle}>{chapter.name}</h2>
          <p className={styles.lead}>{chapter.text}</p>
          <p className={styles.chapterMeta}>
            {chapter.deckName && <span>{t('career.campaign.deck', { deck: chapter.deckName })}</span>}
            {chapter.unlockSet && (
              <span>{t(chapter.done ? 'career.campaign.setOpened' : 'career.campaign.opensSet', { set: chapter.unlockSet, boss: boss?.name ?? '' })}</span>
            )}
          </p>
        </div>
      </header>

      <div className={styles.mapWrap}>
        <div className={styles.map} style={{ aspectRatio: `${path.width} / ${path.height}` }}>
          <svg className={styles.mapTrail} viewBox={`0 0 ${path.width} ${path.height}`} aria-hidden="true" focusable="false">
            {path.legs.map((leg, index) => (
              <path key={index} d={legCurve(leg)} className={[styles.leg, styles[`leg_${leg.state}`]].join(' ')} />
            ))}
          </svg>
          <ol className={styles.stops} aria-label={t('career.campaign.path')}>
            {path.stops.map((stop) => (
              <li
                key={stop.node.id}
                className={styles.stopItem}
                style={{ left: `${(stop.x / path.width) * 100}%`, top: `${(stop.y / path.height) * 100}%` }}
              >
                <Stop stop={stop} selected={stop.node.id === node?.id} onSelect={() => setChosenId(stop.node.id ?? null)} />
              </li>
            ))}
          </ol>
        </div>
      </div>

      {node && <NodeDetail key={node.id} campaignId={campaignId} school={school} chapter={chapter} node={node} back={back} />}
    </section>
  );
}

const STATE_KEYS = { locked: 'career.node.locked', open: 'career.node.open', done: 'career.node.done' } as const;

function Stop({ stop, selected, onSelect }: { stop: PathStop; selected: boolean; onSelect(): void }) {
  const t = useT();
  const { node, state } = stop;
  const Icon = state === 'done' ? Check : state === 'locked' ? Lock : node.type === 'choice' ? Signpost : node.type === 'trial' ? Puzzle : node.boss ? Crown : Swords;
  const label = [
    node.name, t(STATE_KEYS[state]), node.boss ? t('career.node.boss') : null, node.type === 'choice' ? t('career.node.choice') : null,
    node.type === 'trial' ? t('career.node.trial') : null,
  ].filter(Boolean).join(', ');
  return (
    <button
      type="button"
      className={[styles.stop, styles[`stop_${state}`], node.boss ? styles.stopBoss : '', selected ? styles.stopOn : ''].join(' ')}
      aria-pressed={selected}
      aria-label={label}
      onClick={onSelect}
    >
      <span className={styles.stopDot} aria-hidden="true">
        <Icon size={node.boss ? 24 : 18} />
      </span>
      <span className={styles.stopName} aria-hidden="true">{node.name}</span>
      {node.boss && <span className={styles.bossTag} aria-hidden="true">{t('career.node.boss')}</span>}
    </button>
  );
}

function NodeDetail({ campaignId, school, chapter, node, back }: { campaignId: string; school: boolean; chapter: CareerChapter; node: CareerNode; back: string }) {
  const t = useT();
  const state: NodeState = nodeState(node);
  const playing = usePlay((play) => play.phase !== 'idle');
  const [versus, setVersus] = useState(false);
  const [list, setList] = useState(false);
  const twists = twistLines(node.twists);
  const reward = rewardLines(node.reward);
  // what they say: the challenge before the duel, the parting word once it's won
  const line = state === 'done' ? node.after ?? node.before : node.before;
  const deckName = node.deckName ?? chapter.deckName ?? '';
  const trial = node.type === 'trial';
  const bestOf = (node.winsNeeded ?? 1) * 2 - 1;
  const others = node.others ?? [];
  const opponentName = node.opponentName ?? node.name ?? '';
  // a Five Paths chapter is its decks' colour; a school's act colour says nothing about the decks at the table
  const pathColors = school ? undefined : chapter.color;
  const start = () => playMode('campaign', () => api.careerCampaignPlay(campaignId, node.id!), trial ? null : { intro: node.before, lose: node.after, speaker: others.length > 0 ? opponentName : undefined }, {
    back,
    lessons: node.tips?.length || node.sideboard ? { tips: node.tips ?? [], sideboard: node.sideboard ?? null } : null,
  });

  return (
    <article className={styles.detail} aria-labelledby={`node-${node.id}`}>
      <div className={styles.detailArt}>
        {node.type === 'choice'
          ? <span className={styles.choiceArt} aria-hidden="true"><Signpost size={40} /></span>
          : <PortraitCard name={node.name ?? ''} colors={chapter.color} cover={node.cover} boss={node.boss} locked={state === 'locked'} size="sm" />}
      </div>
      <div className={styles.detailBody}>
        <header className={styles.detailHead}>
          <h3 id={`node-${node.id}`}>{node.name}</h3>
          <span className={[styles.stateTag, styles[`tag_${state}`]].join(' ')}>{t(STATE_KEYS[state])}</span>
          {node.boss && <span className={styles.bossBadge}><Crown size={14} aria-hidden="true" /> {t('career.node.boss')}</span>}
        </header>
        {node.text && <p className={styles.lead}>{node.text}</p>}
        {line && node.type !== 'choice' && (
          <blockquote className={styles.speech}>
            <p>“{line}”</p>
            <footer>{opponentName}</footer>
          </blockquote>
        )}

        {node.lesson && <LessonCard lesson={node.lesson} />}

        {node.type === 'choice' ? (
          <Choice campaignId={campaignId} node={node} state={state} />
        ) : trial ? (
          <Trial node={node} state={state} reward={reward.map((item) => t(item.key, item.vars))} playing={playing} onStart={start} />
        ) : (
          <>
            <div className={styles.detailCols}>
              <div>
                <h4 className={styles.subhead}>{t(state === 'done' ? 'career.node.paid' : 'career.node.pays')}</h4>
                {reward.length > 0 ? <LineList lines={reward} /> : <p className={styles.small}>{t('career.node.noReward')}</p>}
              </div>
              <div>
                <h4 className={styles.subhead}>{t(others.length > 0 ? 'career.node.pod' : 'career.node.opponent')}</h4>
                {others.length > 0 ? (
                  <ul className={styles.seats}>
                    {[{ name: opponentName, skill: node.skill }, ...others].map((seat) => (
                      <li key={seat.name}><Users size={14} aria-hidden="true" /> {seat.name} · {t('career.node.skill', { skill: seat.skill ?? 1 })}</li>
                    ))}
                  </ul>
                ) : (
                  <p className={styles.small}>{t('career.node.skill', { skill: node.skill ?? 1 })}</p>
                )}
                {bestOf > 1 && <p className={styles.small}><b>{t('career.node.bestOf', { count: bestOf })}</b></p>}
                {twists.length > 0 && (
                  <>
                    <h4 className={styles.subhead}>{t('career.twist.title')}</h4>
                    <LineList lines={twists} className={styles.twists} />
                  </>
                )}
              </div>
            </div>
            <div className={styles.actions}>
              {state === 'locked' ? (
                <p className={styles.small}><Lock size={14} aria-hidden="true" /> {t('career.node.lockedHint')}</p>
              ) : (
                <Button variant={state === 'open' ? 'decision' : 'print'} icon={<Swords size={16} />} disabled={playing} onClick={() => setVersus(true)} data-nav>
                  {t(state === 'done' ? 'career.node.playAgain' : 'career.node.play')}
                </Button>
              )}
              <small className={styles.small}>
                {t(node.deckName ? 'career.node.youPlay' : 'career.campaign.youPlay', { deck: deckName })}
              </small>
              <Button variant="quiet" size="sm" icon={<ListChecks size={16} />} onClick={() => setList(true)}>{t('career.node.deckList')}</Button>
            </div>
          </>
        )}
      </div>
      {list && <DeckListDialog campaignId={campaignId} chapterId={chapter.id} nodeId={node.id ?? ''} name={deckName} onClose={() => setList(false)} />}
      {versus && (
        <Versus
          kicker={chapter.name}
          opponent={{
            name: opponentName,
            tagline: others.length > 0 ? t('career.node.podWith', { names: others.map((seat) => seat.name).join(', ') }) : undefined,
            colors: pathColors, cover: node.cover, boss: node.boss, line: node.before,
          }}
          deck={{ name: deckName, colors: pathColors }}
          stakes={[...(bestOf > 1 ? [{ key: 'career.node.bestOf' as const, vars: { count: bestOf } }] : []), ...reward, ...twists].map((item) => t(item.key, item.vars))}
          onFight={start}
          onClose={() => setVersus(false)}
        />
      )}
    </article>
  );
}

/** What the duel teaches: a few sentences and the cards to know. */
function LessonCard({ lesson }: { lesson: CareerLesson }) {
  const t = useT();
  return (
    <section className={styles.lesson} aria-label={t('career.lesson.title')}>
      <h4 className={styles.lessonTitle}><Lightbulb size={16} aria-hidden="true" /> {lesson.title}</h4>
      <p>{lesson.text}</p>
      {(lesson.cards?.length ?? 0) > 0 && (
        <ul className={styles.lessonCards} aria-label={t('career.lesson.cards')}>
          {lesson.cards!.map((name) => (
            <li key={name} title={name}><CardFace card={{ name }} size="small" /></li>
          ))}
        </ul>
      )}
    </section>
  );
}

/** A trial: a set position, its goal and a hint for when you're stuck. */
function Trial({ node, state, reward, playing, onStart }: { node: CareerNode; state: NodeState; reward: string[]; playing: boolean; onStart(): Promise<void> }) {
  const t = useT();
  const [hint, setHint] = useState(false);
  const [busy, setBusy] = useState(false);
  async function begin() {
    setBusy(true);
    try {
      await onStart();
    } catch (reason) {
      notify(t('career.playFailed'), reason instanceof Error ? reason.message : String(reason), 'error');
    } finally {
      setBusy(false);
    }
  }
  return (
    <>
      <p className={styles.objective}>
        <Puzzle size={16} aria-hidden="true" /> {t(node.objective === 'survive' ? 'career.trial.survive' : 'career.trial.win')}
      </p>
      {reward.length > 0 && <p className={styles.small}>{t(state === 'done' ? 'career.node.paid' : 'career.node.pays')}: {reward.join(' · ')}</p>}
      {node.hint && hint && <p className={styles.small}><Lightbulb size={14} aria-hidden="true" /> {node.hint}</p>}
      <div className={styles.actions}>
        {state === 'locked' ? (
          <p className={styles.small}><Lock size={14} aria-hidden="true" /> {t('career.node.lockedHint')}</p>
        ) : (
          <Button variant={state === 'open' ? 'decision' : 'print'} icon={<Puzzle size={16} />} busy={busy} disabled={playing} onClick={() => void begin()} data-nav>
            {t(state === 'done' ? 'career.trial.again' : 'career.trial.start')}
          </Button>
        )}
        {node.hint && !hint && state !== 'locked' && (
          <Button variant="quiet" size="sm" icon={<Lightbulb size={14} />} onClick={() => setHint(true)}>{t('career.trial.hint')}</Button>
        )}
      </div>
    </>
  );
}

function Choice({ campaignId, node, state }: { campaignId: string; node: CareerNode; state: NodeState }) {
  const t = useT();
  const [busy, setBusy] = useState<string | null>(null);
  const [paid, setPaid] = useState<string[] | null>(null);
  const options = node.options ?? [];
  const taken = options.find((option) => option.id === node.choice);

  async function choose(option: CareerOption) {
    setBusy(option.id ?? null);
    try {
      const result = await chooseOption(campaignId, node.id!, option.id!);
      const lines = result.mode?.lines ?? [];
      setPaid(lines.length > 0 ? lines : [option.text ?? '']);
      notify(t('career.choice.taken'), option.text ?? '');
    } catch (reason) {
      notify(t('career.choice.failed'), reason instanceof Error ? reason.message : String(reason), 'error');
    } finally {
      setBusy(null);
    }
  }

  if (paid) {
    return (
      <div className={styles.paid} role="status">
        <h4 className={styles.subhead}>{t('career.choice.taken')}</h4>
        <ul className={styles.lines}>{paid.map((line, index) => <li key={index}>{line}</li>)}</ul>
      </div>
    );
  }
  if (state === 'done') {
    return <p className={styles.small}>{taken ? t('career.choice.took', { option: taken.text ?? '' }) : t('career.node.done')}</p>;
  }
  return (
    <div className={styles.options} role="group" aria-label={t('career.choice.pick')}>
      {state === 'locked' && <p className={styles.small}><Lock size={14} aria-hidden="true" /> {t('career.node.lockedHint')}</p>}
      {options.map((option) => (
        <button
          key={option.id}
          type="button"
          className={styles.option}
          disabled={state !== 'open' || busy !== null}
          aria-busy={busy === option.id || undefined}
          onClick={() => void choose(option)}
        >
          <b>{option.text}</b>
          <OptionReward option={option} />
        </button>
      ))}
    </div>
  );
}

function OptionReward({ option }: { option: CareerOption }) {
  const t = useT();
  const lines = rewardLines(option.reward);
  return <small>{lines.map((line) => t(line.key, line.vars)).join(' · ')}</small>;
}
