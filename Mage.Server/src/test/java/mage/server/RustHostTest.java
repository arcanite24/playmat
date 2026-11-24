package mage.server;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

/**
 * The end reasons of a hosted game (docs/rust/GameHost.md): which leave the match, not only the game.
 */
class RustHostTest {

    @Test
    void hostsTheCareerKindsWhoseGamesAreDuelsFromTheStart() {
        for (String kind : new String[]{"duel", "campaign", "gauntlet", "challenge"}) {
            assertTrue(RustHost.HOSTED_CAREER_KINDS.contains(kind), kind);
        }
        for (String kind : new String[]{"trial", "puzzle", "horde", "limited"}) {
            assertFalse(RustHost.HOSTED_CAREER_KINDS.contains(kind), kind);
        }
    }

    @Test
    void aQuitLeavesTheMatchAConcedeOnlyTheGame() {
        // the host's reasons
        assertTrue(RustHost.quitsMatch("quit"));
        assertFalse(RustHost.quitsMatch("conceded"));
        assertFalse(RustHost.quitsMatch("over"));
        assertFalse(RustHost.quitsMatch("stopped"));
        assertFalse(RustHost.quitsMatch("error: a card"));
        // Java's own: a host that's gone with its player (the sweep, a matchQuit that reached Java)
        assertTrue(RustHost.quitsMatch("quit, its host is gone"));
        // a player whose session expired (User.removeUserFromAllTables)
        assertTrue(RustHost.quitsMatch("quit, the player left"));
        // a join that reached Java: the host is gone, the player isn't
        assertFalse(RustHost.quitsMatch("its host is gone"));
        assertFalse(RustHost.quitsMatch(null));
        // running out of time loses the match, as in Java (PlayerImpl.timerTimeout)
        assertTrue(RustHost.quitsMatch("timed out"));
    }
}
