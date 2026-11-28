package org.mage.test.serverside.solo;

import mage.constants.RangeOfInfluence;
import mage.game.GameState;
import mage.player.ai.ComputerPlayerControllableProxy;
import mage.players.Player;
import org.junit.Assert;
import org.junit.Test;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.UUID;

/**
 * A game carried on from a snapshot (try this turn again, play an earlier turn) starts at a new table, whose match
 * shuffles the seats. The resumed game seats everyone, and plays the turns, as the saved game did.
 */
public class ResumedSeatsTest {

    @Test
    public void aResumedGameKeepsTheSavedSeatsAndTurnOrder() {
        List<Player> players = new ArrayList<>();
        for (String name : Arrays.asList("A", "B", "C", "D")) {
            players.add(new ComputerPlayerControllableProxy(name, RangeOfInfluence.ALL, 1));
        }
        GameState saved = new GameState();
        for (Player player : players) {
            saved.addPlayer(player);
        }
        GameState shuffled = new GameState();
        for (int index : new int[]{2, 0, 3, 1}) {
            shuffled.addPlayer(players.get(index));
        }
        UUID b = players.get(1).getId();
        shuffled.getPlayerList().setCurrent(b);

        Assert.assertTrue(shuffled.seatLike(saved));
        Assert.assertEquals("table order as saved", new ArrayList<>(saved.getPlayers().keySet()),
                new ArrayList<>(shuffled.getPlayers().keySet()));
        Assert.assertEquals("whose turn it is stays put", b, shuffled.getPlayerList().get());
        Assert.assertEquals("turns go round as saved", turnsFrom(saved, b), turnsFrom(shuffled, b));
    }

    @Test
    public void otherPlayersChangeNothing() {
        GameState saved = new GameState();
        saved.addPlayer(new ComputerPlayerControllableProxy("A", RangeOfInfluence.ALL, 1));
        GameState now = new GameState();
        Player other = new ComputerPlayerControllableProxy("B", RangeOfInfluence.ALL, 1);
        now.addPlayer(other);
        Assert.assertFalse(now.seatLike(saved));
        Assert.assertEquals(other.getId(), now.getPlayers().keySet().iterator().next());
    }

    private static List<UUID> turnsFrom(GameState state, UUID start) {
        state.getPlayerList().setCurrent(start);
        List<UUID> turns = new ArrayList<>();
        for (int i = 0; i < 8; i++) {
            turns.add(state.getPlayerList().getNext());
        }
        return turns;
    }
}
