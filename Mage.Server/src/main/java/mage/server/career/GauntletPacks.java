package mage.server.career;

import mage.ObjectColor;
import mage.cards.decks.DeckCardInfo;
import mage.cards.repository.CardCriteria;
import mage.cards.repository.CardInfo;
import mage.cards.repository.CardRepository;
import mage.constants.CardType;
import mage.constants.Rarity;

import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Where a gauntlet run's packs come from (S026): Jumpstart's themed packs, packs of commons (pauper), highlander packs
 * of one copy each (singleton), or packs cut from a cube uploaded to the server. Every source makes one-colour 20-card
 * packs, 13 spells and 7 basic lands, so the run's deck works the same way whatever the variant.
 */
final class GauntletPacks {

    static final String JUMPSTART = "jumpstart";
    static final String PAUPER = "pauper";
    static final String SINGLETON = "singleton";
    static final String CUBE = "cube";
    static final String DRAFT = "draft";

    static final int SPELLS = 13;
    static final int LANDS = 7;
    /** the sets generated packs draw from: Jumpstart's own and the core set it was printed with */
    static final String[] SETS = {"JMP", "M21"};

    private static final String COLORS = "WUBRG";
    private static final Map<Character, String> BASICS = new LinkedHashMap<>();
    private static final Map<Character, String> COLOR_NAMES = new LinkedHashMap<>();

    static {
        BASICS.put('W', "Plains");
        BASICS.put('U', "Island");
        BASICS.put('B', "Swamp");
        BASICS.put('R', "Mountain");
        BASICS.put('G', "Forest");
        COLOR_NAMES.put('W', "White");
        COLOR_NAMES.put('U', "Blue");
        COLOR_NAMES.put('B', "Black");
        COLOR_NAMES.put('R', "Red");
        COLOR_NAMES.put('G', "Green");
    }

    /** a cube's card names: a deck sent as the cube ("deck:" ids, held by the run), or one of the server's draft cubes */
    static List<String> cubeCards(String cubeId, List<String> deckCube) {
        if (deckCube != null && !deckCube.isEmpty()) {
            return deckCube;
        }
        if (cubeId == null) {
            return null;
        }
        try {
            mage.game.draft.DraftCube cube = mage.server.draft.CubeFactory.instance.createDraftCube(cubeId);
            if (cube == null) {
                return null;
            }
            List<String> names = new ArrayList<>();
            for (mage.game.draft.DraftCube.CardIdentity card : cube.getCubeCards()) {
                names.add(card.getName());
            }
            return names;
        } catch (RuntimeException e) {
            return null;
        }
    }

    /** spells by variant and colour, read once from the card database */
    private static final Map<String, List<CardInfo>> spells = new ConcurrentHashMap<>();

    private GauntletPacks() {
    }

    static boolean known(String variant) {
        return JUMPSTART.equals(variant) || PAUPER.equals(variant) || SINGLETON.equals(variant) || CUBE.equals(variant) || DRAFT.equals(variant);
    }

    /** {@code count} packs of different colours to start a run with */
    static List<CareerJumpstart.Pack> start(String variant, String cubeId, List<String> deckCube, Random random, int count) throws CareerService.CareerException {
        if (JUMPSTART.equals(variant)) {
            List<CareerJumpstart.Pack> packs = new ArrayList<>(CareerJumpstart.get().packs());
            Collections.shuffle(packs, random);
            return new ArrayList<>(packs.subList(0, Math.min(count, packs.size())));
        }
        List<Character> colors = new ArrayList<>();
        for (char color : COLORS.toCharArray()) {
            if (!pool(variant, cubeId, deckCube, color).isEmpty()) {
                colors.add(color);
            }
        }
        if (colors.size() < 2) {
            throw new CareerService.CareerException(CUBE.equals(variant)
                    ? "That cube doesn't have two colours of cards to make packs from."
                    : "The card database doesn't have the cards for " + variant + " packs.");
        }
        Collections.shuffle(colors, random);
        List<CareerJumpstart.Pack> packs = new ArrayList<>();
        for (char color : colors.subList(0, Math.min(count, colors.size()))) {
            packs.add(pack(variant, cubeId, deckCube, color, random));
        }
        return packs;
    }

    /** packs to cut a bundle from, in the run's colours */
    static List<CareerJumpstart.Pack> bundles(String variant, String cubeId, List<String> deckCube, String runColors, Random random, int count) {
        if (JUMPSTART.equals(variant) || DRAFT.equals(variant)) {
            // a drafted deck takes its bundles from Jumpstart packs of its colours
            List<CareerJumpstart.Pack> packs = new ArrayList<>(CareerJumpstart.get().inColors(runColors));
            Collections.shuffle(packs, random);
            return new ArrayList<>(packs.subList(0, Math.min(count, packs.size())));
        }
        List<Character> colors = new ArrayList<>();
        for (char color : (runColors == null || runColors.isEmpty() ? COLORS : runColors).toCharArray()) {
            if (!pool(variant, cubeId, deckCube, color).isEmpty()) {
                colors.add(color);
            }
        }
        List<CareerJumpstart.Pack> packs = new ArrayList<>();
        for (int i = 0; i < count && !colors.isEmpty(); i++) {
            packs.add(pack(variant, cubeId, deckCube, colors.get(random.nextInt(colors.size())), random));
        }
        return packs;
    }

    /** one colour's pack: 13 different spells and 7 of the colour's basic land */
    static CareerJumpstart.Pack pack(String variant, String cubeId, List<String> deckCube, char color, Random random) {
        List<CardInfo> cards = new ArrayList<>(pool(variant, cubeId, deckCube, color));
        Collections.shuffle(cards, random);
        CareerJumpstart.Pack pack = new CareerJumpstart.Pack();
        pack.color = String.valueOf(color);
        pack.name = COLOR_NAMES.get(color) + " " + (PAUPER.equals(variant) ? "commons" : SINGLETON.equals(variant) ? "singles" : "cube") + " (" + (1 + random.nextInt(99)) + ")";
        for (CardInfo card : cards.subList(0, Math.min(SPELLS, cards.size()))) {
            pack.cards.add(new DeckCardInfo(card.getName(), card.getCardNumber(), card.getSetCode(), 1));
        }
        CardInfo basic = CardRepository.instance.findPreferredCoreExpansionCard(BASICS.get(color));
        pack.cards.add(basic == null ? new DeckCardInfo(BASICS.get(color), null, null, LANDS)
                : new DeckCardInfo(basic.getName(), basic.getCardNumber(), basic.getSetCode(), LANDS));
        return pack;
    }

    /** the spells of one colour a variant draws from: mono-coloured and not lands */
    static List<CardInfo> pool(String variant, String cubeId, List<String> deckCube, char color) {
        String key = variant + "|" + (cubeId == null ? "" : cubeId) + "|" + (deckCube == null ? 0 : deckCube.hashCode()) + "|" + color;
        List<CardInfo> cached = spells.get(key);
        if (cached != null) {
            return cached;
        }
        List<CardInfo> found = new ArrayList<>();
        Map<String, CardInfo> byName = new LinkedHashMap<>();
        if (CUBE.equals(variant)) {
            List<String> names = cubeCards(cubeId, deckCube);
            if (names != null) {
                for (String name : names) {
                    CardInfo info = CardRepository.instance.findPreferredCoreExpansionCard(name);
                    if (info != null && isMono(info, color)) {
                        byName.putIfAbsent(info.getName(), info);
                    }
                }
            }
        } else if (PAUPER.equals(variant) || SINGLETON.equals(variant)) {
            CardCriteria criteria = new CardCriteria().setCodes(SETS).notTypes(CardType.LAND);
            if (PAUPER.equals(variant)) {
                criteria.rarities(Rarity.COMMON);
            }
            for (CardInfo info : CardRepository.instance.findCards(criteria)) {
                if (isMono(info, color)) {
                    byName.putIfAbsent(info.getName(), info);
                }
            }
        }
        found.addAll(byName.values());
        found.sort((a, b) -> a.getName().compareTo(b.getName()));
        if (!found.isEmpty() && (!CUBE.equals(variant) || spells.size() < 200)) {
            spells.put(key, found);
        }
        return found;
    }

    private static boolean isMono(CardInfo info, char color) {
        if (info.getTypes().contains(CardType.LAND)) {
            return false;
        }
        ObjectColor colors = info.getColor();
        if (colors == null || colors.getColorCount() != 1) {
            return false;
        }
        switch (color) {
            case 'W':
                return colors.isWhite();
            case 'U':
                return colors.isBlue();
            case 'B':
                return colors.isBlack();
            case 'R':
                return colors.isRed();
            default:
                return colors.isGreen();
        }
    }
}
