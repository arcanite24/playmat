package mage.server.ladder;

import mage.cards.decks.DeckCardInfo;
import mage.cards.decks.DeckCardLists;
import mage.cards.decks.DeckValidatorFactory;
import mage.cards.repository.CardScanner;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Set;
import java.util.stream.Stream;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * Validates every dedicated ranked deck under `resources/ladder/decks/{standard,pioneer,modern}/`:
 * format legality (`DeckLegality`), 100% Rust engine coverage (`verified.txt`), and a valid playbook entry.
 */
public class LadderDeckPoolTest {

    private static final Path LADDER_DECKS = Paths.get("src/main/resources/ladder/decks");

    @BeforeAll
    static void init() {
        CardScanner.scan();
        DeckValidatorFactory.instance.addDeckType("Constructed - Standard", mage.deck.Standard.class);
        DeckValidatorFactory.instance.addDeckType("Constructed - Pioneer", mage.deck.Pioneer.class);
        DeckValidatorFactory.instance.addDeckType("Constructed - Modern", mage.deck.Modern.class);
    }

    @Test
    void allLadderDecksAreFormatLegalAndRustVerified() throws Exception {
        Set<String> verified = LeagueBudgetDecksTest.verified();
        List<String> problems = new ArrayList<>();
        int total = 0;

        for (String fmt : new String[]{"standard", "pioneer", "modern"}) {
            Path dir = LADDER_DECKS.resolve(fmt);
            assertTrue(Files.isDirectory(dir), "missing " + dir);
            String deckType = RankedService.deckType(fmt);
            try (Stream<Path> stream = Files.list(dir)) {
                List<Path> files = new ArrayList<>();
                stream.filter(p -> p.getFileName().toString().endsWith(".dck")).sorted().forEach(files::add);
                for (Path file : files) {
                    total++;
                    String rel = "ladder/" + fmt + "/" + file.getFileName();
                    BotRoster.Bot dummy = new BotRoster.Bot();
                    dummy.deck = rel;
                    DeckCardLists deck = BotRoster.get().deck(dummy);
                    List<String> legality = DeckLegality.problems(deckType, deck);
                    if (!legality.isEmpty()) {
                        problems.add(rel + " illegal: " + legality);
                    }
                    List<DeckCardInfo> all = new ArrayList<>(deck.getCards());
                    all.addAll(deck.getSideboard());
                    for (DeckCardInfo info : all) {
                        if (!verified.contains(info.getCardName())) {
                            problems.add(rel + " unverified card: " + info.getCardName());
                        }
                    }
                    String pbId = rel.substring(0, rel.length() - 4);
                    assertNotNull(LadderPlaybooks.get().deck(pbId), "missing playbook for " + pbId);
                }
            }
        }
        assertTrue(total >= 50, "expected >= 50 dedicated ladder decks, got " + total);
        assertEquals(Collections.emptyList(), problems, "ladder deck pool problems:\n" + String.join("\n", problems));
    }
}
