package mage.server.career;

import com.google.gson.Gson;
import mage.cards.decks.DeckCardInfo;
import mage.cards.decks.DeckCardLists;
import mage.cards.decks.importer.DeckImporter;
import org.apache.log4j.Logger;

import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Career content shipped with the server, as data: the starter decks a Career can open with, and the AI opponent
 * roster in tiers (resources/career). Adding an opponent is a line in opponents.json and a .dck file.
 */
public final class CareerContent {

    private static final Logger logger = Logger.getLogger(CareerContent.class);
    private static final String ROOT = "/career/";

    public static class Tier {
        public int tier;
        public String name;
        /** the server AI's skill for this tier's opponents (1-10) */
        public int skill;
    }

    public static class Opponent {
        public String id;
        public String name;
        public int tier;
        /** WUBRG letters */
        public String colors;
        public String deck;
        public String tagline;
        /** the server AI type; the usual one when absent */
        public String aiType;
        /** overrides the tier's skill */
        public Integer skill;
        /** the card whose art is the opponent's portrait; picked from the deck when absent */
        public CareerCover cover;
        /** what the opponent says during a game */
        public CareerLines lines;
        /** the starter decks' measured win rate against this opponent, in percent (CALIBRATION.md); null if unmeasured */
        public Integer measured;
    }

    /** An AI opponent's few lines: as the game starts, after a big play, at low life, and as it ends either way. */
    public static class CareerLines {
        public String intro;
        public String bigPlay;
        public String lowLife;
        /** the opponent won */
        public String win;
        /** the opponent lost */
        public String lose;
    }

    public static class CareerStarter {
        public String id;
        public String name;
        public int cards;
        public CareerCover cover;
        /** WUBRG letters, most basic lands first: the colors the deck plays */
        public String colors;
    }

    public static class CareerCover {
        public String name;
        public String setCode;
        public String cardNumber;
    }

    private static class Roster {
        List<Tier> tiers;
        List<Opponent> opponents;
    }

    private static class StarterEntry {
        String file;
        String name;
        int cards;
        CareerCover cover;
    }

    private static volatile CareerContent instance;

    private final List<Tier> tiers;
    private final Map<String, Opponent> opponents = new LinkedHashMap<>();
    private final Map<String, CareerStarter> starters = new LinkedHashMap<>();

    private CareerContent() throws IOException {
        Gson gson = new Gson();
        Roster roster;
        try (Reader reader = resource("opponents.json")) {
            roster = gson.fromJson(reader, Roster.class);
        }
        tiers = Collections.unmodifiableList(roster.tiers);
        for (Opponent opponent : roster.opponents) {
            opponents.put(opponent.id, opponent);
        }
        for (Opponent opponent : mage.server.solo.ContentPacks.get().opponents()) {
            opponents.putIfAbsent(opponent.id, opponent);
        }
        StarterEntry[] index;
        try (Reader reader = resource("starters/index.json")) {
            index = gson.fromJson(reader, StarterEntry[].class);
        }
        for (StarterEntry entry : index) {
            // a Career is played with 60-card decks; the commander starters stay in free play
            if (entry.cards != 60) {
                continue;
            }
            CareerStarter starter = new CareerStarter();
            starter.id = entry.file.replaceAll("\\.dck$", "");
            starter.name = entry.name;
            starter.cards = entry.cards;
            starter.cover = entry.cover;
            starter.colors = basicLandColors("starters/" + entry.file);
            starters.put(starter.id, starter);
        }
    }

    public static CareerContent get() {
        CareerContent current = instance;
        if (current == null) {
            synchronized (CareerContent.class) {
                current = instance;
                if (current == null) {
                    try {
                        current = new CareerContent();
                    } catch (IOException | RuntimeException e) {
                        logger.error("Career content can't be read", e);
                        throw new IllegalStateException("Career content is missing from this server");
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    /** the roster the server ships with: tiers and opponent ids */
    static List<Tier> builtInTiers() throws IOException {
        try (Reader reader = resource("opponents.json")) {
            return new Gson().fromJson(reader, Roster.class).tiers;
        }
    }

    static List<String> builtInOpponentIds() throws IOException {
        try (Reader reader = resource("opponents.json")) {
            List<String> ids = new java.util.ArrayList<>();
            for (Opponent opponent : new Gson().fromJson(reader, Roster.class).opponents) {
                ids.add(opponent.id);
            }
            return ids;
        }
    }

    static void forget() {
        instance = null;
        covers.clear();
    }

    private static Reader resource(String path) throws IOException {
        InputStream stream = CareerContent.class.getResourceAsStream(ROOT + path);
        if (stream == null) {
            throw new IOException("Missing career resource " + path);
        }
        return new InputStreamReader(stream, StandardCharsets.UTF_8);
    }

    /** AI seats take the opponent's whole name ("Wren of the Hedgerow"): the server's name length limit is for people */
    public static final int MAX_AI_NAME_LENGTH = 40;

    public static String aiSeatName(String opponentName) {
        return opponentName.length() > MAX_AI_NAME_LENGTH ? opponentName.substring(0, MAX_AI_NAME_LENGTH).trim() : opponentName;
    }

    /** the measured starter win rate against an opponent: higher is easier; 50 when it wasn't measured */
    public static int measuredWinRate(Opponent opponent) {
        return opponent.measured == null ? 50 : opponent.measured;
    }

    public List<Tier> tiers() {
        return tiers;
    }

    public Tier tier(int tier) {
        return tiers.stream().filter(candidate -> candidate.tier == tier).findFirst().orElse(tiers.get(tiers.size() - 1));
    }

    public List<Opponent> opponents() {
        return new ArrayList<>(opponents.values());
    }

    public Opponent opponent(String id) {
        return opponents.get(id);
    }

    public List<CareerStarter> starters() {
        return new ArrayList<>(starters.values());
    }

    public CareerStarter starter(String id) {
        return starters.get(id);
    }

    public DeckCardLists starterDeck(String id) throws IOException {
        return deck("starters/" + id + ".dck");
    }

    public DeckCardLists opponentDeck(Opponent opponent) throws IOException {
        return deck("opponents/" + opponent.deck);
    }

    private static final Map<String, java.util.Optional<CareerCover>> covers = new java.util.concurrent.ConcurrentHashMap<>();

    /**
     * The card a deck shows as its face: of the cards that aren't lands, the rarest, then the most expensive, then the
     * one with the most copies. Null without a card database (the bridge's own tests) or for a deck that can't be read.
     */
    public static CareerCover coverOf(String path) {
        return covers.computeIfAbsent(path, key -> {
            try {
                return java.util.Optional.ofNullable(pickCover(deck(key)));
            } catch (IOException | RuntimeException e) {
                return java.util.Optional.empty();
            }
        }).orElse(null);
    }

    /** a card's face by its name, for a node without a deck across the table (a trial shows its lesson's first card) */
    public static CareerCover coverOfCard(String name) {
        return covers.computeIfAbsent("card:" + name, key -> {
            try {
                List<mage.cards.repository.CardInfo> found = mage.cards.repository.CardRepository.instance.findCards(name);
                if (found.isEmpty()) {
                    return java.util.Optional.empty();
                }
                CareerCover cover = new CareerCover();
                cover.name = found.get(0).getName();
                cover.setCode = found.get(0).getSetCode();
                cover.cardNumber = found.get(0).getCardNumber();
                return java.util.Optional.of(cover);
            } catch (RuntimeException e) {
                return java.util.Optional.empty();
            }
        }).orElse(null);
    }

    static CareerCover pickCover(DeckCardLists deck) {
        DeckCardInfo best = null;
        int[] bestScore = null;
        // a Commander or Brawl deck's face is its commander, kept in the sideboard
        int sideboard = deck.getSideboard().stream().mapToInt(DeckCardInfo::getAmount).sum();
        int main = deck.getCards().stream().mapToInt(DeckCardInfo::getAmount).sum();
        boolean commander = sideboard >= 1 && sideboard <= 2 && (main == 99 || main == 98 || main == 59 || main == 58);
        for (DeckCardInfo card : commander ? deck.getSideboard() : deck.getCards()) {
            List<mage.cards.repository.CardInfo> found = mage.cards.repository.CardRepository.instance.findCards(card.getCardName());
            if (found.isEmpty()) {
                continue;
            }
            mage.cards.repository.CardInfo info = found.get(0);
            if (info.getTypes().contains(mage.constants.CardType.LAND)) {
                continue;
            }
            int rarity = info.getRarity() == null ? 0 : Math.min(info.getRarity().ordinal(), mage.constants.Rarity.MYTHIC.ordinal());
            int[] score = {rarity, info.getManaValue(), card.getAmount()};
            if (bestScore == null || ahead(score, bestScore)) {
                best = card;
                bestScore = score;
            }
        }
        if (best == null) {
            return null;
        }
        CareerCover cover = new CareerCover();
        cover.name = best.getCardName();
        cover.setCode = best.getSetCode();
        cover.cardNumber = best.getCardNumber();
        return cover;
    }

    /** whether a score beats another, field by field */
    private static boolean ahead(int[] score, int[] other) {
        for (int i = 0; i < score.length; i++) {
            if (score[i] != other[i]) {
                return score[i] > other[i];
            }
        }
        return false;
    }

    /** an opponent's portrait card: the one its entry names, or one picked from its deck */
    public CareerCover cover(Opponent opponent) {
        return opponent.cover != null ? opponent.cover : coverOf("opponents/" + opponent.deck);
    }

    /** reads a .dck file shipped in resources (the importer reads files, so it goes through a temporary copy) */
    static DeckCardLists deck(String path) throws IOException {
        if (mage.server.solo.ContentPacks.isPackDeck(path)) {
            return mage.server.solo.ContentPacks.get().deck(path);
        }
        InputStream stream = CareerContent.class.getResourceAsStream(ROOT + path);
        if (stream == null) {
            throw new IOException("Missing career deck " + path);
        }
        Path file = Files.createTempFile("career-", ".dck");
        try {
            try (InputStream in = stream) {
                Files.copy(in, file, StandardCopyOption.REPLACE_EXISTING);
            }
            StringBuilder errors = new StringBuilder();
            DeckCardLists deck = DeckImporter.importDeckFromFile(file.toString(), errors, false);
            if ((deck == null || deck.getCards().isEmpty()) && mage.cards.repository.CardRepository.instance.findCards("Plains").isEmpty()) {
                // no card database (the bridge's own tests): take the file as written
                deck = readAsWritten(file);
            }
            if (deck == null || deck.getCards().isEmpty()) {
                throw new IOException("Career deck " + path + " can't be read: " + errors);
            }
            if (errors.length() > 0) {
                logger.warn("Career deck " + path + ": " + errors);
            }
            return deck;
        } finally {
            Files.deleteIfExists(file);
        }
    }

    /**
     * Reads a .dck file on disk the way Career decks are read (content packs): without a card database, as written.
     *
     * @throws IOException when it has no cards, or names cards the database doesn't know
     */
    public static DeckCardLists readDeckFile(Path file) throws IOException {
        StringBuilder errors = new StringBuilder();
        DeckCardLists deck = DeckImporter.importDeckFromFile(file.toString(), errors, false);
        boolean database = !mage.cards.repository.CardRepository.instance.findCards("Plains").isEmpty();
        if ((deck == null || deck.getCards().isEmpty()) && !database) {
            deck = readAsWritten(file);
        }
        if (deck == null || deck.getCards().isEmpty()) {
            throw new IOException("has no cards the server knows" + (errors.length() > 0 ? ": " + errors.toString().trim() : ""));
        }
        if (errors.length() > 0 && database) {
            throw new IOException(errors.toString().trim());
        }
        return deck;
    }

    private static final String[][] BASIC_LANDS = {{"Plains", "W"}, {"Island", "U"}, {"Swamp", "B"}, {"Mountain", "R"}, {"Forest", "G"}};

    /** a deck's colors read from its basic lands (no card database needed): WUBRG letters, the most lands first */
    static String basicLandColors(String path) throws IOException {
        Map<String, Integer> counts = new LinkedHashMap<>();
        try (java.io.BufferedReader reader = new java.io.BufferedReader(resource(path))) {
            String line;
            while ((line = reader.readLine()) != null) {
                java.util.regex.Matcher matcher = DCK_LINE.matcher(line.trim());
                if (!matcher.matches() || matcher.group(1) != null) {
                    continue;
                }
                for (String[] basic : BASIC_LANDS) {
                    if (basic[0].equals(matcher.group(5).trim())) {
                        counts.merge(basic[1], Integer.parseInt(matcher.group(2)), Integer::sum);
                    }
                }
            }
        }
        StringBuilder colors = new StringBuilder();
        counts.entrySet().stream()
                .sorted((a, b) -> b.getValue() - a.getValue())
                .forEach(entry -> colors.append(entry.getKey()));
        return colors.toString();
    }

    private static final java.util.regex.Pattern DCK_LINE = java.util.regex.Pattern.compile("^(SB:\\s*)?(\\d+)\\s+\\[([^:\\]]+):([^\\]]+)]\\s+(.+)$");

    private static DeckCardLists readAsWritten(Path file) throws IOException {
        DeckCardLists deck = new DeckCardLists();
        for (String line : Files.readAllLines(file)) {
            line = line.trim();
            if (line.startsWith("NAME:")) {
                deck.setName(line.substring(5).trim());
                continue;
            }
            java.util.regex.Matcher matcher = DCK_LINE.matcher(line);
            if (matcher.matches()) {
                DeckCardInfo card = new DeckCardInfo(matcher.group(5).trim(), matcher.group(4), matcher.group(3), Integer.parseInt(matcher.group(2)));
                (matcher.group(1) == null ? deck.getCards() : deck.getSideboard()).add(card);
            }
        }
        return deck;
    }

    /** every card of a deck, main deck and sideboard */
    static List<DeckCardInfo> allCards(DeckCardLists deck) {
        List<DeckCardInfo> cards = new ArrayList<>(deck.getCards());
        cards.addAll(deck.getSideboard());
        return cards;
    }
}
