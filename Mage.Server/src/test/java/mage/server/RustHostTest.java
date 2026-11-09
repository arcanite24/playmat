package mage.server;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

/**
 * The end reasons of a hosted game (docs/rust/GameHost.md): which leave the match, not only the game.
 */
class RustHostTest {

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
        // a join that reached Java: the host is gone, the player isn't
        assertFalse(RustHost.quitsMatch("its host is gone"));
        assertFalse(RustHost.quitsMatch(null));
    }
}
