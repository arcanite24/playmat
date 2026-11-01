package mage.game;

import mage.players.PlayerList;
import org.junit.Before;
import org.junit.Test;

import static org.junit.Assert.*;

/**
 * Bookmarks are positions in the saved-state stack; the state index stored at each position can run ahead of it
 * (seen in production as "Saved states: [0, 3]"). Undo must still find the state, or a cancelled payment leaves the
 * spell on the stack for free (#148).
 */
public class BookmarkTest {

    private FakeGame game;

    @Before
    public void setUp() {
        game = new FakeGame();
        game.playerList = new PlayerList();
    }

    /** Saves states that no bookmark points at, so the next bookmark's stored index runs ahead of its position. */
    private void drift(int states) {
        for (int i = 0; i < states; i++) {
            game.getGameStates().save(game.getState());
        }
    }

    @Test
    public void restoresABookmarkWhoseStateIndexRanAhead() {
        game.bookmarkState();
        drift(2);
        int bookmark = game.bookmarkState();

        assertEquals(2, bookmark);
        assertNotNull("undo must find the bookmark", game.restoreState(bookmark, "cancel payment"));
    }

    @Test
    public void removingALaterBookmarkKeepsTheStateOfAnEarlierOne() {
        game.bookmarkState();
        drift(2);
        int cast = game.bookmarkState();
        int manaAbility = game.bookmarkState();

        game.removeBookmark(manaAbility);
        game.removeBookmark(cast);

        assertNotNull("the cast's state must survive", game.restoreState(cast, "cancel payment"));
    }

    @Test
    public void refusesABookmarkThatIsGone() {
        int bookmark = game.bookmarkState();
        game.removeBookmark_v2(bookmark);

        assertNull(game.restoreState(bookmark, "stale"));
    }
}
