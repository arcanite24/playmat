package mage.server.ladder;

import mage.cards.decks.DeckCardInfo;
import mage.cards.decks.DeckCardLists;
import mage.cards.repository.CardCriteria;
import mage.cards.repository.CardInfo;
import mage.cards.repository.CardRepository;
import mage.constants.CardType;
import mage.constants.Rarity;
import mage.server.career.CareerCampaigns;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.TreeSet;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * The League's budget bots (#433): each Standard and Pioneer meta list with its rares and mythics swapped for the most
 * similar commons and uncommons of the format that the Rust host plays (cards/verified.txt), and rare lands for
 * basics. The decks are checked in under career/campaigns/decks/&lt;format&gt;/budget-*.dck; this test makes sure they
 * are what the generator makes from today's meta lists. After changing a meta list or the rules below, write them
 * again with
 * <pre>mvn -o -pl Mage.Server test -Dtest=LeagueBudgetDecksTest -Dxmage.writeBudgetDecks=true</pre>
 * then run Mage.Server/ladder-src/roster.py and ladder-calibrate for the new bots' ratings.
 */
public class LeagueBudgetDecksTest {

    private static final Path DECKS = Paths.get("src/main/resources/career/campaigns/decks");
    private static final Path VERIFIED = Paths.get("../rust/crates/playmat-engine/cards/verified.txt");
    private static final String[] FORMATS = {"standard", "pioneer"};
    private static final List<String> BASICS = Arrays.asList("Plains", "Island", "Swamp", "Mountain", "Forest");
    private static final String WUBRG = "WUBRG";
    private static final Pattern ADDS = Pattern.compile("\\{([WUBRG])\\}");
    private static final Pattern WORD = Pattern.compile("[a-z]+");
    private static final Set<String> STOP = new HashSet<>(Arrays.asList("a", "an", "the", "of", "to", "you", "your",
            "it", "its", "and", "or", "that", "this", "on", "in", "with", "is", "for", "each", "until", "end", "turn"));

    @BeforeAll
    static void deckTypes() {
        // the server registers these from config.xml at start
        mage.cards.decks.DeckValidatorFactory.instance.addDeckType("Constructed - Standard", mage.deck.Standard.class);
        mage.cards.decks.DeckValidatorFactory.instance.addDeckType("Constructed - Pioneer", mage.deck.Pioneer.class);
    }

    /**
     * Writes the budget decks, with -Dxmage.writeBudgetDecks=true only: the checked-in files are what the bots play and
     * what their ratings measured, so a new verified card or set never changes them behind the calibration's back.
     */
    @Test
    void writeBudgetDecks() throws Exception {
        org.junit.jupiter.api.Assumptions.assumeTrue(Boolean.getBoolean("xmage.writeBudgetDecks"), "writes only when asked");
        Set<String> verified = verified();
        for (String format : FORMATS) {
            Pool pool = new Pool(format, verified);
            try (java.util.stream.Stream<Path> files = Files.list(DECKS.resolve(format))) {
                for (Path meta : (Iterable<Path>) files.sorted()::iterator) {
                    String file = meta.getFileName().toString();
                    if (file.startsWith("meta-") && file.endsWith(".dck")) {
                        Path out = meta.resolveSibling("budget-" + file.substring("meta-".length()));
                        Files.write(out, budget(format + "/" + file, pool).getBytes(StandardCharsets.UTF_8));
                    }
                }
            }
        }
    }

    /** each meta list has its budget deck, which holds no rare or mythic */
    @Test
    void everyMetaListHasABudgetDeckWithoutRares() throws Exception {
        List<String> problems = new ArrayList<>();
        for (String format : FORMATS) {
            try (java.util.stream.Stream<Path> files = Files.list(DECKS.resolve(format))) {
                for (Path meta : (Iterable<Path>) files.sorted()::iterator) {
                    String file = meta.getFileName().toString();
                    if (!file.startsWith("meta-") || !file.endsWith(".dck")) {
                        continue;
                    }
                    String budget = format + "/budget-" + file.substring("meta-".length());
                    if (!Files.exists(DECKS.resolve(budget))) {
                        problems.add(budget + " is missing");
                        continue;
                    }
                    DeckCardLists deck = CareerCampaigns.get().deck(budget);
                    List<DeckCardInfo> all = new ArrayList<>(deck.getCards());
                    all.addAll(deck.getSideboard());
                    for (Line line : lines(all)) {
                        if (pricey(line.info.getRarity())) {
                            problems.add(budget + ": " + line.info.getName() + " is " + line.info.getRarity());
                        }
                    }
                }
            }
        }
        assertEquals(Collections.emptyList(), problems, "write them with -Dxmage.writeBudgetDecks=true");
    }

    /** every bot's deck is whole, legal in its format, and all on the Rust host */
    @Test
    void everyBotDeckIsLegalAndOnTheRustHost() throws Exception {
        Set<String> verified = verified();
        List<String> problems = new ArrayList<>();
        Set<String> seen = new HashSet<>();
        for (BotRoster.Bot bot : BotRoster.get().bots()) {
            if (!seen.add(bot.deck)) {
                continue;
            }
            DeckCardLists deck = BotRoster.get().deck(bot);
            int main = deck.getCards().stream().mapToInt(DeckCardInfo::getAmount).sum();
            if (main < 60) {
                problems.add(bot.deck + ": " + main + " cards");
            }
            for (String problem : DeckLegality.problems(RankedService.deckType(bot.format), deck)) {
                problems.add(bot.deck + ": " + problem);
            }
            List<DeckCardInfo> all = new ArrayList<>(deck.getCards());
            all.addAll(deck.getSideboard());
            for (DeckCardInfo card : all) {
                if (!verified.contains(card.getCardName())) {
                    problems.add(bot.deck + ": " + card.getCardName() + " isn't on the Rust host");
                }
            }
        }
        assertEquals(Collections.emptyList(), problems);
    }

    /** budget bots play in the League only, full meta lists at skill 5 in the Open queue only, every other bot in both */
    @Test
    void theQueuesDrawFromTheirOwnBots() throws Exception {
        LadderStore store = new LadderStore("jdbc:sqlite::memory:");
        try {
            LadderRatings ratings = new LadderRatings(store, BotRoster.get());
            boolean budget = false;
            for (String format : FORMATS) {
                for (double rating = 400; rating <= 2000; rating += 50) {
                    for (LadderRatings.Candidate c : ratings.candidates(format, RankedService.OPEN, rating)) {
                        assertTrue(!"budget".equals(c.bot.kind), c.bot.id + " in the Open queue");
                    }
                    for (LadderRatings.Candidate c : ratings.candidates(format, RankedService.LEAGUE, rating)) {
                        assertTrue(!("meta".equals(c.bot.kind) && c.bot.skill < 7), c.bot.id + " in the League");
                        budget |= "budget".equals(c.bot.kind);
                    }
                }
            }
            assertTrue(budget, "the League meets budget bots");
            for (BotRoster.Bot bot : BotRoster.get().bots()) {
                boolean special = "budget".equals(bot.kind) || ("meta".equals(bot.kind) && bot.skill < 7);
                assertEquals(!special, bot.plays(RankedService.OPEN) && bot.plays(RankedService.LEAGUE), bot.id);
            }
        } finally {
            store.close();
        }
    }

    // ---- the generator

    private static Set<String> verified() throws IOException {
        Set<String> out = new HashSet<>();
        for (String line : Files.readAllLines(VERIFIED, StandardCharsets.UTF_8)) {
            if (!line.trim().isEmpty()) {
                out.add(line.trim());
            }
        }
        return out;
    }

    private static boolean pricey(Rarity rarity) {
        return rarity == Rarity.RARE || rarity == Rarity.MYTHIC;
    }

    /** the commons and uncommons of the format's sets that the Rust host plays, by name: their newest printing */
    private static final class Pool {
        final Map<String, CardInfo> cards = new java.util.TreeMap<>();
        final List<String> sets;

        Pool(String format, Set<String> verified) {
            sets = mage.server.career.CareerLeague.legalSets(format);
            CardCriteria criteria = new CardCriteria().rarities(Rarity.COMMON, Rarity.UNCOMMON).setCodes(sets.toArray(new String[0]));
            for (CardInfo info : CardRepository.instance.findCards(criteria)) {
                if (!verified.contains(info.getName()) || info.getTypes().contains(CardType.LAND) || info.isNightCard()) {
                    continue;
                }
                cards.merge(info.getName(), info, (a, b) -> order(a) >= order(b) ? a : b);
            }
        }

        /** a printing's place by its set's release, newest last */
        int order(CardInfo info) {
            return sets.size() - sets.indexOf(info.getSetCode());
        }

        /** a basic land's printing in one of the format's sets */
        CardInfo basic(String name) {
            for (String set : sets) {
                for (CardInfo info : CardRepository.instance.findCards(name)) {
                    if (info.getSetCode().equals(set)) {
                        return info;
                    }
                }
            }
            return CardRepository.instance.findCards(name).get(0);
        }
    }

    /** a card as the deck has it, read once */
    private static final class Line {
        final int amount;
        final CardInfo info;

        Line(int amount, CardInfo info) {
            this.amount = amount;
            this.info = info;
        }
    }

    private static String budget(String file, Pool pool) throws IOException {
        DeckCardLists deck = CareerCampaigns.get().deck(file);
        List<Line> main = lines(deck.getCards());
        List<Line> side = lines(deck.getSideboard());
        String colors = colors(main);
        // copies of a name count over the main deck and the sideboard together
        Map<String, Integer> copies = new HashMap<>();
        for (Line line : main) {
            if (!pricey(line.info.getRarity())) {
                copies.merge(line.info.getName(), line.amount, Integer::sum);
            }
        }
        for (Line line : side) {
            if (!pricey(line.info.getRarity())) {
                copies.merge(line.info.getName(), line.amount, Integer::sum);
            }
        }
        Map<String, Line> outMain = swap(main, colors, pool, copies);
        Map<String, Line> outSide = swap(side, colors, pool, copies);
        StringBuilder text = new StringBuilder("NAME:Budget ").append(deck.getName()).append('\n');
        for (Line line : outMain.values()) {
            text.append(line.amount).append(" [").append(line.info.getSetCode()).append(':').append(line.info.getCardNumber()).append("] ")
                    .append(line.info.getName()).append('\n');
        }
        for (Line line : outSide.values()) {
            text.append("SB: ").append(line.amount).append(" [").append(line.info.getSetCode()).append(':').append(line.info.getCardNumber())
                    .append("] ").append(line.info.getName()).append('\n');
        }
        return text.toString();
    }

    private static List<Line> lines(List<DeckCardInfo> cards) {
        List<Line> out = new ArrayList<>();
        for (DeckCardInfo card : cards) {
            CardInfo info = CardRepository.instance.findCard(card.getSetCode(), card.getCardNumber());
            if (info == null) {
                info = CardRepository.instance.findCards(card.getCardName()).get(0);
            }
            out.add(new Line(card.getAmount(), info));
        }
        return out;
    }

    /** the deck's colours, from its spells */
    private static String colors(List<Line> main) {
        Set<Character> found = new TreeSet<>();
        for (Line line : main) {
            if (!line.info.getTypes().contains(CardType.LAND)) {
                for (char c : line.info.getColor().toString().toCharArray()) {
                    if (WUBRG.indexOf(c) >= 0) {
                        found.add(c);
                    }
                }
            }
        }
        StringBuilder out = new StringBuilder();
        for (char c : WUBRG.toCharArray()) {
            if (found.contains(c)) {
                out.append(c);
            }
        }
        return out.toString();
    }

    /**
     * The list with each rare and mythic swapped: a land for basics of the colours it makes (the deck's colours, in
     * turn, when it makes none or any), a spell for the closest common or uncommon (see {@link #distance}), never past
     * four copies of a name.
     */
    private static Map<String, Line> swap(List<Line> lines, String colors, Pool pool, Map<String, Integer> copies) {
        Map<String, Line> out = new LinkedHashMap<>();
        int basicTurn = 0;
        for (Line line : lines) {
            CardInfo card = line.info;
            if (!pricey(card.getRarity())) {
                add(out, card, line.amount);
                continue;
            }
            if (card.getTypes().contains(CardType.LAND)) {
                String makes = makes(card, colors);
                for (int i = 0; i < line.amount; i++) {
                    char color = makes.charAt((basicTurn++) % makes.length());
                    add(out, pool.basic(BASICS.get(WUBRG.indexOf(color))), 1);
                }
                continue;
            }
            int left = line.amount;
            Set<String> tried = new HashSet<>();
            while (left > 0) {
                CardInfo best = null;
                double bestScore = Double.MAX_VALUE;
                for (CardInfo candidate : pool.cards.values()) {
                    if (tried.contains(candidate.getName()) || !within(candidate, colors) || copies.getOrDefault(candidate.getName(), 0) >= 4) {
                        continue;
                    }
                    double score = distance(card, candidate, copies.containsKey(candidate.getName()));
                    if (score < bestScore) {
                        bestScore = score;
                        best = candidate;
                    }
                }
                if (best == null) {
                    throw new IllegalStateException("No budget card for " + card.getName());
                }
                tried.add(best.getName());
                int take = Math.min(left, 4 - copies.getOrDefault(best.getName(), 0));
                copies.merge(best.getName(), take, Integer::sum);
                add(out, best, take);
                left -= take;
            }
        }
        return out;
    }

    private static void add(Map<String, Line> out, CardInfo card, int amount) {
        Line before = out.get(card.getName());
        out.put(card.getName(), new Line((before == null ? 0 : before.amount) + amount, before == null ? card : before.info));
    }

    /** the colours a land adds that the deck plays; the deck's colours when it adds none of them (or any colour) */
    private static String makes(CardInfo land, String colors) {
        StringBuilder out = new StringBuilder();
        for (String rule : land.getRules()) {
            Matcher m = ADDS.matcher(rule);
            while (m.find()) {
                String c = m.group(1);
                if (colors.contains(c) && out.indexOf(c) < 0) {
                    out.append(c);
                }
            }
        }
        return out.length() == 0 ? (colors.isEmpty() ? "W" : colors) : out.toString();
    }

    private static boolean within(CardInfo card, String colors) {
        for (char c : card.getColor().toString().toCharArray()) {
            if (WUBRG.indexOf(c) >= 0 && colors.indexOf(c) < 0) {
                return false;
            }
        }
        return true;
    }

    /** the card's kind for matching: a creature, a planeswalker as a creature, an instant or sorcery, else its type */
    private static String role(CardInfo card) {
        List<CardType> types = card.getTypes();
        if (types.contains(CardType.CREATURE) || types.contains(CardType.PLANESWALKER)) {
            return "creature";
        }
        if (types.contains(CardType.INSTANT) || types.contains(CardType.SORCERY)) {
            return "spell";
        }
        if (types.contains(CardType.ENCHANTMENT)) {
            return "enchantment";
        }
        return types.isEmpty() ? "other" : types.get(0).name().toLowerCase(Locale.ROOT);
    }

    /**
     * How far a budget card is from the one it replaces, lower is closer: a different role is far; then mana value,
     * colours the original has that it lacks, power and toughness for creatures, the creature types they share, how much
     * of their rules text they share, and a nudge towards cards the deck already plays.
     */
    private static double distance(CardInfo original, CardInfo candidate, boolean inDeck) {
        double d = role(original).equals(role(candidate)) ? 0 : 20;
        if (original.getTypes().contains(CardType.INSTANT) != candidate.getTypes().contains(CardType.INSTANT)) {
            d += 1;
        }
        d += 3 * Math.abs(original.getManaValue() - candidate.getManaValue());
        for (char c : original.getColor().toString().toCharArray()) {
            if (WUBRG.indexOf(c) >= 0 && candidate.getColor().toString().indexOf(c) < 0) {
                d += 1;
            }
        }
        if ("creature".equals(role(original)) && "creature".equals(role(candidate))) {
            d += 0.5 * (Math.abs(number(original.getPower()) - number(candidate.getPower()))
                    + Math.abs(number(original.getToughness()) - number(candidate.getToughness())));
            Set<String> shared = new HashSet<>();
            for (Object type : original.getSubTypes()) {
                shared.add(type.toString());
            }
            int same = 0;
            for (Object type : candidate.getSubTypes()) {
                if (shared.contains(type.toString())) {
                    same++;
                }
            }
            d -= 2 * Math.min(2, same);
        }
        d -= 6 * overlap(words(original), words(candidate));
        if (inDeck) {
            d -= 1.5;
        }
        return d;
    }

    private static int number(String value) {
        try {
            return value == null ? 0 : Integer.parseInt(value.trim());
        } catch (NumberFormatException e) {
            return 0;
        }
    }

    private static Set<String> words(CardInfo card) {
        Set<String> out = new HashSet<>();
        String name = card.getName().toLowerCase(Locale.ROOT);
        for (String rule : card.getRules()) {
            Matcher m = WORD.matcher(rule.toLowerCase(Locale.ROOT).replace(name, ""));
            while (m.find()) {
                if (!STOP.contains(m.group()) && m.group().length() > 2) {
                    out.add(m.group());
                }
            }
        }
        return out;
    }

    private static double overlap(Set<String> a, Set<String> b) {
        if (a.isEmpty() || b.isEmpty()) {
            return 0;
        }
        int same = 0;
        for (String w : a) {
            if (b.contains(w)) {
                same++;
            }
        }
        return (double) same / (a.size() + b.size() - same);
    }
}
