package mage.game.advice;

import mage.game.Game;

import java.util.UUID;

/**
 * Thinks from a player's seat and says what it would do, without doing it: the basis of hints and game reviews. The
 * AI plugins implement it; the server finds an implementation through the AI player's class loader.
 */
public interface GameAdvisor {

    /**
     * @param game     a copy of the game that the advisor owns and may change (a simulation copy, never the live game)
     * @param playerId the seat to think for
     * @param skill    how hard to think, as the AI skill levels 1 to 10
     */
    Advice advise(Game game, UUID playerId, int skill);
}
