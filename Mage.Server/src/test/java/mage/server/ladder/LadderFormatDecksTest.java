package mage.server.ladder;

import mage.cards.decks.DeckCardInfo;
import mage.cards.decks.DeckCardLists;
import mage.constants.Rarity;
import mage.server.career.CareerCampaigns;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeSet;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * The ladder's Modern decks (#435): the Rust host doesn't play every card of the Modern school decks yet (#187), so the
 * ladder fields copies of them, career/campaigns/decks/modern/ladder-*.dck, with the few missing cards swapped for the
 * closest Modern cards it plays (any rarity) and missing lands for basics. Only decks missing at most
 * {@link #MAX_SWAPS} different cards get a ladder copy: past that it isn't the same deck any more. Career Story keeps
 * the school decks as they are. Write them again after changing a school deck or when the Rust host gains cards with
 * <pre>mvn -o -pl Mage.Server test -Dtest=LadderFormatDecksTest -Dxmage.writeLadderDecks=true</pre>
 * then run Mage.Server/ladder-src/roster.py and ladder-calibrate for the bots' ratings.
 * <p>
 * Pauper was measured the same way and stays off the ladder: see {@link #pauperIsNotCoveredYet}.
 */
public class LadderFormatDecksTest {

    static final Path DECKS = Paths.get("src/main/resources/career/campaigns/decks");
    /** the most different cards a school deck may be missing on the Rust host and still get a ladder copy */
    static final int MAX_SWAPS = 3;
    /** a format joins the ladder with at least this many decks */
    static final int MIN_DECKS = 8;

    @BeforeAll
    static void cards() {
        // the swaps look the cards up, and a fresh checkout has no card database until something scans it
        mage.cards.repository.CardScanner.scan();
        mage.cards.decks.DeckValidatorFactory.instance.addDeckType("Constructed - Modern", mage.deck.Modern.class);
        mage.cards.decks.DeckValidatorFactory.instance.addDeckType("Constructed - Pauper", mage.deck.Pauper.class);
    }

    /** the format's banned list, which only a subclass of the validator can read */
    private static Set<String> banned(String format) {
        if ("modern".equals(format)) {
            return new HashSet<>(new mage.deck.Modern() {
                List<String> list() {
                    return banned;
                }
            }.list());
        }
        return new HashSet<>(new mage.deck.Pauper() {
            List<String> list() {
                return banned;
            }
        }.list());
    }

    /** the school decks of a format and the different cards each is missing on the Rust host */
    static Map<String, Set<String>> missing(String format, Set<String> verified) throws Exception {
        Map<String, Set<String>> out = new LinkedHashMap<>();
        try (java.util.stream.Stream<Path> files = Files.list(DECKS.resolve(format))) {
            for (Path path : (Iterable<Path>) files.sorted()::iterator) {
                String file = path.getFileName().toString();
                if (!file.endsWith(".dck") || file.startsWith("ladder-") || file.startsWith("budget-")) {
                    continue;
                }
                DeckCardLists deck = CareerCampaigns.get().deck(format + "/" + file);
                Set<String> names = new TreeSet<>();
                List<DeckCardInfo> all = new ArrayList<>(deck.getCards());
                all.addAll(deck.getSideboard());
                for (DeckCardInfo card : all) {
                    if (!verified.contains(card.getCardName())) {
                        names.add(card.getCardName());
                    }
                }
                out.put(file, names);
            }
        }
        return out;
    }

    /** Writes the ladder copies, with -Dxmage.writeLadderDecks=true only, so a new verified card never changes them behind the calibration's back. */
    @Test
    void writeLadderDecks() throws Exception {
        org.junit.jupiter.api.Assumptions.assumeTrue(Boolean.getBoolean("xmage.writeLadderDecks"), "writes only when asked");
        Set<String> verified = LeagueBudgetDecksTest.verified();
        String format = "modern";
        LeagueBudgetDecksTest.Pool pool = new LeagueBudgetDecksTest.Pool(format, verified, banned(format),
                Rarity.COMMON, Rarity.UNCOMMON, Rarity.RARE, Rarity.MYTHIC);
        for (Map.Entry<String, Set<String>> deck : missing(format, verified).entrySet()) {
            if (deck.getValue().size() > MAX_SWAPS) {
                continue;
            }
            Path out = DECKS.resolve(format).resolve("ladder-" + deck.getKey());
            Files.write(out, ladder(format + "/" + deck.getKey(), pool, verified).getBytes(StandardCharsets.UTF_8));
        }
    }

    private static String ladder(String file, LeagueBudgetDecksTest.Pool pool, Set<String> verified) throws Exception {
        DeckCardLists deck = CareerCampaigns.get().deck(file);
        List<LeagueBudgetDecksTest.Line> main = LeagueBudgetDecksTest.lines(deck.getCards());
        List<LeagueBudgetDecksTest.Line> side = LeagueBudgetDecksTest.lines(deck.getSideboard());
        String colors = LeagueBudgetDecksTest.colors(main);
        Map<String, Integer> copies = new HashMap<>();
        for (LeagueBudgetDecksTest.Line line : main) {
            if (verified.contains(line.info.getName())) {
                copies.merge(line.info.getName(), line.amount, Integer::sum);
            }
        }
        for (LeagueBudgetDecksTest.Line line : side) {
            if (verified.contains(line.info.getName())) {
                copies.merge(line.info.getName(), line.amount, Integer::sum);
            }
        }
        Map<String, LeagueBudgetDecksTest.Line> outMain = LeagueBudgetDecksTest.swap(main, colors, pool, copies, card -> verified.contains(card.getName()));
        Map<String, LeagueBudgetDecksTest.Line> outSide = LeagueBudgetDecksTest.swap(side, colors, pool, copies, card -> verified.contains(card.getName()));
        StringBuilder text = new StringBuilder("NAME:").append(deck.getName()).append('\n');
        for (LeagueBudgetDecksTest.Line line : outMain.values()) {
            text.append(line.amount).append(" [").append(line.info.getSetCode()).append(':').append(line.info.getCardNumber()).append("] ")
                    .append(line.info.getName()).append('\n');
        }
        for (LeagueBudgetDecksTest.Line line : outSide.values()) {
            text.append("SB: ").append(line.amount).append(" [").append(line.info.getSetCode()).append(':').append(line.info.getCardNumber())
                    .append("] ").append(line.info.getName()).append('\n');
        }
        return text.toString();
    }

    /** every Modern school deck close enough to the Rust host has its ladder copy, and enough of them for a ladder */
    @Test
    void modernHasItsLadderDecks() throws Exception {
        Set<String> verified = LeagueBudgetDecksTest.verified();
        List<String> problems = new ArrayList<>();
        int decks = 0;
        for (Map.Entry<String, Set<String>> deck : missing("modern", verified).entrySet()) {
            boolean close = deck.getValue().size() <= MAX_SWAPS;
            boolean written = Files.exists(DECKS.resolve("modern").resolve("ladder-" + deck.getKey()));
            if (close != written) {
                problems.add(deck.getKey() + (close ? " has no ladder copy" : " has a ladder copy but misses " + deck.getValue()));
            }
            decks += written ? 1 : 0;
        }
        assertEquals(Collections.emptyList(), problems, "write them with -Dxmage.writeLadderDecks=true");
        assertTrue(decks >= MIN_DECKS, "Modern has " + decks + " ladder decks");
    }

    /**
     * Pauper's school decks are too far from the Rust host for a ladder (#435): fewer than {@link #MIN_DECKS} of them
     * miss at most {@link #MAX_SWAPS} cards. When this fails, the host has caught up: give Pauper its ladder copies as
     * Modern's, add it to RankedService and the roster.
     */
    @Test
    void pauperIsNotCoveredYet() throws Exception {
        int close = 0;
        for (Set<String> names : missing("pauper", LeagueBudgetDecksTest.verified()).values()) {
            close += names.size() <= MAX_SWAPS ? 1 : 0;
        }
        assertTrue(close < MIN_DECKS, close + " Pauper decks are close to the Rust host now: Pauper can join the ladder");
        assertTrue(!RankedService.isFormat("pauper"));
    }
}
