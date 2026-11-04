package org.mage.test.serverside.trace;

import mage.cards.decks.Deck;
import mage.cards.decks.DeckCardInfo;
import mage.cards.decks.DeckCardLists;
import mage.cards.repository.CardInfo;
import mage.cards.repository.CardRepository;
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
import mage.game.trace.GameTrace;
import mage.game.trace.StateFingerprint;
import mage.player.ai.ComputerPlayerControllableProxy;
import mage.players.Player;
import mage.util.RandomUtil;
import mage.util.ThreadUtils;
import org.junit.Assert;
import org.junit.Before;
import org.junit.Test;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Random;
import java.util.concurrent.atomic.AtomicReference;
import java.util.stream.Collectors;

/**
 * State traces (Rust migration phase 0): the same seeded game played twice gives the same fingerprints, so a trace is
 * an oracle another engine can be checked against. -Dxmage.trace.dir=path also writes each trace as NDJSON.
 */
public class GameTraceTest {

    private static final long GAME_TIMEOUT_MS = 5 * 60 * 1000L;

    @Before
    public void setUp() {
        CardScanner.scan();
    }

    @Test
    public void fingerprintHashIsFnv1a() {
        // the published FNV-1a 64 test vectors
        Assert.assertEquals("cbf29ce484222325", StateFingerprint.hashText(""));
        Assert.assertEquals("af63dc4c8601ec8c", StateFingerprint.hashText("a"));
        Assert.assertEquals("85944171f73967e8", StateFingerprint.hashText("foobar"));
    }

    @Test
    public void sameSeedSameTrace() throws Exception {
        GameTrace first = play(4242L, 6, "first");
        GameTrace second = play(4242L, 6, "second");
        Assert.assertTrue("the game was traced: " + first.getEntries().size(), first.getEntries().size() > 20);
        int diff = GameTrace.firstDifference(first.hashes(), second.hashes());
        if (diff >= 0) {
            String a = diff < first.getEntries().size() ? first.getEntries().get(diff).text : "(ended)";
            String b = diff < second.getEntries().size() ? second.getEntries().get(diff).text : "(ended)";
            Assert.fail("traces part at entry " + diff + "\n--- first\n" + a + "\n--- second\n" + b);
        }
        GameTrace other = play(2424L, 6, "other");
        Assert.assertNotEquals("another seed plays another game", first.hashes(), other.hashes());
    }

    private GameTrace play(long seed, int stopOnTurn, String label) throws Exception {
        GameTrace trace = new GameTrace(true, null);
        Game game = new TwoPlayerDuel(MultiplayerAttackOption.LEFT, RangeOfInfluence.ALL,
                MulliganType.GAME_DEFAULT.getMulligan(0), 60, 20, 7);
        Match match = new TwoPlayerMatch(new MatchOptions("trace test", "Two Player Duel", false));
        Player one = new ComputerPlayerControllableProxy("One", RangeOfInfluence.ALL, 1);
        Player two = new ComputerPlayerControllableProxy("Two", RangeOfInfluence.ALL, 1);
        seat(game, match, one, deck("Grizzly Bears", "Forest"));
        seat(game, match, two, deck("Hill Giant", "Mountain"));
        GameOptions options = new GameOptions();
        options.testMode = false;
        options.rollbackTurnsAllowed = false;
        options.stopOnTurn = stopOnTurn;
        options.stopAtStep = PhaseStep.UNTAP;
        options.seed = seed;
        options.trace = trace;
        game.setGameOptions(options);

        AtomicReference<Throwable> failure = new AtomicReference<>();
        Thread thread = new Thread(() -> {
            try {
                // the AI's own choices draw from the thread's generator: seed it as the game does
                RandomUtil.withRandom(new Random(seed), () -> {
                    game.start(one.getId());
                    return null;
                });
            } catch (Throwable e) {
                failure.set(e);
            }
        }, ThreadUtils.THREAD_PREFIX_GAME + " trace test");
        thread.start();
        thread.join(GAME_TIMEOUT_MS);
        if (thread.isAlive()) {
            game.setGameStopped(true);
            thread.interrupt();
            Assert.fail("the game didn't finish in time");
        }
        if (failure.get() != null) {
            throw new AssertionError("the game failed", failure.get());
        }
        write(trace, label);
        return trace;
    }

    private static void write(GameTrace trace, String label) throws IOException {
        String dir = System.getProperty("xmage.trace.dir");
        if (dir == null || dir.isEmpty()) {
            return;
        }
        Path path = Paths.get(dir, "trace-" + label + ".ndjson");
        Files.createDirectories(path.getParent());
        Files.write(path, trace.getEntries().stream().map(GameTrace.Entry::toJson).collect(Collectors.toList()),
                StandardCharsets.UTF_8);
    }

    static void seat(Game game, Match match, Player player, DeckCardLists list) throws Exception {
        Deck deck = Deck.load(list, false, false);
        game.loadCards(deck.getCards(), player.getId());
        game.loadCards(deck.getSideboard(), player.getId());
        game.addPlayer(player, deck);
        match.addPlayer(player, deck);
    }

    static DeckCardLists deck(String creature, String land) {
        DeckCardLists list = new DeckCardLists();
        list.getCards().add(info(creature, 24));
        list.getCards().add(info(land, 24));
        list.getCards().add(info("Lightning Bolt", 12));
        return list;
    }

    static DeckCardInfo info(String name, int amount) {
        CardInfo card = CardRepository.instance.findPreferredCoreExpansionCard(name);
        Assert.assertNotNull(name, card);
        return new DeckCardInfo(card.getName(), card.getCardNumber(), card.getSetCode(), amount);
    }
}
