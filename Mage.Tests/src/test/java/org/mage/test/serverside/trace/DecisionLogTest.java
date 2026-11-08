package org.mage.test.serverside.trace;

import mage.cards.decks.DeckCardLists;
import mage.cards.repository.CardScanner;
import mage.constants.MultiplayerAttackOption;
import mage.constants.PhaseStep;
import mage.constants.RangeOfInfluence;
import mage.game.Game;
import mage.game.GameOptions;
import mage.game.TwoPlayerDuel;
import mage.game.TwoPlayerMatch;
import mage.game.match.Match;
import mage.game.match.MatchOptions;
import mage.game.mulligan.MulliganType;
import mage.game.trace.DecisionLog;
import mage.game.trace.GameTrace;
import mage.player.human.HumanPlayer;
import mage.players.net.UserData;
import mage.util.RandomUtil;
import mage.util.ThreadUtils;
import org.junit.Assert;
import org.junit.Before;
import org.junit.Test;

import java.util.List;
import java.util.Random;
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
            RandomDriver driver = new RandomDriver(game, answers, new Random(SEED));
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
}
