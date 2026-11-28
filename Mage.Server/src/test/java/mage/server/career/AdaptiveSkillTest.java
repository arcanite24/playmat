package mage.server.career;

import mage.server.solo.SoloStore;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.UUID;

import static org.junit.jupiter.api.Assertions.*;

/**
 * AdaptiveSkill table markers belong to the marked user and must not be consumed when another human seat is checked first.
 */
public class AdaptiveSkillTest {

    @TempDir
    Path dir;

    private SoloStore store;

    @BeforeEach
    void setUp() throws Exception {
        store = new SoloStore("jdbc:sqlite:" + dir.resolve("solo.db"));
        SoloStore.use(store);
    }

    @AfterEach
    void tearDown() throws Exception {
        store.close();
        SoloStore.use(null);
    }

    @Test
    void endedDoesNotRemoveMarkerForDifferentUser() throws Exception {
        UUID tableId = UUID.randomUUID();
        AdaptiveSkill.mark(tableId, "Ana", "standard");
        assertTrue(AdaptiveSkill.marked(tableId));

        // Another human seat checked first must not consume Ana's marker
        AdaptiveSkill.ended(tableId, "Bob", false);
        assertTrue(AdaptiveSkill.marked(tableId), "marker stays for the marked player");
        assertEquals(0, AdaptiveSkill.view("Ana", "standard").games);
        assertEquals(0, AdaptiveSkill.view("Bob", "standard").games);

        // Now Ana's seat ends the game and consumes the marker once
        AdaptiveSkill.ended(tableId, "ana", true);
        assertFalse(AdaptiveSkill.marked(tableId));
        assertEquals(1, AdaptiveSkill.view("Ana", "standard").games);
        assertEquals(1, AdaptiveSkill.view("Ana", "standard").recentWins);

        // A duplicate call does not double-record
        AdaptiveSkill.ended(tableId, "Ana", true);
        assertEquals(1, AdaptiveSkill.view("Ana", "standard").games);
    }
}
