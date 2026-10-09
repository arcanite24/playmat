package mage.cards.decks;

import mage.MageObject;
import mage.cards.Card;
import org.apache.log4j.Logger;

import java.io.BufferedReader;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.stream.Collectors;
import java.util.stream.Stream;

/**
 * Commander brackets: finds the cards that limit a deck's bracket level.
 * Shared by the desktop client and the web client bridge.
 * See <a href="https://mtg.wiki/page/Commander_Brackets">Commander Brackets</a>.
 */
public final class CommanderBrackets {

    private static final Logger logger = Logger.getLogger(CommanderBrackets.class);

    public static final String GROUP_GAME_CHANGERS = "Game Changers";
    public static final String GROUP_INFINITE_COMBOS = "Early-game 2-Card Combos";
    public static final String GROUP_MASS_LAND_DESTRUCTION = "Mass Land Destruction";
    public static final String GROUP_EXTRA_TURNS = "Extra Turns";

    public static final int MAX_LEVEL = 5;
    public static final int ANY_AMOUNT = 99;

    private static final String RESOURCE_INFINITE_COMBOS = "brackets/infinite-combos.txt";

    // cards limits per group for bracket levels 0..5, 99 means any amount
    // 1-2: no game changers, no two-card infinite combos, no mass land destruction, extra turns rare (2)
    // 3: up to three game changers, no early two-card combos, no mass land destruction
    // 4-5: anything goes
    private static final Map<String, List<Integer>> GROUP_LIMITS = new LinkedHashMap<>();

    static {
        GROUP_LIMITS.put(GROUP_GAME_CHANGERS, Arrays.asList(0, 0, 0, 3, ANY_AMOUNT, ANY_AMOUNT));
        GROUP_LIMITS.put(GROUP_INFINITE_COMBOS, Arrays.asList(0, 0, 0, 0, ANY_AMOUNT, ANY_AMOUNT));
        GROUP_LIMITS.put(GROUP_MASS_LAND_DESTRUCTION, Arrays.asList(0, 0, 0, 0, ANY_AMOUNT, ANY_AMOUNT));
        GROUP_LIMITS.put(GROUP_EXTRA_TURNS, Arrays.asList(0, 0, 0, 3, ANY_AMOUNT, ANY_AMOUNT));
    }

    // https://mtg.wiki/page/Game_Changers
    private static final Set<String> GAME_CHANGERS = new HashSet<>(Arrays.asList(
            "Ad Nauseam",
            "Ancient Tomb",
            "Aura Shards",
            "Biorhythm",
            "Bolas's Citadel",
            "Braids, Cabal Minion",
            "Demonic Tutor",
            "Drannith Magistrate",
            "Chrome Mox",
            "Coalition Victory",
            "Consecrated Sphinx",
            "Crop Rotation",
            "Cyclonic Rift",
            "Enlightened Tutor",
            "Farewell",
            "Field of the Dead",
            "Fierce Guardianship",
            "Force of Will",
            "Gaea's Cradle",
            "Gamble",
            "Gifts Ungiven",
            "Glacial Chasm",
            "Grand Arbiter Augustin IV",
            "Grim Monolith",
            "Humility",
            "Imperial Seal",
            "Intuition",
            "Jeska's Will",
            "Lion's Eye Diamond",
            "Mana Vault",
            "Mishra's Workshop",
            "Mox Diamond",
            "Mystical Tutor",
            "Narset, Parter of Veils",
            "Natural Order",
            "Necropotence",
            "Notion Thief",
            "Rhystic Study",
            "Opposition Agent",
            "Orcish Bowmasters",
            "Panoptic Mirror",
            "Seedborn Muse",
            "Serra's Sanctum",
            "Smothering Tithe",
            "Survival of the Fittest",
            "Teferi's Protection",
            "Tergrid, God of Fright",
            "Thassa's Oracle",
            "The One Ring",
            "The Tabernacle at Pendrell Vale",
            "Underworld Breach",
            "Vampiric Tutor",
            "Worldly Tutor"
    ));

    private static volatile Set<String> infiniteCombos; // card1@card2, names sorted

    private CommanderBrackets() {
    }

    public static List<String> getGroupNames() {
        return new ArrayList<>(GROUP_LIMITS.keySet());
    }

    /**
     * @return max allowed cards of a group for a bracket level, {@link #ANY_AMOUNT} for unlimited
     */
    public static int getMaxCards(String groupName, int level) {
        List<Integer> limits = GROUP_LIMITS.get(groupName);
        if (limits == null) {
            throw new IllegalArgumentException("Unknown commander bracket group: " + groupName);
        }
        return limits.get(Math.max(0, Math.min(MAX_LEVEL, level)));
    }

    public static Analysis analyze(Deck deck) {
        List<Card> cards = Stream.concat(deck.getCards().stream(), deck.getSideboard().stream())
                .collect(Collectors.toList());

        Map<String, List<String>> groups = new LinkedHashMap<>();
        groups.put(GROUP_GAME_CHANGERS, cards.stream()
                .map(MageObject::getName)
                .filter(GAME_CHANGERS::contains)
                .sorted()
                .collect(Collectors.toList()));
        groups.put(GROUP_INFINITE_COMBOS, findInfiniteComboCards(cards));
        groups.put(GROUP_MASS_LAND_DESTRUCTION, cardsWithRule(cards, CommanderBrackets::isMassLandDestruction));
        groups.put(GROUP_EXTRA_TURNS, cardsWithRule(cards, rule -> rule.contains("extra turn")));
        return new Analysis(groups);
    }

    private static List<String> findInfiniteComboCards(List<Card> cards) {
        Set<String> combos = loadInfiniteCombos();
        Set<String> names = cards.stream().map(MageObject::getName).collect(Collectors.toSet());
        Set<String> found = new HashSet<>();
        for (String combo : combos) {
            String[] pair = combo.split("@", 2);
            if (names.contains(pair[0]) && names.contains(pair[1]) && !pair[0].equals(pair[1])) {
                found.add(pair[0]);
                found.add(pair[1]);
            }
        }
        // one entry per deck card, like other groups
        return cards.stream()
                .map(MageObject::getName)
                .filter(found::contains)
                .distinct()
                .sorted()
                .collect(Collectors.toList());
    }

    private static List<String> cardsWithRule(List<Card> cards, java.util.function.Predicate<String> rulePredicate) {
        return cards.stream()
                .filter(card -> card.getRules().stream()
                        .map(rule -> rule.toLowerCase(Locale.ENGLISH))
                        .anyMatch(rulePredicate))
                .map(Card::getName)
                .sorted()
                .collect(Collectors.toList());
    }

    private static boolean isMassLandDestruction(String rule) {
        // https://mtg.wiki/page/Land_destruction
        return (rule.contains("destroy") || rule.contains("sacrifice"))
                && (rule.contains("all") || rule.contains("x target") || rule.contains("{x} target"))
                && (rule.contains("lands")
                || rule.contains("plains")
                || rule.contains("island")
                || rule.contains("swamp")
                || rule.contains("mountain")
                || rule.contains("forest"));
    }

    private static Set<String> loadInfiniteCombos() {
        Set<String> loaded = infiniteCombos;
        if (loaded != null) {
            return loaded;
        }
        synchronized (CommanderBrackets.class) {
            if (infiniteCombos != null) {
                return infiniteCombos;
            }
            Set<String> combos = new HashSet<>();
            InputStream in = CommanderBrackets.class.getClassLoader().getResourceAsStream(RESOURCE_INFINITE_COMBOS);
            if (in == null) {
                logger.warn("Commander brackets: can't find infinite combos list " + RESOURCE_INFINITE_COMBOS);
            } else {
                try (BufferedReader reader = new BufferedReader(new InputStreamReader(in))) {
                    String line;
                    while ((line = reader.readLine()) != null) {
                        line = line.trim();
                        if (line.isEmpty() || line.startsWith("#")) {
                            continue;
                        }
                        List<String> pair = Arrays.asList(line.split("@"));
                        if (pair.size() != 2) {
                            logger.warn("Commander brackets: wrong line format in combos list: " + line);
                            continue;
                        }
                        Collections.sort(pair);
                        combos.add(String.join("@", pair));
                    }
                } catch (Exception e) {
                    logger.warn("Commander brackets: can't load infinite combos list - " + e, e);
                }
            }
            infiniteCombos = Collections.unmodifiableSet(combos);
            return infiniteCombos;
        }
    }

    public static final class Analysis {
        private final Map<String, List<String>> groups;

        private Analysis(Map<String, List<String>> groups) {
            this.groups = groups;
        }

        /**
         * @return cards found per group, in group order
         */
        public Map<String, List<String>> getGroups() {
            return Collections.unmodifiableMap(groups);
        }

        public List<String> getCards(String groupName) {
            return groups.getOrDefault(groupName, Collections.emptyList());
        }

        /**
         * @return all cards of every group that exceeds its limit for the level, sorted and distinct
         */
        public List<String> getBadCards(int level) {
            List<String> badCards = new ArrayList<>();
            groups.forEach((groupName, cards) -> {
                if (cards.size() > getMaxCards(groupName, level)) {
                    badCards.addAll(cards);
                }
            });
            return badCards.stream().distinct().sorted().collect(Collectors.toList());
        }
    }
}
