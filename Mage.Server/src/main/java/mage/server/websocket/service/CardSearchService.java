package mage.server.websocket.service;

import mage.ObjectColor;
import mage.cards.decks.Constructed;
import mage.cards.decks.DeckCardInfo;
import mage.cards.decks.DeckValidator;
import mage.cards.decks.DeckValidatorFactory;
import mage.cards.repository.CardCriteria;
import mage.cards.repository.CardInfo;
import mage.cards.repository.CardRepository;
import mage.cards.repository.ExpansionInfo;
import mage.cards.repository.ExpansionRepository;
import mage.filter.FilterMana;
import mage.filter.predicate.card.CardTextPredicate;
import mage.view.CardView;

import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Set;
import java.util.stream.Collectors;
import java.util.stream.Stream;

/**
 * Web client bridge: card database queries for the deck editor and card resolution.
 */
public final class CardSearchService {

    private static final long DEFAULT_PAGE_SIZE = 100L;
    private static final long MAX_PAGE_SIZE = 1000L;

    public List<CardView> search(CardCriteria criteria) {
        return search(criteria, null);
    }

    public List<CardView> search(CardCriteria criteria, WebCardFilters web) {
        // the same query pages the same way, and Rust's catalog (CatalogExport) gives the same order
        criteria.stableOrder();
        Set<Character> identity = web == null || web.getColorIdentity() == null ? null : identityOf(web.getColorIdentity());
        long start = criteria.getStart() == null ? 0L : Math.max(0L, criteria.getStart());
        long count = criteria.getCount() == null ? DEFAULT_PAGE_SIZE : Math.max(0L, Math.min(MAX_PAGE_SIZE, criteria.getCount()));

        boolean textSearch = hasTextSearch(criteria);
        boolean refinements = textSearch || identity != null || hasRefinements(criteria);
        if (refinements) {
            // web-only filters run in memory, so the database must return the unpaged list
            criteria.start(null);
            criteria.count(null);
        } else {
            criteria.start(start);
            criteria.count(count);
        }

        if (criteria.getFormat() != null && !criteria.getFormat().isEmpty()) {
            DeckValidator validator = DeckValidatorFactory.instance.createDeckValidator(criteria.getFormat());
            if (validator instanceof Constructed) {
                List<String> setCodes = ((Constructed) validator).getSetCodes();
                if (setCodes != null && !setCodes.isEmpty()) {
                    criteria.setCodes(setCodes);
                }
            }
        }

        // the database only matches mana value exactly; ranges are applied below
        Integer manaValue = criteria.getManaValue();
        String manaValueOperator = criteria.getManaValueOperator();
        if (manaValue != null && manaValueOperator != null && !manaValueOperator.equals("eq")) {
            criteria.manaValue(null);
        }

        Stream<CardInfo> cards = CardRepository.instance.findCards(criteria).stream();
        if (textSearch) {
            CardTextPredicate textPredicate = new CardTextPredicate(
                    criteria.getSearchText(),
                    criteria.isSearchNames(),
                    criteria.isSearchTypes(),
                    criteria.isSearchRules(),
                    criteria.isUniqueNames()
            );
            cards = cards.filter(info -> textPredicate.apply(info.createMockCard(), null));
        }
        if (refinements) {
            cards = cards.filter(info -> matchesRefinements(info, criteria)
                    && matchesManaValue(info, manaValue, manaValueOperator)
                    && (identity == null || withinIdentity(info, identity)));
            if (criteria.isUniqueNames()) {
                // one printing per card name: the first the database returns
                Set<String> seen = new HashSet<>();
                cards = cards.filter(info -> seen.add(info.getName()));
            }
            cards = cards.skip(start).limit(count);
        }
        return cards.map(info -> new CardView(info.createMockCard())).collect(Collectors.toList());
    }

    private static final int MAX_LOOKUP = 500;

    /** Card details for deck entries, in order; null where the card is unknown. */
    public List<CardView> lookup(DeckCardInfo[] entries) {
        List<CardView> result = new ArrayList<>();
        if (entries == null) {
            return result;
        }
        for (int i = 0; i < Math.min(entries.length, MAX_LOOKUP); i++) {
            DeckCardInfo entry = entries[i];
            CardInfo info = entry == null ? null : findEntry(entry);
            result.add(info == null ? null : new CardView(info.createMockCard()));
        }
        return result;
    }

    private static CardInfo findEntry(DeckCardInfo entry) {
        String setCode = entry.getSetCode();
        String number = entry.getCardNumber();
        if (setCode != null && !setCode.isEmpty() && number != null && !number.isEmpty()) {
            CardInfo exact = CardRepository.instance.findCard(setCode, number);
            if (exact != null) {
                return exact;
            }
        }
        String name = entry.getCardName();
        if (name == null || name.isEmpty()) {
            return null;
        }
        return CardRepository.instance.findPreferredCoreExpansionCard(name, setCode);
    }

    public List<ExpansionSetInfo> expansionSets() {
        return ExpansionRepository.instance.getAll().stream()
                .map(ExpansionSetInfo::new)
                .collect(Collectors.toList());
    }

    public List<BasicLandSetInfo> basicLandSets() {
        return ExpansionRepository.instance.getSetsWithBasicLandsByReleaseDate().stream()
                .map(BasicLandSetInfo::new)
                .collect(Collectors.toList());
    }

    private static boolean hasTextSearch(CardCriteria criteria) {
        return criteria.getSearchText() != null && !criteria.getSearchText().trim().isEmpty()
                && (criteria.isSearchNames() || criteria.isSearchTypes() || criteria.isSearchRules());
    }

    private static boolean hasRefinements(CardCriteria criteria) {
        return criteria.isUniqueNames()
                || !criteria.getWebColors().isEmpty()
                || !criteria.getWebExcludedColors().isEmpty()
                || !criteria.getExcludedRarities().isEmpty()
                || (criteria.getManaValue() != null && criteria.getManaValueOperator() != null
                && !criteria.getManaValueOperator().equals("eq"));
    }

    private static boolean matchesRefinements(CardInfo info, CardCriteria criteria) {
        Set<String> cardColors = colorsOf(info);
        return matchesColors(cardColors, criteria.getWebColors(), criteria.getColorMatch())
                && !matchesAnyColor(cardColors, criteria.getWebExcludedColors())
                && !criteria.getExcludedRarities().contains(info.getRarity());
    }

    private static Set<Character> identityOf(String letters) {
        Set<Character> identity = new HashSet<>();
        for (char letter : letters.toUpperCase(Locale.ENGLISH).toCharArray()) {
            if ("WUBRG".indexOf(letter) >= 0) {
                identity.add(letter);
            }
        }
        return identity;
    }

    /**
     * The card's color identity fits inside the allowed colors (Commander deck building, rule 903.4).
     */
    private static boolean withinIdentity(CardInfo info, Set<Character> identity) {
        // a card's colors are part of its identity: most cards are ruled out without building the card
        for (String color : colorsOf(info)) {
            if (!identity.contains(COLOR_LETTERS.get(color))) {
                return false;
            }
        }
        FilterMana cardIdentity = info.createMockCard().getColorIdentity();
        return (!cardIdentity.isWhite() || identity.contains('W'))
                && (!cardIdentity.isBlue() || identity.contains('U'))
                && (!cardIdentity.isBlack() || identity.contains('B'))
                && (!cardIdentity.isRed() || identity.contains('R'))
                && (!cardIdentity.isGreen() || identity.contains('G'));
    }

    private static final java.util.Map<String, Character> COLOR_LETTERS = new java.util.HashMap<>();

    static {
        COLOR_LETTERS.put("white", 'W');
        COLOR_LETTERS.put("blue", 'U');
        COLOR_LETTERS.put("black", 'B');
        COLOR_LETTERS.put("red", 'R');
        COLOR_LETTERS.put("green", 'G');
    }

    private static boolean matchesManaValue(CardInfo info, Integer manaValue, String operator) {
        if (manaValue == null || operator == null) {
            return true;
        }
        switch (operator) {
            case "lte":
                return info.getManaValue() <= manaValue;
            case "gte":
                return info.getManaValue() >= manaValue;
            default:
                return true;
        }
    }

    private static boolean matchesColors(Set<String> cardColors, List<String> colors, String colorMatch) {
        if (colors.isEmpty()) {
            return true;
        }
        Set<String> requested = normalize(colors);
        if (cardColors.isEmpty()) {
            return requested.contains("colorless");
        }
        requested.remove("colorless");
        if (requested.isEmpty()) {
            return false;
        }
        switch (colorMatch == null ? "any" : colorMatch) {
            case "exact":
                return cardColors.equals(requested);
            case "include":
                return cardColors.containsAll(requested);
            case "any":
            default:
                for (String color : requested) {
                    if (cardColors.contains(color)) {
                        return true;
                    }
                }
                return false;
        }
    }

    private static boolean matchesAnyColor(Set<String> cardColors, List<String> colors) {
        if (colors.isEmpty()) {
            return false;
        }
        Set<String> excluded = normalize(colors);
        if (cardColors.isEmpty()) {
            return excluded.contains("colorless");
        }
        for (String color : excluded) {
            if (cardColors.contains(color)) {
                return true;
            }
        }
        return false;
    }

    private static Set<String> normalize(List<String> colors) {
        return colors.stream()
                .filter(color -> color != null && !color.trim().isEmpty())
                .map(color -> color.trim().toLowerCase(Locale.ENGLISH))
                .collect(Collectors.toSet());
    }

    private static Set<String> colorsOf(CardInfo info) {
        ObjectColor color = info.getColor();
        Set<String> colors = new HashSet<>();
        if (color.isWhite()) colors.add("white");
        if (color.isBlue()) colors.add("blue");
        if (color.isBlack()) colors.add("black");
        if (color.isRed()) colors.add("red");
        if (color.isGreen()) colors.add("green");
        return colors;
    }

    public static class ExpansionSetInfo {
        private final String setCode;
        private final String name;
        private final String blockName;
        private final long releaseDate;
        private final String type;
        private final boolean hasBoosters;
        private final boolean hasBasicLands;

        ExpansionSetInfo(ExpansionInfo info) {
            this.setCode = info.getCode();
            this.name = info.getName();
            this.blockName = info.getBlockName();
            this.releaseDate = info.getReleaseDate() == null ? 0L : info.getReleaseDate().getTime();
            this.type = info.getType() == null ? "" : info.getType().name();
            this.hasBoosters = info.hasBoosters();
            this.hasBasicLands = info.hasBasicLands();
        }
    }

    public static final class BasicLandSetInfo extends ExpansionSetInfo {
        private final boolean hasSnowBasics;

        BasicLandSetInfo(ExpansionInfo info) {
            super(info);
            this.hasSnowBasics = CardRepository.haveSnowLands(info.getCode());
        }
    }
}
