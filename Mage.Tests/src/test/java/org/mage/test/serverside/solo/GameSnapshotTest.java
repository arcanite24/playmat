package org.mage.test.serverside.solo;

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
import mage.game.GameSnapshot;
import mage.game.GameState;
import mage.game.TwoPlayerDuel;
import mage.game.TwoPlayerMatch;
import mage.game.match.Match;
import mage.game.match.MatchOptions;
import mage.game.mulligan.MulliganType;
import mage.game.permanent.Permanent;
import mage.player.ai.ComputerPlayerControllableProxy;
import mage.players.Player;
import mage.util.GameRandom;
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
import java.util.TreeMap;
import java.util.UUID;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.atomic.AtomicReference;
import java.util.stream.Collectors;

/**
 * Seeded games and game snapshots (solo foundations S001, S002 and S006): the same seed deals the same cards, a game
 * frozen at the start of a turn reads back whole, and a new table with new players carries on from it.
 */
public class GameSnapshotTest {

    private static final long GAME_TIMEOUT_MS = 5 * 60 * 1000L;

    @Before
    public void setUp() {
        CardScanner.scan();
    }

    @Test
    public void gameRandomCopiesAndRestoresExactly() throws Exception {
        GameRandom a = new GameRandom(99);
        Random reference = new Random(99);
        for (int i = 0; i < 20; i++) {
            Assert.assertEquals("same sequence as java.util.Random", reference.nextInt(1000), a.nextInt(1000));
        }
        GameRandom copy = a.copy();
        int next = a.nextInt();
        Assert.assertEquals(next, copy.nextInt());
        GameRandom other = new GameRandom(5);
        other.restoreFrom(a);
        Assert.assertEquals(99, other.getSeed());
        Assert.assertEquals(a.nextLong(), other.nextLong());

        // a thread with a generator installed draws from it, others from the shared one
        GameRandom installed = new GameRandom(7);
        int expected = new GameRandom(7).nextInt(100);
        int drawn = RandomUtil.withRandom(installed, () -> RandomUtil.nextInt(100));
        Assert.assertEquals(expected, drawn);
        Assert.assertFalse(RandomUtil.isInstalled());
    }

    @Test
    public void sameSeedDealsTheSameCards() throws Exception {
        Run first = play(1234L, 2, null);
        Run second = play(1234L, 2, null);
        Run other = play(4321L, 2, null);
        Assert.assertFalse("a snapshot at turn 1", first.snapshots.isEmpty());
        Assert.assertEquals(Long.valueOf(1234L), first.snapshots.get(0).getSeed());

        List<List<String>> a = libraries(first.snapshots.get(0));
        List<List<String>> b = libraries(second.snapshots.get(0));
        List<List<String>> c = libraries(other.snapshots.get(0));
        Assert.assertEquals("same seed: same opening hands and library order", a, b);
        Assert.assertNotEquals("another seed: other cards", a, c);
    }

    @Test
    public void newTableCarriesOnFromASnapshot() throws Exception {
        Run original = play(77L, 6, null);
        GameSnapshot saved = original.snapshots.stream().filter(s -> s.getTurnNum() == 4).findFirst().orElse(null);
        Assert.assertNotNull("a snapshot at the start of turn 4", saved);
        // through bytes, as a save on disk does
        saved = GameSnapshot.fromBytes(saved.toBytes());
        GameState before = saved.restore().state;

        Run resumed = play(null, 7, saved);
        Assert.assertNull("the resumed game ran: " + resumed.failure, resumed.failure);
        Assert.assertFalse(resumed.snapshots.isEmpty());
        GameSnapshot first = resumed.snapshots.get(0);
        Assert.assertEquals("the resumed game starts at turn 4", 4, first.getTurnNum());
        Assert.assertEquals("and keeps the original seed", Long.valueOf(77L), first.getSeed());
        GameState after = first.restore().state;

        Map<UUID, UUID> ids = resumed.savedToNew;
        Assert.assertEquals("same permanents", battlefield(before, ids), battlefield(after, null));
        for (Player savedPlayer : before.getPlayers().values()) {
            Player now = after.getPlayer(ids.get(savedPlayer.getId()));
            Assert.assertNotNull("player carried over under the new id", now);
            Assert.assertEquals(savedPlayer.getLife(), now.getLife());
            Assert.assertEquals(savedPlayer.getHand().size(), now.getHand().size());
            Assert.assertEquals(savedPlayer.getLibrary().size(), now.getLibrary().size());
            Assert.assertEquals(savedPlayer.getGraveyard().size(), now.getGraveyard().size());
        }
        Assert.assertTrue("and plays on past it", resumed.game.getTurnNum() > 4);
    }

    private static Map<String, Long> battlefield(GameState state, Map<UUID, UUID> ids) {
        Map<String, Long> names = new TreeMap<>();
        for (Permanent permanent : state.getBattlefield().getAllPermanents()) {
            UUID controller = ids == null ? permanent.getControllerId() : ids.get(permanent.getControllerId());
            names.merge(controller + " " + permanent.getName(), 1L, Long::sum);
        }
        return names;
    }

    private static List<List<String>> libraries(GameSnapshot snapshot) {
        GameSnapshot.Restored restored = snapshot.restore();
        List<List<String>> all = new ArrayList<>();
        for (Player player : restored.state.getPlayers().values()) {
            List<String> names = new ArrayList<>();
            player.getHand().stream().map(id -> restored.gameCards.get(id).getName()).sorted().forEach(names::add);
            names.add("|");
            player.getLibrary().getCardList().stream().map(id -> restored.gameCards.get(id).getName()).forEach(names::add);
            all.add(names);
        }
        return all;
    }

    private static class Run {
        Game game;
        final List<GameSnapshot> snapshots = new CopyOnWriteArrayList<>();
        Map<UUID, UUID> savedToNew = new HashMap<>();
        Throwable failure;
    }

    private Run play(Long seed, int stopOnTurn, GameSnapshot resumeFrom) throws Exception {
        Run run = new Run();
        Game game = new TwoPlayerDuel(MultiplayerAttackOption.LEFT, RangeOfInfluence.ALL,
                MulliganType.GAME_DEFAULT.getMulligan(0), 60, 20, 7);
        Match match = new TwoPlayerMatch(new MatchOptions("snapshot test", "Two Player Duel", false));
        Player one = new ComputerPlayerControllableProxy("One", RangeOfInfluence.ALL, 1);
        Player two = new ComputerPlayerControllableProxy("Two", RangeOfInfluence.ALL, 1);
        seat(game, match, one, deck("Grizzly Bears", "Forest"));
        seat(game, match, two, deck("Hill Giant", "Mountain"));
        GameOptions options = new GameOptions();
        options.testMode = false;
        options.rollbackTurnsAllowed = false; // as in Career: snapshots must not need rollbacks
        options.stopOnTurn = stopOnTurn;
        options.stopAtStep = PhaseStep.UNTAP;
        options.seed = seed;
        options.turnSnapshots = run.snapshots::add;
        if (resumeFrom != null) {
            // the saved players were seated in the same order
            List<UUID> saved = resumeFrom.getPlayerIds();
            run.savedToNew.put(saved.get(0), one.getId());
            run.savedToNew.put(saved.get(1), two.getId());
            options.resumeFrom = resumeFrom.withPlayers(run.savedToNew);
        }
        game.setGameOptions(options);
        run.game = game;

        AtomicReference<Throwable> failure = new AtomicReference<>();
        Thread thread = new Thread(() -> {
            try {
                game.start(one.getId());
            } catch (Throwable e) {
                failure.set(e);
            }
        }, ThreadUtils.THREAD_PREFIX_GAME + " snapshot test");
        thread.start();
        thread.join(GAME_TIMEOUT_MS);
        if (thread.isAlive()) {
            game.setGameStopped(true);
            thread.interrupt();
            Assert.fail("the game didn't finish in time");
        }
        run.failure = failure.get();
        if (run.failure != null && resumeFrom == null) {
            throw new AssertionError("the game failed", run.failure);
        }
        return run;
    }

    private static void seat(Game game, Match match, Player player, DeckCardLists list) throws Exception {
        Deck deck = Deck.load(list, false, false);
        game.loadCards(deck.getCards(), player.getId());
        game.loadCards(deck.getSideboard(), player.getId());
        game.addPlayer(player, deck);
        match.addPlayer(player, deck);
    }

    private static DeckCardLists deck(String creature, String land) {
        DeckCardLists list = new DeckCardLists();
        list.getCards().add(info(creature, 24));
        list.getCards().add(info(land, 24));
        list.getCards().add(info("Lightning Bolt", 12));
        return list;
    }

    private static DeckCardInfo info(String name, int amount) {
        CardInfo card = CardRepository.instance.findPreferredCoreExpansionCard(name);
        Assert.assertNotNull(name, card);
        return new DeckCardInfo(card.getName(), card.getCardNumber(), card.getSetCode(), amount);
    }

    @SuppressWarnings("unused")
    private static String describe(List<GameSnapshot> snapshots) {
        return snapshots.stream().map(s -> s.getTurnNum() + ":" + s.size()).collect(Collectors.joining(", "));
    }
}
