package mage.server.ladder;

import mage.cards.Card;
import mage.cards.decks.Deck;
import mage.cards.decks.DeckCardLists;
import mage.cards.repository.CardScanner;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Arrays;
import java.util.UUID;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

/** Tests playbook loading, revealed-card matchup classification, and Bo3 sideboard swaps. */
public class LadderSideboarderTest {

    @BeforeAll
    static void scan() {
        CardScanner.scan();
    }

    @Test
    void rustAndJavaPlaybooksJsonAreIdentical() throws Exception {
        Path javaFile = Paths.get("src/main/resources/ladder/playbooks.json");
        Path rustFile = Paths.get("../rust/crates/playmat-ai/src/playbooks.json");
        assertTrue(Files.exists(javaFile), "Java playbooks.json exists");
        assertTrue(Files.exists(rustFile), "Rust playbooks.json exists");
        String j = new String(Files.readAllBytes(javaFile), StandardCharsets.UTF_8);
        String r = new String(Files.readAllBytes(rustFile), StandardCharsets.UTF_8);
        assertEquals(r, j, "Java and Rust playbooks.json must stay in sync");
    }

    @Test
    void classifiesRevealedOpponentCardsIntoMatchups() {
        assertEquals("vs_aggro", LadderSideboarder.classify(Arrays.asList(
                "Monastery Swiftspear", "Goblin Guide", "Lightning Bolt", "Lava Spike")));
        assertEquals("vs_control", LadderSideboarder.classify(Arrays.asList(
                "No More Lies", "Day of Judgment", "Island", "Plains")));
        assertEquals("vs_graveyard", LadderSideboarder.classify(Arrays.asList(
                "Treasure Cruise", "Faithless Looting", "Arclight Phoenix")));
    }

    @Test
    void appliesScriptedSideboardPlanPreservingDeckSize() throws Exception {
        BotRoster.Bot bot = BotRoster.get().bot("standard-meta-azorius-control-s7");
        DeckCardLists lists = BotRoster.get().deck(bot);
        Deck deck = Deck.load(lists, true, true);
        int mainBefore = deck.getCards().size();
        int sbBefore = deck.getSideboard().size();

        UUID tableId = UUID.randomUUID();
        LadderSideboarder.recordRevealedNames(tableId, Arrays.asList(
                "Monastery Swiftspear", "Goblin Guide", "Lightning Bolt", "Lava Spike"));
        try {
            int swapped = LadderSideboarder.apply(tableId, "standard/meta-azorius-control", deck);
            assertTrue(swapped >= 3, "expected at least 3 aggro sideboard swaps, got " + swapped);
            assertEquals(mainBefore, deck.getCards().size(), "maindeck size stays identical");
            assertEquals(sbBefore, deck.getSideboard().size(), "sideboard size stays identical");
            long bezaInMain = deck.getCards().stream().map(Card::getName).filter("Beza, the Bounding Spring"::equals).count();
            assertEquals(2, bezaInMain, "Beza came in against aggro");
        } finally {
            LadderSideboarder.forget(tableId);
        }
    }
}
