package mage.server.decks;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.List;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

public class DeckStoreTest {

    @TempDir
    Path dir;

    private DeckStore store;

    @BeforeEach
    void setUp() throws Exception {
        store = new DeckStore("jdbc:sqlite:" + dir.resolve("decks.db"));
    }

    @Test
    void storesDecksPerAccountIgnoringCase() throws Exception {
        assertThat(store.put("Ana", "d1", "Burn", 100, "{\"name\":\"Burn\"}").stored).isTrue();
        store.put("bob", "d1", "Bob's", 100, "{}");

        List<DeckSyncEntry> mine = store.list("ana", 200);
        assertThat(mine).hasSize(1);
        assertThat(mine.get(0).name).isEqualTo("Burn");
        assertThat(mine.get(0).data).as("the list leaves data out").isNull();
        assertThat(store.get("ANA", "d1").data).isEqualTo("{\"name\":\"Burn\"}");
        assertThat(store.get("bob", "d1").name).isEqualTo("Bob's");
    }

    @Test
    void theNewerChangeWins() throws Exception {
        store.put("ana", "d1", "Second", 200, "{\"v\":2}");
        DeckStore.DeckPutResult older = store.put("ana", "d1", "First", 100, "{\"v\":1}");
        assertThat(older.stored).isFalse();
        assertThat(older.updatedAt).isEqualTo(200);
        assertThat(store.get("ana", "d1").data).isEqualTo("{\"v\":2}");
    }

    @Test
    void deletionsLeaveTombstonesUntilTheyAreOld() throws Exception {
        store.put("ana", "d1", "Burn", 100, "{}");
        assertThat(store.delete("ana", "d1", 150).stored).isTrue();
        assertThat(store.get("ana", "d1")).isNull();
        List<DeckSyncEntry> list = store.list("ana", 200);
        assertThat(list).hasSize(1);
        assertThat(list.get(0).deleted).isTrue();
        assertThat(list.get(0).updatedAt).isEqualTo(150);

        assertThat(store.list("ana", 150 + DeckStore.TOMBSTONE_MILLIS + 1)).isEmpty();
    }

    @Test
    void anOlderDeletionDoesNotRemoveANewerDeck() throws Exception {
        store.put("ana", "d1", "Burn", 300, "{}");
        assertThat(store.delete("ana", "d1", 200).stored).isFalse();
        assertThat(store.get("ana", "d1")).isNotNull();
    }

    @Test
    void aDeletedDeckCanComeBack() throws Exception {
        store.put("ana", "d1", "Burn", 100, "{}");
        store.delete("ana", "d1", 200);
        assertThat(store.put("ana", "d1", "Burn again", 300, "{\"back\":true}").stored).isTrue();
        assertThat(store.get("ana", "d1").name).isEqualTo("Burn again");
    }

    @Test
    void refusesBadIdsAndHugeDecks() {
        assertThatThrownBy(() -> store.put("ana", "../etc", "x", 1, "{}")).isInstanceOf(IllegalArgumentException.class);
        assertThatThrownBy(() -> store.put("ana", "d1", "x", 1, "")).isInstanceOf(IllegalArgumentException.class);
        StringBuilder huge = new StringBuilder();
        while (huge.length() <= DeckStore.MAX_DECK_CHARS) {
            huge.append("0123456789");
        }
        assertThatThrownBy(() -> store.put("ana", "d1", "x", 1, huge.toString()))
                .isInstanceOf(IllegalArgumentException.class).hasMessageContaining("too large");
    }

    private static final String BURN = "{\"name\":\"Burn\",\"format\":\"Constructed - Modern\",\"colors\":{\"white\":false,\"red\":true},"
            + "\"cards\":[{\"cardName\":\"Lightning Bolt\",\"amount\":4},{\"cardName\":\"Mountain\",\"amount\":20}],"
            + "\"sideboard\":[{\"cardName\":\"Smash to Smithereens\",\"amount\":3}],"
            + "\"coverCard\":{\"setCode\":\"M10\",\"cardNumber\":\"146\",\"name\":\"Lightning Bolt\"}}";

    @Test
    void sharedDecksAreTheOwnersLatestVersionWithASummary() throws Exception {
        store.put("Ana", "d1", "Burn", 100, BURN);
        store.put("ana", "d2", "Private", 100, "{}");
        assertThat(store.setShared("Ana", "d1", true, 1000)).isTrue();

        List<SharedDeck> list = store.sharedList("bob");
        assertThat(list).hasSize(1);
        SharedDeck burn = list.get(0);
        assertThat(burn.owner).as("the owner's own spelling").isEqualTo("Ana");
        assertThat(burn.deckId).isEqualTo("d1");
        assertThat(burn.name).isEqualTo("Burn");
        assertThat(burn.format).isEqualTo("Constructed - Modern");
        assertThat(burn.colors).isEqualTo("R");
        assertThat(burn.cardCount).isEqualTo(24);
        assertThat(burn.sideboardCount).isEqualTo(3);
        assertThat(burn.coverSetCode).isEqualTo("M10");
        assertThat(burn.coverName).isEqualTo("Lightning Bolt");
        assertThat(burn.mine).isFalse();
        assertThat(burn.data).as("the list leaves data out").isNull();
        assertThat(store.sharedList("ANA").get(0).mine).isTrue();

        // an edit reaches everyone
        store.put("ana", "d1", "Burn v2", 200, BURN);
        SharedDeck got = store.sharedGet("bob", "ana", "d1");
        assertThat(got.name).isEqualTo("Burn v2");
        assertThat(got.data).isEqualTo(BURN);
        assertThat(store.sharedGet("bob", "ana", "d2")).as("not shared").isNull();
    }

    @Test
    void stoppingDeletingOrRemovingTheAccountEndsSharing() throws Exception {
        store.put("ana", "d1", "Burn", 100, BURN);
        store.put("ana", "d2", "Zoo", 100, BURN);
        store.put("ana", "d3", "Tron", 100, BURN);
        store.setShared("ana", "d1", true, 1);
        store.setShared("ana", "d2", true, 1);
        store.setShared("ana", "d3", true, 1);

        assertThat(store.setShared("ana", "d1", false, 2)).isFalse();
        store.delete("ana", "d2", 300);
        assertThat(store.sharedGet("bob", "ana", "d2")).isNull();
        assertThat(store.sharedList("bob")).extracting(deck -> deck.deckId).containsExactly("d3");

        // a deck deleted where the share store doesn't hear of it (Rust's deck sync) drops out too, and stays out
        store.put("ana", "d4", "Elves", 100, BURN);
        store.setShared("ana", "d4", true, 1);
        assertThat(store.sharedList("bob")).hasSize(2);
        try (java.sql.Connection sql = java.sql.DriverManager.getConnection("jdbc:sqlite:" + dir.resolve("decks.db"));
             java.sql.Statement update = sql.createStatement()) {
            update.executeUpdate("UPDATE web_deck SET deleted = 1, data = NULL, updatedAt = 400 WHERE \"key\" = 'ana/d4'");
        }
        assertThat(store.sharedList("bob")).extracting(deck -> deck.deckId).containsExactly("d3");
        store.put("ana", "d4", "Elves again", 500, BURN);
        assertThat(store.sharedList("bob")).extracting(deck -> deck.deckId).containsExactly("d3");

        store.deleteAll("ana");
        assertThat(store.sharedList("bob")).isEmpty();
        store.put("ana", "d3", "Tron", 500, BURN);
        assertThat(store.sharedList("bob")).as("a new account with the name starts unshared").isEmpty();
    }

    @Test
    void onlySavedDecksCanBeSharedAndOnlySoMany() throws Exception {
        assertThatThrownBy(() -> store.setShared("ana", "nope", true, 1))
                .isInstanceOf(IllegalArgumentException.class).hasMessageContaining("isn't saved");
        store.put("ana", "gone", "Gone", 100, "{}");
        store.delete("ana", "gone", 200);
        assertThatThrownBy(() -> store.setShared("ana", "gone", true, 1)).isInstanceOf(IllegalArgumentException.class);
        assertThatThrownBy(() -> store.setShared("ana", "../x", true, 1)).isInstanceOf(IllegalArgumentException.class);

        for (int i = 0; i < DeckStore.MAX_SHARED; i++) {
            store.put("ana", "d" + i, "Deck " + i, 100 + i, "{}");
            store.setShared("ana", "d" + i, true, 1);
        }
        assertThat(store.setShared("ana", "d0", true, 2)).as("sharing again is no new share").isTrue();
        store.put("ana", "extra", "Extra", 100, "{}");
        assertThatThrownBy(() -> store.setShared("ana", "extra", true, 1))
                .isInstanceOf(IllegalArgumentException.class).hasMessageContaining("up to");
        assertThat(store.sharedList("bob")).hasSize(DeckStore.MAX_SHARED);
        assertThat(store.sharedList("bob").get(0).name).as("newest change first").isEqualTo("Deck " + (DeckStore.MAX_SHARED - 1));
    }

    @Test
    void anUnreadableDeckIsListedByNameAlone() throws Exception {
        store.put("ana", "d1", "Odd", 100, "not json");
        store.setShared("ana", "d1", true, 1);
        SharedDeck odd = store.sharedList("bob").get(0);
        assertThat(odd.name).isEqualTo("Odd");
        assertThat(odd.cardCount).isZero();
        assertThat(odd.format).isNull();
    }
}
