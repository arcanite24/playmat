package org.mage.test.serverside.trace;

import mage.abilities.ActivatedAbility;
import mage.abilities.PlayLandAbility;
import mage.cards.decks.DeckCardLists;
import mage.cards.repository.CardScanner;
import mage.constants.MultiplayerAttackOption;
import mage.constants.PhaseStep;
import mage.constants.RangeOfInfluence;
import mage.constants.Zone;
import mage.game.Game;
import mage.game.GameOptions;
import mage.game.TwoPlayerDuel;
import mage.game.TwoPlayerMatch;
import mage.game.events.PlayerQueryEvent;
import mage.game.match.Match;
import mage.game.match.MatchOptions;
import mage.game.mulligan.MulliganType;
import mage.game.permanent.Permanent;
import mage.game.trace.DecisionLog;
import mage.game.trace.GameTrace;
import mage.player.human.HumanPlayer;
import mage.players.Player;
import mage.players.net.UserData;
import mage.util.RandomUtil;
import mage.util.ThreadUtils;
import org.junit.Assert;
import org.junit.Before;
import org.junit.Test;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.UUID;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
import java.util.stream.Collectors;

/**
 * Decision logs (Rust migration phase 0): two human players answer through the same calls a client makes, picked by a
 * seeded driver (spells, targets, attacks, blocks, mana). The answers are kept; the same decks, seed and answers then
 * play the game again with no driver, twice, and each replay gives the recorded trace.
 */
public class DecisionLogTest {

    private static final long GAME_TIMEOUT_MS = 5 * 60 * 1000L;
    private static final long SEED = 99L;
    private static final int TURNS = 14;

    @Before
    public void setUp() {
        CardScanner.scan();
    }

    @Test
    public void recordedAnswersReplayToTheSameTrace() throws Exception {
        DecisionLog log = DecisionLog.recording();
        GameTrace recorded = play(log, true);
        List<DecisionLog.Decision> decisions = log.getDecisions();
        Assert.assertTrue("the game was traced: " + recorded.getEntries().size(), recorded.getEntries().size() > 50);
        Assert.assertTrue("the players answered: " + decisions.size(), decisions.size() > 30);
        Assert.assertTrue("every answer names objects of the game: "
                        + decisions.stream().filter(d -> d.target != null && d.target.startsWith("?")).collect(Collectors.toList()),
                decisions.stream().noneMatch(d -> d.target != null && d.target.startsWith("?")));
        Assert.assertTrue("spells and abilities were played",
                decisions.stream().anyMatch(d -> d.target != null && d.target.startsWith("H")));

        Assert.assertTrue("creatures attacked", decisions.stream().anyMatch(d -> "special".equals(d.string)));
        System.out.println("[decision log] " + decisions.size() + " answers, " + recorded.getEntries().size() + " trace entries");
        for (String run : new String[]{"first", "second"}) {
            DecisionLog replay = DecisionLog.replaying(decisions);
            GameTrace again = play(replay, false);
            Assert.assertNull(run + " replay: " + replay.getFailure(), replay.getFailure());
            Assert.assertEquals(run + " replay used every answer", decisions.size(), replay.used());
            int diff = GameTrace.firstDifference(recorded.hashes(), again.hashes());
            if (diff >= 0) {
                String a = diff < recorded.getEntries().size() ? recorded.getEntries().get(diff).text : "(ended)";
                String b = diff < again.getEntries().size() ? again.getEntries().get(diff).text : "(ended)";
                Assert.fail(run + " replay parts at entry " + diff + "\n--- recorded\n" + a + "\n--- replayed\n" + b);
            }
        }
    }

    private GameTrace play(DecisionLog log, boolean drive) throws Exception {
        GameTrace trace = new GameTrace(true, null);
        Game game = new TwoPlayerDuel(MultiplayerAttackOption.LEFT, RangeOfInfluence.ALL,
                MulliganType.GAME_DEFAULT.getMulligan(0), 60, 20, 7);
        Match match = new TwoPlayerMatch(new MatchOptions("decision log test", "Two Player Duel", false));
        HumanPlayer one = new HumanPlayer("One", RangeOfInfluence.ALL, 1);
        HumanPlayer two = new HumanPlayer("Two", RangeOfInfluence.ALL, 1);
        // a client's default settings
        one.setUserData(UserData.getDefaultUserDataView());
        two.setUserData(UserData.getDefaultUserDataView());
        GameTraceTest.seat(game, match, one, deck("Grizzly Bears", "Giant Growth", "Forest"));
        GameTraceTest.seat(game, match, two, deck("Hill Giant", "Shock", "Mountain"));
        GameOptions options = new GameOptions();
        options.testMode = false;
        options.rollbackTurnsAllowed = false;
        options.stopOnTurn = TURNS;
        options.stopAtStep = PhaseStep.UNTAP;
        options.seed = SEED;
        options.trace = trace;
        options.decisions = log;
        game.setGameOptions(options);

        ExecutorService answers = Executors.newSingleThreadExecutor();
        if (drive) {
            Driver driver = new Driver(game, answers, new Random(SEED));
            game.addPlayerQueryEventListener(driver::ask);
        }
        AtomicReference<Throwable> failure = new AtomicReference<>();
        Thread thread = new Thread(() -> {
            try {
                RandomUtil.withRandom(new Random(SEED), () -> {
                    game.start(one.getId());
                    return null;
                });
            } catch (Throwable e) {
                failure.set(e);
            }
        }, ThreadUtils.THREAD_PREFIX_GAME + " decision log test");
        thread.start();
        thread.join(GAME_TIMEOUT_MS);
        answers.shutdownNow();
        answers.awaitTermination(5, TimeUnit.SECONDS);
        if (thread.isAlive()) {
            game.setGameStopped(true);
            thread.interrupt();
            Assert.fail("the game didn't finish in time");
        }
        if (failure.get() != null) {
            throw new AssertionError("the game failed", failure.get());
        }
        return trace;
    }

    private static DeckCardLists deck(String creature, String spell, String land) {
        DeckCardLists list = new DeckCardLists();
        list.getCards().add(GameTraceTest.info(creature, 20));
        list.getCards().add(GameTraceTest.info(spell, 12));
        list.getCards().add(GameTraceTest.info(land, 22));
        list.getCards().add(GameTraceTest.info("Lightning Bolt", 6));
        return list;
    }

    /**
     * Answers each question as a client would, from another thread, after the game thread opens the answer. What to
     * answer is picked on the game thread, where the question is asked, so it sees the game as the question does.
     */
    private static final class Driver {
        private final Game game;
        private final ExecutorService answers;
        private final Random random;
        /** tries per priority window or payment, so a spell that can't be cast or paid is given up on */
        private final Map<String, Integer> tries = new HashMap<>();

        Driver(Game game, ExecutorService answers, Random random) {
            this.game = game;
            this.answers = answers;
            this.random = random;
        }

        void ask(PlayerQueryEvent event) {
            Player player = game.getPlayer(event.getPlayerId());
            if (!(player instanceof HumanPlayer)) {
                return;
            }
            HumanPlayer human = (HumanPlayer) player;
            String message = event.getMessage() == null ? "" : event.getMessage();
            // a window ends when something changes: then every kind of answer gets its tries again
            String window = game.getTurnNum() + "/" + game.getTurnStepType() + "/" + game.getStack().size() + "/"
                    + game.getBattlefield().count(mage.filter.StaticFilters.FILTER_PERMANENT, player.getId(), null, game) + "/"
                    + player.getHand().size() + "/" + player.getId();
            switch (event.getQueryType()) {
                case ASK:
                    answer(human, null, !message.contains("ulligan") && random.nextInt(3) > 0, null, null);
                    break;
                case SELECT:
                    if (message.startsWith("Play")) {
                        List<UUID> sources = new ArrayList<>();
                        List<UUID> lands = new ArrayList<>();
                        for (ActivatedAbility ability : player.getPlayable(game, true)) {
                            if (ability instanceof PlayLandAbility) {
                                lands.add(ability.getSourceId());
                            } else if (!ability.isManaAbility() && !sources.contains(ability.getSourceId())) {
                                sources.add(ability.getSourceId());
                            }
                        }
                        int tried = tries.merge("play " + window, 1, Integer::sum);
                        if (!lands.isEmpty() && tried == 1) {
                            answer(human, pick(lands), null, null, null);
                        } else if (!sources.isEmpty() && tried <= 3 && random.nextInt(10) < 8) {
                            answer(human, pick(sources), null, null, null);
                        } else {
                            answer(human, null, false, null, null);
                        }
                    } else if (message.startsWith("Select attackers")) {
                        boolean none = game.getCombat().getAttackers().isEmpty();
                        if (none && random.nextInt(3) > 0) {
                            answer(human, null, null, null, "special");
                        } else {
                            answer(human, null, true, null, null);
                        }
                    } else if (message.startsWith("Select blockers")) {
                        List<UUID> blockers = game.getBattlefield().getAllActivePermanents(player.getId()).stream()
                                .filter(p -> p.isCreature(game) && !p.isTapped() && p.getBlocking() == 0)
                                .map(Permanent::getId).collect(Collectors.toList());
                        if (!blockers.isEmpty() && tries.merge("block " + window, 1, Integer::sum) <= 1 && random.nextBoolean()) {
                            answer(human, pick(blockers), null, null, null);
                        } else {
                            answer(human, null, true, null, null);
                        }
                    } else {
                        answer(human, null, true, null, null);
                    }
                    break;
                case PICK_TARGET: {
                    List<UUID> candidates = new ArrayList<>();
                    if (event.getTargets() != null) candidates.addAll(event.getTargets());
                    if (event.getCards() != null) candidates.addAll(event.getCards());
                    if (event.getPerms() != null) event.getPerms().forEach(p -> candidates.add(p.getId()));
                    if (!candidates.isEmpty() && tries.merge("target " + window, 1, Integer::sum) <= 3) {
                        answer(human, pick(candidates), null, null, null);
                    } else {
                        answer(human, null, true, null, null);
                    }
                    break;
                }
                case CHOOSE_ABILITY:
                case PICK_ABILITY:
                    if (event.getAbilities() != null && !event.getAbilities().isEmpty()) {
                        answer(human, event.getAbilities().get(random.nextInt(event.getAbilities().size())).getId(), null, null, null);
                    } else {
                        answer(human, null, false, null, null);
                    }
                    break;
                case CHOOSE_MODE:
                    answer(human, pick(new ArrayList<>(event.getModes().keySet())), null, null, null);
                    break;
                case CHOOSE_CHOICE: {
                    List<String> choices = event.getChoice().isKeyChoice()
                            ? new ArrayList<>(event.getChoice().getKeyChoices().keySet())
                            : new ArrayList<>(event.getChoice().getChoices());
                    choices.sort(null);
                    answer(human, null, choices.isEmpty() ? Boolean.FALSE : null, null,
                            choices.isEmpty() ? null : choices.get(random.nextInt(choices.size())));
                    break;
                }
                case PLAY_MANA: {
                    List<UUID> sources = game.getBattlefield().getAllActivePermanents(player.getId()).stream()
                            .filter(p -> !p.isTapped() && !p.getAbilities().getActivatedManaAbilities(Zone.BATTLEFIELD).isEmpty())
                            .map(Permanent::getId).collect(Collectors.toList());
                    if (!sources.isEmpty() && tries.merge("mana " + window, 1, Integer::sum) <= 8) {
                        answer(human, pick(sources), null, null, null);
                    } else {
                        answer(human, null, false, null, null);
                    }
                    break;
                }
                case AMOUNT:
                    answer(human, null, null, event.getMin() + random.nextInt(Math.max(1, Math.min(20, event.getMax() - event.getMin() + 1))), null);
                    break;
                case PLAY_X_MANA:
                case CHOOSE_PILE:
                case MULTI_AMOUNT:
                    answer(human, null, true, null, null);
                    break;
                default:
                    // messages, drafts and deck building: nothing to answer
            }
        }

        private UUID pick(List<UUID> ids) {
            return ids.get(random.nextInt(ids.size()));
        }

        private void answer(HumanPlayer player, UUID id, Boolean bool, Integer integer, String string) {
            answers.execute(() -> {
                if (id != null) {
                    player.setResponseUUID(id);
                } else if (string != null) {
                    player.setResponseString(string);
                } else if (integer != null) {
                    player.setResponseInteger(integer);
                } else {
                    player.setResponseBoolean(bool);
                }
            });
        }
    }
}
