import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { Menu } from 'lucide-react';
import { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import type { GameSession, GameSessionState } from '../../core/game/gameSession';
import type { Command } from '../../core/game/interaction';
import { commanderDamageTo, commandersByPlayer, type CommanderStatus } from '../../core/game/commander';
import { askedAttacker, defenderChoice, inRange } from '../../core/game/multiplayer';
import { parsePayment } from '../../core/game/payment';
import { xLimit } from '../../core/game/announceX';
import { askedAboutCards } from '../../core/game/reveals';
import { delayLabel, isBroadcastDelay } from '../../core/game/broadcastDelay';
import { spectatorHand } from '../../core/game/spectatorHand';
import { matchProgress } from '../../core/game/matchProgress';
import { pregameChoice } from '../../core/game/pregame';
import { offBoardTargets } from '../../core/game/offBoardTargets';
import type { CardView, ChatMessage, GameView, PlayerView } from '../../protocol/generated/views';
import { rosterOf, sleeveFor, SLEEVE_COLORS, useDecks } from '../stores/decks';
import { useT } from '../i18n';
import { useEvents } from '../stores/events';
import { useGames } from '../stores/games';
import { usePlay } from '../stores/play';
import { useSettings } from '../stores/settings';
import { notify } from '../stores/toasts';
import { Button } from '../ui/Button';
import { CardFace } from '../ui/CardFace';
import { Dialog } from '../ui/Dialog';
import { MatPrint } from '../ui/MatPrint';
import { SettingsDialog } from '../screens/SettingsDialog';
import { Stitch } from '../ui/Stitch';
import { ActionCluster } from './ActionCluster';
import { AlwaysAnswerMenu, TriggerOrderOptions } from './AutoAnswer';
import { BetweenGames } from './BetweenGames';
import { Arrows } from './Arrows';
import { buildBoard, fitCardWidth, type PermanentGroup, type PlayerBoard } from './boardModel';
import { CardDetail } from './CardDetail';
import { Coach } from './Coach';
import { LessonCoach } from './LessonCoach';
import { CardZoom } from './CardZoom';
import { DamageAssigner } from './DamageAssigner';
import { damageCorner, useDamageSplit } from './useDamageSplit';
import { EmoteBubbles } from './EmoteBubbles';
import { useOpponentVoice } from './useOpponentVoice';
import { setCardMotion, useFlipOrigin } from './flip';
import { GameLog } from './GameLog';
import { Hand } from './Hand';
import './matchMessages';
import { useMatchUi } from './matchUi';
import { useGameCues } from './useGameCues';
import { useWarmImages } from './useWarmImages';
import { useAutoPay } from './useAutoPay';
import { useAutoPass } from './useAutoPass';
import { useBlockDrag } from './useBlockDrag';
import { useAutoOrder } from './useAutoOrder';
import { Vfx } from './Vfx';
import { CardPicker, ChoicePanel, GameOverOverlay, MulliganOverlay, StartingPlayerOverlay, ZoneViewer } from './Overlays';
import { PermanentStack } from './PermanentStack';
import { clothOf, clothStyle } from './playmats';
import { PracticeTools } from './PracticeTools';
import { AdviceDialog } from './AdviceDialog';
import { GameReview } from '../review/GameReview';
import { WhatIfDialog } from '../solo/WhatIfDialog';
import { refreshAssists, saveAndLeave, tryTurnAgain, useSoloAssists } from '../solo/soloData';
import { PhaseLadder } from './PhaseLadder';
import { DefenderPicker } from './DefenderPicker';
import { MiniPiles, Piles } from './Piles';
import { PlayerPlate } from './PlayerPlate';
import { SpectatorBar, WatcherCount } from './SpectatorBar';
import { useWatchers } from './useWatchers';
import { Reveals } from './Reveals';
import { StackZone } from './StackZone';
import { Stage } from './Stage';
import { matchLayout, type MatchLayout, type RowsPlacement } from './matchLayout';
import styles from './MatchStage.module.css';

const EMPTY: ReadonlySet<string> = new Set();
const ignore = () => undefined;
const NO_CARDS: readonly CardView[] = [];
const NO_COMMANDERS: readonly CommanderStatus[] = [];

/** Attacking and blocking permanents, from the combat groups. */
function combatSets(combat: GameView['combat']) {
  const attacking = new Set<string>();
  const blocking = new Set<string>();
  const links: [string, string][] = [];
  const attacks: [string, string][] = [];
  for (const group of combat ?? []) {
    const attackers = Object.keys(group.attackers ?? {});
    attackers.forEach((id) => attacking.add(id));
    const blockers = Object.keys(group.blockers ?? {});
    // unblocked attackers point at what they attack; blocked ones are tied to their blockers instead
    if (group.defenderId && blockers.length === 0 && !group.isBlocked) attackers.forEach((id) => attacks.push([id, group.defenderId!]));
    for (const blocker of blockers) {
      blocking.add(blocker);
      attackers.forEach((attacker) => links.push([blocker, attacker]));
    }
  }
  return { attacking, blocking, links, attacks };
}

function useSleeves() {
  const deckId = usePlay((state) => state.deckId);
  const sleeves = useDecks((state) => state.sleeves);
  const saved = useDecks((state) => state.saved);
  const starters = useDecks((state) => state.starters);
  return useMemo(() => {
    const deck = rosterOf({ saved, starters }).find((candidate) => candidate.id === deckId);
    const mine = sleeveFor(sleeves, deck);
    // opponents' sleeves must read as "theirs" at a glance
    const theirs = SLEEVE_COLORS.find((color) => color !== mine && color !== SLEEVE_COLORS[0]) ?? SLEEVE_COLORS[2];
    return { mine, theirs, cover: deck?.cover ?? null };
  }, [deckId, sleeves, saved, starters]);
}

export interface MatchStageProps {
  session: GameSession;
  state: GameSessionState;
  /** replays: the game log as it stood at the moment shown (the stage doesn't join a chat) */
  replayLog?: ChatMessage[];
  /** where leaving goes, instead of closing the game and going back to Play */
  onLeave?: () => void;
}

export function MatchStage({ session, state, replayLog, onLeave }: MatchStageProps) {
  const { view, interaction, playerId, mode, awaitingServer } = state;
  const t = useT();
  const navigate = useNavigate();
  const animations = useSettings((settings) => settings.settings.animations);
  const autoPay = useSettings((settings) => settings.settings.autoPayMana);
  // watching and replays: the broadcast options
  const spectating = mode !== 'play';
  const broadcastDelay = useSettings((settings) => settings.settings.broadcastDelay);
  const hideHands = useSettings((settings) => settings.settings.hideHands);
  const largeZoom = useSettings((settings) => settings.settings.largeZoom);
  useEffect(() => {
    // the session only delays a game being watched; turning the delay off catches up at once
    session.setBroadcastDelay(mode === 'watch' && isBroadcastDelay(broadcastDelay) ? broadcastDelay * 1000 : 0);
  }, [session, mode, broadcastDelay]);
  const viewer = useMatchUi((ui) => ui.viewer);
  const openViewer = useMatchUi((ui) => ui.openViewer);
  const sleeves = useSleeves();
  const [holding, setHolding] = useState(false);
  const fullControl = useSettings((settings) => settings.fullControl);
  const setFullControl = useSettings((settings) => settings.setFullControl);
  // full control lasts for the match on screen, as on Arena
  useEffect(() => () => useSettings.getState().setFullControl(false), []);

  useEffect(() => setCardMotion(animations), [animations]);
  // a new game starts with a clean table
  useEffect(() => () => useMatchUi.setState({ zoom: null, dragging: null, viewer: null, logOpen: false, detail: null }), []);

  const board = useMemo(() => buildBoard(view, playerId), [view, playerId]);
  // combat damage is split on the creatures themselves: clicking one adds a point of damage to it
  const damageSplit = useDamageSplit(interaction, view);
  const clickable = useMemo(
    () => new Set(damageSplit ? damageSplit.assignment.recipients.map((recipient) => recipient.id) : interaction.clickable.keys()),
    [interaction.clickable, damageSplit],
  );
  const combat = view?.combat;
  const { attacking, blocking, links, attacks } = useMemo(() => combatSets(combat), [combat]);
  // lands can always tap for mana while you hold priority; glowing them all would drown the real options
  const quiet = useMemo(() => {
    if (interaction.mode !== 'priority' || !board.me) return EMPTY;
    const lands = new Set<string>();
    for (const group of board.me.back) {
      for (const permanent of group.members) {
        if ((permanent.cardTypes ?? []).includes('LAND')) lands.add(permanent.id!);
      }
    }
    return lands;
  }, [interaction.mode, board.me]);
  const hand = useMemo(() => Object.values(view?.myHand ?? {}), [view?.myHand]);
  // watching: the seat's hand, if its player lets us see it, and only backs while hands are hidden for the broadcast
  // (a replay of one's own game shows the hand that was held, as before)
  const seatHand = useMemo(
    () => (spectating ? spectatorHand(view, board.me?.player, mode === 'watch' && hideHands) : null),
    [spectating, mode, view, board.me?.player, hideHands],
  );
  const stack = useMemo(() => Object.values(view?.stack ?? {}), [view?.stack]);

  const onCommand = useCallback((command: Command) => {
    // one answer per question: while the server works on the last one, further answers are dropped
    if (command.type !== 'action' && session.getState().awaitingServer) return;
    if (command.type === 'action' && (command.action === 'HOLD_PRIORITY' || command.action === 'UNHOLD_PRIORITY')) {
      setHolding(command.action === 'HOLD_PRIORITY');
    }
    void session.respond(command).catch(() => undefined);
  }, [session]);
  const onResend = useCallback(() => void session.resend().catch(() => undefined), [session]);
  const onResync = useCallback(() => {
    void session.resync().then((resent) => {
      if (!resent) notify('Nothing to answer', "The game isn't waiting for you. It goes on as soon as the server is ready.");
    });
  }, [session]);
  // the blocker just chosen: the server's follow-up question ("Select attacker to block") doesn't say which it is
  const [lastBlocker, setLastBlocker] = useState<string | null>(null);
  // likewise the attacker, when the server asks what it attacks
  const [lastAttacker, setLastAttacker] = useState<string | null>(null);
  const [reviewing, setReviewing] = useState(false);
  const onClick = useCallback((id: string) => {
    const recipient = damageSplit?.assignment.recipients.findIndex((candidate) => candidate.id === id) ?? -1;
    if (damageSplit && recipient >= 0) {
      if (!session.getState().awaitingServer) damageSplit.add(recipient);
      return;
    }
    if (session.getState().interaction.mode === 'declareBlockers') setLastBlocker(id);
    if (session.getState().interaction.mode === 'declareAttackers') setLastAttacker(id);
    session.click(id);
  }, [session, damageSplit]);

  const blockDrop = useBlockDrag(session, state);
  const onBlockDrop = useCallback((blockerId: string, attackerId: string) => {
    setLastBlocker(blockerId);
    return blockDrop(blockerId, attackerId);
  }, [blockDrop]);

  const myId = board.me?.player.playerId ?? null;
  const commanders = useMemo(() => commandersByPlayer(view), [view]);
  const damageTaken = useCallback(
    (player: PlayerView) => commanderDamageTo(player, commanders).map((hit) => ({ name: hit.commander.name, amount: hit.amount })),
    [commanders],
  );
  useWarmImages(view);
  useAutoPay(session, state, board.me?.isMe ? board.me : null, autoPay && mode === 'play');
  // holding priority means the player wants every stop
  const autoPassing = useAutoPass(session, state, !holding);
  useGameCues(state, myId, !!autoPassing);
  useAutoOrder(session, state);
  const sleeveOf = useCallback((card: CardView) => (card.controllerId && card.controllerId !== myId ? sleeves.theirs : sleeves.mine), [myId, sleeves]);
  const originOf = useCallback((card: CardView) => (card.controllerId && card.controllerId !== myId ? `hand:${card.controllerId}` : undefined), [myId]);

  // games inside an event lead back to the event; others back to Play
  const eventId = useEvents((events) => events.currentTournamentId);
  const returnPath = usePlay((play) => play.returnPath);
  // Career matches come back through their rewards (the return path); a counted game against the AI shows what it
  // paid on the way out too
  const career = !!returnPath?.startsWith('/career');
  const vsAi = board.opponents.length > 0 && board.opponents.every((opponent) => opponent.player.isHuman === false);
  useOpponentVoice(state, myId, career);
  const rewardsAfter = mode === 'play' && !!state.gameOver && !eventId && !career
    && vsAi && !usePlay.getState().lastOptions.practice && useSettings.getState().settings.careerOptIn;
  // a match against people (not in an event or Career) was found on Tables, and leads back there
  const vsPeople = mode === 'play' && !eventId && !career && board.opponents.length > 0 && !vsAi;
  const exit = useCallback((rewards: boolean) => {
    if (onLeave) {
      onLeave();
      return;
    }
    useGames.getState().close(state.gameId);
    const back = eventId ? `/event/${eventId}` : vsPeople ? '/tables' : returnPath ?? '/';
    navigate(rewards ? `/career/rewards?game=${encodeURIComponent(state.gameId)}&back=${encodeURIComponent(back)}` : back);
  }, [navigate, state.gameId, eventId, returnPath, onLeave, vsPeople]);
  const leave = useCallback(() => exit(rewardsAfter), [exit, rewardsAfter]);
  const deckId = usePlay((play) => play.deckId);
  // players see who is watching them too
  const watchers = useWatchers(state.gameId, mode === 'play' && !state.gameOver);
  // between games of a match the server deals the next game by itself: show the score, not a way out
  const betweenGames = useMemo(() => {
    const progress = mode === 'play' ? matchProgress(state.endInfo) : null;
    return progress && !progress.over ? progress : null;
  }, [mode, state.endInfo]);
  const playAgain = useCallback(() => {
    const { lastOptions } = usePlay.getState();
    exit(false);
    if (deckId) void usePlay.getState().playVsAi(deckId, lastOptions);
  }, [exit, deckId]);

  // the arrow starts at the spell being cast (top of the stack) while it chooses targets
  const choosingTargets = interaction.mode === 'target' && !awaitingServer;
  const arrowSource = stack[0]?.id ?? null;
  const arrowTargets = useMemo(() => {
    if (choosingTargets) return [...interaction.selected];
    return (stack[0]?.targets ?? []).filter(Boolean) as string[];
  }, [choosingTargets, interaction.selected, stack]);

  const prompt = interaction.prompt;
  const targeting = interaction.mode === 'target' && !awaitingServer;
  // the decision corner names the blocker when the server asks which attacker it blocks
  // while paying: what is still owed, and for what
  const payment = useMemo(() => (interaction.mode === 'payMana' && prompt?.kind === 'playMana' ? parsePayment(prompt) : null), [interaction.mode, prompt]);
  const cornerInteraction = useMemo(() => {
    if (damageSplit) return damageCorner(interaction, damageSplit);
    if (payment?.cost && !(prompt?.kind === 'playMana' && prompt.isX)) {
      return { ...interaction, headline: payment.sourceName ? `Pay ${payment.cost} for ${payment.sourceName}` : `Pay ${payment.cost}` };
    }
    if (interaction.mode !== 'target' || !/attacker to block/i.test(prompt?.text ?? '') || !lastBlocker) return interaction;
    const blocker = board.me?.front.flatMap((group) => group.members).find((card) => card.id === lastBlocker);
    return blocker ? { ...interaction, headline: `Which attacker does ${blocker.name} block?` } : interaction;
  }, [interaction, prompt, board.me, lastBlocker, damageSplit, payment]);
  const pregame = !view?.step;
  const handIds = useMemo(() => new Set(hand.map((card) => card.id!)), [hand]);
  // the server names the pre-game choices (who starts, the London mulligan's bottom cards); older servers are recognized
  // by what can be chosen
  const pregamePick = useMemo(
    () => (interaction.mode === 'target' ? pregameChoice(prompt, { pregame, clickable, handIds, playerIds: board.players }) : null),
    [interaction.mode, prompt, pregame, clickable, handIds, board.players],
  );
  // London mulligan: choose cards from the opening hand to put on the bottom
  const choosingFromHand = pregamePick === 'mulliganBottom';
  // a target among graveyard or exiled cards (the server sends only their ids): those zones aren't on the board
  const offBoard = useMemo(
    () => (interaction.mode === 'target' && !pregamePick ? offBoardTargets(view, clickable) : null),
    [interaction.mode, pregamePick, view, clickable],
  );
  const pickerCards = interaction.mode === 'pickCards' && prompt?.kind === 'target' && prompt.cards
    ? Object.values(prompt.cards)
    : choosingFromHand ? hand : offBoard;
  const canAct = mode === 'play' && !state.gameOver;
  // several opponents: the server asks what each attacker attacks
  const defenders = useMemo(() => (canAct && !awaitingServer ? defenderChoice(prompt, view) : null), [canAct, awaitingServer, prompt, view]);
  const attackerName = useMemo(() => {
    const forced = /\(([^)]+)\)\s*$/.exec(prompt?.text ?? '')?.[1];
    if (forced) return forced;
    // the creature the question is about: the one just clicked, or one put onto the battlefield attacking
    return defenders ? askedAttacker(view, myId, lastAttacker)?.name ?? null : null;
  }, [prompt, defenders, view, myId, lastAttacker]);
  // a yes/no question about cards an effect just looked at ("you may put a land from among them..."): show them
  const askCards = useMemo(() => (canAct ? askedAboutCards(prompt, view)?.cards ?? null : null), [canAct, prompt, view]);
  // announcing X: what the player could pay, when the server sets no limit
  const announceLimit = useMemo(() => (prompt?.kind === 'amount' ? xLimit(prompt, view, myId) : null), [prompt, view, myId]);
  // a concession takes effect when the player holding priority passes: name them while it is pending
  const priorityName = (view?.players ?? []).find((player) => player.hasPriority && player.playerId !== myId)?.name ?? null;
  const choosingStarter = pregamePick === 'startingPlayer';
  // one decision, one set of controls: an overlay owns the choice while it's open
  const overlayOpen = (interaction.mode === 'mulligan' || choosingStarter || !!pickerCards || interaction.mode === 'panel') && !awaitingServer;
  const handHidden = (interaction.mode === 'mulligan' || choosingStarter || !!pickerCards) && !awaitingServer;
  // a choice among cards in hand (discard, reveal...): eligible cards take the decision edge
  const choosingInHand = interaction.mode === 'target' && !pickerCards && [...clickable].some((id) => handIds.has(id));
  const opponentIds = useMemo(() => board.opponents.map((opponent) => opponent.player.playerId!), [board.opponents]);
  const matCloth = useSettings((state) => state.settings.matCloth);
  const matArt = useSettings((state) => state.settings.matArt);
  const matCard = useSettings((state) => state.settings.matCard);
  const matStyle = useMemo(() => clothStyle(clothOf(matCloth)), [matCloth]);
  const myArt = matArt === 'none' ? null : matArt === 'card' ? matCard : sleeves.cover;
  const opponentArt = useMemo(() => {
    const permanent = board.opponents[0]?.front[0]?.lead ?? board.opponents[0]?.back.find((group) => !(group.lead.cardTypes ?? []).includes('LAND'))?.lead;
    return permanent?.expansionSetCode && permanent.cardNumber
      ? { setCode: permanent.expansionSetCode, cardNumber: permanent.cardNumber, name: permanent.name }
      : null;
  }, [board.opponents]);

  return (
    <Stage style={matStyle}>
      {(stage) => {
        // the stage widens with the window and the field takes the extra width; a portrait window gets the tall layout
        const layout = matchLayout(stage);
        return (
          <div className={styles.layout} style={{ ['--seam' as string]: `${layout.seam}px`, ['--hand-raise' as string]: `${layout.handRaise}px` }}>
            <div className={styles.mat} aria-hidden="true" />
            {/* each half carries its player's deck art, printed faintly into the mat */}
            <div className={styles.printTheirs}><MatPrint card={opponentArt} /></div>
            <div className={styles.printMine}><MatPrint card={myArt} /></div>
            <div className={styles.seam} aria-hidden="true" />
            {/* under the hand and the controls, over the mat */}
            <Stitch inset={10} radius={22} zIndex={2} />

            {board.opponents.map((opponent, index) => (
              <OpponentSide
                key={opponent.player.playerId}
                board={opponent}
                index={index}
                count={board.opponents.length}
                sleeve={sleeves.theirs}
                handCount={opponent.player.handCount ?? 0}
                clickable={clickable}
                selected={interaction.selected}
                quiet={EMPTY}
                attacking={attacking}
                targeting={targeting}
                blocking={blocking}
                onClick={onClick}
                deciding={!!opponent.player.hasPriority && interaction.mode === 'waiting'}
                layout={layout}
                commanders={commanders.get(opponent.player.playerId!) ?? NO_COMMANDERS}
                commanderDamage={damageTaken(opponent.player)}
                outOfRange={!inRange(view, myId, opponent.player.playerId!)}
              />
            ))}

            {board.me && (
              <>
                <Battlefield board={board.me} sleeve={sleeves.mine} clickable={clickable} selected={interaction.selected} quiet={quiet} attacking={attacking} blocking={blocking} targeting={targeting} onClick={onClick} onBlockDrop={interaction.mode === 'declareBlockers' && canAct ? onBlockDrop : undefined} place={layout.me} />
                <div className={styles.myPlate} style={layout.myPlate}>
                  <PlayerPlate
                    player={board.me.player}
                    isMe={board.me.isMe}
                    sleeve={sleeves.mine}
                    targetable={clickable.has(board.me.player.playerId!)}
                    selected={interaction.selected.has(board.me.player.playerId!)}
                    deciding={canAct && interaction.mode !== 'waiting'}
                    onClick={() => onClick(board.me!.player.playerId!)}
                    toPay={canAct && !awaitingServer ? payment?.cost : null}
                    payable={canAct ? clickable : undefined}
                    onPay={onClick}
                    commanderDamage={damageTaken(board.me.player)}
                  />
                </div>
                <div className={styles.myPiles} style={layout.myPiles}>
                  <Piles
                    player={board.me.player}
                    sleeve={sleeves.mine}
                    isMe={board.me.isMe}
                    commanders={myId ? commanders.get(myId) : undefined}
                    clickable={canAct ? clickable : undefined}
                    onCast={onClick}
                  />
                </div>
              </>
            )}

            <Reveals view={view} myPlayerId={myId} opponentIds={opponentIds} />
            <StackZone items={stack} clickable={clickable} selected={interaction.selected} sleeveOf={sleeveOf} onClick={onClick} originOf={originOf} />
            <PhaseLadder
              step={view?.step}
              myTurn={!spectating && !!myId && view?.activePlayerId === myId}
              turn={view?.turn ?? 0}
              started={!!view?.activePlayerId && !!view?.step}
              activeName={spectating ? view?.activePlayerName ?? null : null}
            />

            {board.me && seatHand && (
              <Hand
                cards={seatHand.cards}
                faceDown={seatHand.faceDown}
                label={`${board.me.player.name ?? 'Player'}'s hand`}
                clickable={EMPTY}
                selected={EMPTY}
                sleeve={sleeves.mine}
                onPlay={ignore}
                playLine={0}
                libraryOrigin={`library:${board.me.player.playerId}`}
              />
            )}
            {board.me && !handHidden && mode === 'play' && (
              <Hand
                cards={hand}
                choosing={choosingInHand}
                clickable={pickerCards ? EMPTY : clickable}
                selected={interaction.selected}
                sleeve={sleeves.mine}
                onPlay={onClick}
                playLine={layout.playLine}
                libraryOrigin={`library:${board.me.player.playerId}`}
              />
            )}

            {mode === 'play' ? (
              !overlayOpen && <ActionCluster
                interaction={cornerInteraction}
                awaiting={awaitingServer}
                status={state.status}
                canAct={canAct}
                special={!!view?.special}
                holdingPriority={holding}
                autoPassing={!!autoPassing}
                extra={prompt?.kind === 'ask' ? <AlwaysAnswerMenu gameId={state.gameId} prompt={prompt} onCommand={onCommand} /> : undefined}
                fullControl={fullControl}
                onFullControl={setFullControl}
                stalled={state.stalled}
                conceding={state.conceding ? { waitingOn: priorityName } : null}
                cards={askCards}
                onResend={onResend}
                onResync={onResync}
                onCommand={onCommand}
              />
            ) : (
              <SpectatorBar session={session} state={state} onLeave={leave} />
            )}

            {damageSplit && canAct && !awaitingServer && <DamageAssigner split={damageSplit} />}
            {defenders && <DefenderPicker attacker={attackerName} options={defenders} onPick={onClick} />}
            <Vfx view={view} myPlayerId={myId} />
            <Arrows sourceId={arrowSource} targetIds={arrowTargets} live={choosingTargets} links={links} attacks={attacks} />
            <EmoteBubbles view={view} />
            {mode === 'play' && <Coach view={view ?? null} mode={choosingStarter ? 'panel' : interaction.mode} awaiting={awaitingServer} gameOver={!!state.gameOver} />}
            {mode === 'play' && <LessonCoach gameId={state.gameId} view={view ?? null} mode={choosingStarter ? 'panel' : interaction.mode} awaiting={awaitingServer} gameOver={!!state.gameOver} />}
            <GameLog gameId={state.gameId} notices={state.notices} canChat={mode !== 'replay'} view={view} replayLog={replayLog} delayMs={state.broadcastDelayMs} />
            {mode === 'watch' && !view && state.broadcastDelayMs > 0 && (
              <p className={styles.delayNote} role="status">
                The table appears in {delayLabel(Math.round(state.broadcastDelayMs / 1000))}: the broadcast delay holds the game back.
              </p>
            )}
            {mode === 'play' && <WatcherCount watchers={watchers} className={styles.watchers} />}
            <GameMenu
              gameId={state.gameId}
              practice={mode === 'play' && !state.gameOver && !career && board.opponents.length > 0 && board.opponents.every((opponent) => opponent.player.isHuman === false)}
              solo={mode === 'play' && !state.gameOver && !career && !eventId && vsAi && (view?.turn ?? 0) > 0}
              careerAssists={mode === 'play' && !state.gameOver && career && !eventId && vsAi && (view?.turn ?? 0) > 0}
              vsAi={vsAi}
              onSaved={() => exit(false)}
              canConcede={canAct}
              onConcede={() => onCommand({ type: 'action', action: 'CONCEDE' })}
              onLeave={leave}
              rollback={canAct && !!view?.rollbackTurnsAllowed && (view?.turn ?? 0) > 0
                ? { ready: interaction.mode === 'priority' && !awaitingServer, request: () => onCommand({ type: 'action', action: 'ROLLBACK_TURNS', data: 0 }) }
                : null}
            />

            {interaction.mode === 'mulligan' && !awaitingServer && (
              <MulliganOverlay hand={hand} interaction={interaction} sleeve={sleeves.mine} onCommand={onCommand} />
            )}
            {choosingStarter && board.me && !awaitingServer && (
              <StartingPlayerOverlay
                me={{ id: board.me.player.playerId!, name: board.me.player.name ?? 'You' }}
                opponents={board.opponents.map((opponent) => ({ id: opponent.player.playerId!, name: opponent.player.name ?? 'Opponent' }))}
                onChoose={onClick}
              />
            )}
            {pickerCards && !awaitingServer && (
              <CardPicker
                title={choosingFromHand ? 'Put cards on the bottom' : 'Choose cards'}
                cards={pickerCards}
                interaction={interaction}
                sleeve={sleeves.mine}
                footer={prompt ? <TriggerOrderOptions gameId={state.gameId} prompt={prompt} onCommand={onCommand} /> : undefined}
                onCommand={onCommand}
              />
            )}
            {interaction.mode === 'panel' && prompt && !awaitingServer && <ChoicePanel prompt={prompt} onCommand={onCommand} xLimit={announceLimit} />}
            {viewer && <ZoneViewer title={viewer.title} cards={viewer.cards} onClose={() => openViewer(null)} />}
            {state.gameOver && betweenGames && (
              <BetweenGames
                progress={betweenGames}
                won={state.endInfo?.won ? true : /\bdraw\b/i.test(state.endInfo?.gameInfo ?? '') ? null : false}
                onLeave={leave}
                leaveLabel={eventId ? 'Back to the event' : 'Leave match'}
                concedes={!eventId}
              />
            )}
            {state.gameOver && !betweenGames && (
              <GameOverOverlay
                message={state.gameOver}
                endInfo={state.endInfo}
                myName={board.me?.player.name ?? null}
                onLeave={leave}
                leaveLabel={eventId ? 'Back to the event' : career ? 'Back to Career' : vsPeople ? t('match.end.backToTables') : 'Back to Play'}
                // a rematch is only dealt against the computer; people go back to Tables
                onPlayAgain={mode === 'play' && deckId && !eventId && vsAi ? playAgain : undefined}
                onReview={mode === 'play' && vsAi && board.me ? () => setReviewing(true) : undefined}
              />
            )}
            {reviewing && (
              <Dialog open onOpenChange={(open) => !open && setReviewing(false)} title="Game review" width="lg">
                <GameReview gameId={state.gameId} me={board.me?.player.name ?? null} onNavigate={() => setReviewing(false)} />
              </Dialog>
            )}
            <CardZoom large={spectating && largeZoom} />
            <CardDetail view={view} extra={pickerCards ?? viewer?.cards ?? NO_CARDS} sleeveOf={sleeveOf} attacking={attacking} blocking={blocking} />
          </div>
        );
      }}
    </Stage>
  );
}

interface RowProps {
  sleeve: string;
  clickable: ReadonlySet<string>;
  selected: ReadonlySet<string>;
  quiet: ReadonlySet<string>;
  attacking: ReadonlySet<string>;
  blocking: ReadonlySet<string>;
  targeting: boolean;
  onClick(id: string): void;
  onBlockDrop?(blockerId: string, attackerId: string): boolean;
}

/** One player's two rows of permanents. Rows shrink their cards to fit rather than wrapping. */
const Battlefield = memo(function Battlefield({ board, place, ...row }: RowProps & {
  board: PlayerBoard;
  place: RowsPlacement;
}) {
  const forward = board.isMe ? -1 : 1;
  const { left, width } = place;
  return (
    <>
      <Row groups={board.front} left={left} width={width} top={place.frontTop} ideal={place.frontIdeal} min={58} forward={forward} label="Creatures" flip={!board.isMe} {...row} />
      <Row groups={board.back} left={left} width={width} top={place.backTop} ideal={place.backIdeal} min={50} forward={forward} label="Lands & permanents" flip={!board.isMe} {...row} />
    </>
  );
});

function Row({ groups, left, width, top, ideal, min, forward, label, flip, ...rest }: RowProps & {
  groups: PermanentGroup[];
  left: number;
  width: number;
  top: number;
  ideal: number;
  min: number;
  forward: 1 | -1;
  /** printed into the zone outline */
  label: string;
  /** the opponent's zones read from their side: label at the top edge */
  flip: boolean;
}) {
  const cardWidth = fitCardWidth(groups, width - 24, ideal, min);
  const height = ideal * (88 / 63);
  return (
    <div
      className={[styles.row, flip ? styles.rowFlip : ''].join(' ')}
      style={{ left, top, width, height, gap: cardWidth * 0.12 }}
      data-label={label}
    >
      {groups.map((group) => (
        <PermanentStack key={group.key} group={group} width={cardWidth} forward={forward} {...rest} />
      ))}
    </div>
  );
}

/** An opponent's half: their rows mirrored above the seam, their seat, piles and hidden hand. */
function OpponentSide({ board, index, count, sleeve, handCount, deciding, layout, commanders, commanderDamage, outOfRange, ...row }: RowProps & {
  layout: MatchLayout;
  board: PlayerBoard;
  index: number;
  count: number;
  handCount: number;
  deciding: boolean;
  commanders: readonly CommanderStatus[];
  commanderDamage: readonly { name: string; amount: number }[];
  outOfRange: boolean;
}) {
  const place = layout.them(index, count);
  const player = board.player;
  return (
    <>
      <Battlefield board={board} sleeve={sleeve} place={place} {...row} />
      <div className={[styles.theirPlate, count > 1 ? styles.seat : ''].join(' ')} style={layout.theirPlate(index, count)}>
        <PlayerPlate
          player={player}
          isMe={false}
          sleeve={sleeve}
          targetable={row.clickable.has(player.playerId!)}
          selected={row.selected.has(player.playerId!)}
          deciding={deciding}
          onClick={() => row.onClick(player.playerId!)}
          commanderDamage={commanderDamage}
          outOfRange={outOfRange}
        />
        {count > 1 && (
          <div className={styles.theirMini}>
            <MiniPiles player={player} commanders={commanders} />
          </div>
        )}
      </div>
      {count === 1 && (
        <div className={styles.theirPiles} style={layout.theirPiles}>
          <Piles player={player} sleeve={sleeve} isMe={false} commanders={commanders} />
        </div>
      )}
      {count > 1 && !layout.tall && <span className={styles.seatName} style={{ left: place.left + 16 }} aria-hidden="true">{player.name}</span>}
      <HiddenHand playerId={player.playerId!} count={handCount} sleeve={sleeve} left={layout.hiddenHandX(index, count)} />
    </>
  );
}

/** The backs of an opponent's hand peeking over the top edge; their spells fly out of it. */
function HiddenHand({ playerId, count, sleeve, left }: { playerId: string; count: number; sleeve: string; left: number }) {
  const ref = useRef<HTMLDivElement>(null);
  useFlipOrigin(`hand:${playerId}`, ref);
  const shown = Math.min(count, 10);
  return (
    <div ref={ref} className={styles.hiddenHand} style={{ left }} aria-label={`${count} cards in hand`} role="img">
      {Array.from({ length: shown }, (_, i) => (
        <div key={i} className={styles.hiddenCard} style={{ ['--i' as string]: i - (shown - 1) / 2 }}>
          <CardFace card={{}} hidden sleeve={sleeve} size="small" />
        </div>
      ))}
      {count > 0 && <span className={styles.handCount}>{count}</span>}
    </div>
  );
}

/** Asking to roll the game back to the start of this turn: only while holding priority (the server's rule). */
interface RollbackOption {
  ready: boolean;
  request(): void;
}

const CONFIRM = {
  concede: {
    title: 'Concede this game?',
    description: 'Your opponent wins this game. You can\'t undo it.',
    action: 'Concede',
  },
  leave: {
    title: 'Leave this match?',
    description: 'You concede this game and the rest of the match. You can\'t undo it.',
    action: 'Leave match',
  },
  fork: {
    title: 'Try this turn again?',
    description: 'A new game starts from the start of this turn, with the same cards in the same places and the same draws to come. You leave this one.',
    action: 'Start the turn again',
  },
  save: {
    title: 'Save and leave?',
    description: 'The game is kept as it was at the start of this turn. Resume it from Home whenever you like.',
    action: 'Save and leave',
  },
  rollback: {
    title: 'Go back to the start of this turn?',
    description: 'Every other player must agree; a game against the computer goes back at once.',
    action: 'Ask to go back',
  },
  rollbackAi: {
    title: 'Take back this turn?',
    description: 'The game goes back to the start of this turn at once. Against the computer you can take back as often as you like.',
    action: 'Take it back',
  },
  takeBack: {
    title: 'Take back this turn?',
    description: 'A new game starts from the start of this turn, with the same cards and the same draws to come. You get one take-back a Career game, and the game then pays half and counts for no quests.',
    action: 'Take it back',
  },
} as const;

function GameMenu({ gameId, practice, solo, careerAssists, vsAi, onSaved, canConcede, onConcede, onLeave, rollback }: {
  gameId: string;
  /** a Career game against the computer, past the first turn: counted hints and one take-back, where the mode allows */
  careerAssists: boolean;
  /** every opponent is the computer */
  vsAi: boolean;
  /** a game against the computer only: the practice tools are on offer */
  practice: boolean;
  /** free play against the computer, past the first turn: advice, a fresh try at the turn, save and leave */
  solo: boolean;
  onSaved(): void;
  canConcede: boolean;
  onConcede(): void;
  onLeave(): void;
  rollback: RollbackOption | null;
}) {
  // conceding gives up this game; leaving a game in progress gives up the whole match
  const [confirming, setConfirming] = useState<keyof typeof CONFIRM | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [practiceOpen, setPracticeOpen] = useState(false);
  const [adviceOpen, setAdviceOpen] = useState(false);
  const [whatIfOpen, setWhatIfOpen] = useState(false);
  const copy = confirming ? CONFIRM[confirming] : null;
  const assists = useSoloAssists(gameId, careerAssists).data;
  async function soloAction(choice: 'fork' | 'save' | 'takeBack') {
    try {
      if (choice === 'takeBack') {
        await tryTurnAgain(gameId, true);
        notify('Take back', 'A new game starts from the start of this turn.');
      } else if (choice === 'fork') {
        await tryTurnAgain(gameId);
        notify('Try again', 'A new game starts from the start of this turn.');
      } else {
        await saveAndLeave(gameId);
        notify('Saved', 'Resume the game from Home.');
        onSaved();
      }
    } catch (failure) {
      notify(choice === 'fork' ? 'Try again' : choice === 'takeBack' ? 'Take back' : 'Save', failure instanceof Error ? failure.message : String(failure));
    }
  }
  return (
    <div className={styles.menu}>
      <DropdownMenu.Root>
        <DropdownMenu.Trigger className={styles.menuButton} aria-label="Game menu">
          <Menu size={24} />
        </DropdownMenu.Trigger>
        <DropdownMenu.Portal>
          <DropdownMenu.Content className={styles.menuContent} align="end" sideOffset={8}>
            <DropdownMenu.Item className={styles.menuItem} onSelect={() => setSettingsOpen(true)}>Settings</DropdownMenu.Item>
            {practice && (
              <DropdownMenu.Item className={styles.menuItem} onSelect={() => setPracticeOpen(true)}>Practice tools</DropdownMenu.Item>
            )}
            {practice && (
              <DropdownMenu.Item className={styles.menuItem} onSelect={() => setAdviceOpen(true)}>Ask the AI</DropdownMenu.Item>
            )}
            {careerAssists && assists?.hints && (
              <DropdownMenu.Item className={styles.menuItem} onSelect={() => setAdviceOpen(true)}>
                Ask for a hint ({assists.hintsLeft ?? 0} left)
              </DropdownMenu.Item>
            )}
            {careerAssists && assists?.takeBack && (
              <DropdownMenu.Item className={styles.menuItem} onSelect={() => setConfirming('takeBack')}>Take back this turn</DropdownMenu.Item>
            )}
            {solo && (
              <DropdownMenu.Item className={styles.menuItem} onSelect={() => setConfirming('fork')}>Try this turn again</DropdownMenu.Item>
            )}
            {solo && (
              <DropdownMenu.Item className={styles.menuItem} onSelect={() => setWhatIfOpen(true)}>Play an earlier turn…</DropdownMenu.Item>
            )}
            {solo && (
              <DropdownMenu.Item className={styles.menuItem} onSelect={() => setConfirming('save')}>Save and leave</DropdownMenu.Item>
            )}
            {rollback && (
              <DropdownMenu.Item
                className={styles.menuItem}
                disabled={!rollback.ready}
                title={rollback.ready ? undefined : 'Available while you have priority'}
                onSelect={() => setConfirming(vsAi ? 'rollbackAi' : 'rollback')}
              >
                {vsAi ? 'Take back this turn' : 'Request rollback to start of turn'}
              </DropdownMenu.Item>
            )}
            {canConcede && (
              <DropdownMenu.Item className={styles.menuItem} onSelect={() => setConfirming('concede')}>Concede</DropdownMenu.Item>
            )}
            <DropdownMenu.Item className={styles.menuItem} onSelect={() => (canConcede ? setConfirming('leave') : onLeave())}>
              Leave the game
            </DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Portal>
      </DropdownMenu.Root>
      <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
      {practice && <PracticeTools gameId={gameId} open={practiceOpen} onOpenChange={setPracticeOpen} />}
      {practice && <AdviceDialog gameId={gameId} open={adviceOpen} onOpenChange={setAdviceOpen} />}
      {solo && <WhatIfDialog gameId={gameId} open={whatIfOpen} onOpenChange={setWhatIfOpen} />}
      {careerAssists && !practice && (
        <AdviceDialog
          gameId={gameId}
          open={adviceOpen}
          onOpenChange={setAdviceOpen}
          counted={{ hintsLeft: assists?.hintsLeft ?? 0 }}
          onUsed={() => refreshAssists(gameId)}
        />
      )}
      <Dialog
        open={confirming !== null}
        onOpenChange={(open) => !open && setConfirming(null)}
        title={copy?.title ?? ''}
        description={copy?.description}
        width="sm"
        footer={(
          <>
            <Button variant="quiet" onClick={() => setConfirming(null)}>Keep playing</Button>
            <Button
              variant={confirming === 'concede' || confirming === 'leave' ? 'danger' : 'decision'}
              onClick={() => {
                const choice = confirming;
                setConfirming(null);
                if (choice === 'leave') onLeave();
                else if (choice === 'concede') onConcede();
                else if (choice === 'rollback' || choice === 'rollbackAi') rollback?.request();
                else if (choice === 'fork' || choice === 'save' || choice === 'takeBack') void soloAction(choice);
              }}
            >
              {copy?.action}
            </Button>
          </>
        )}
      >
        {null}
      </Dialog>
    </div>
  );
}
