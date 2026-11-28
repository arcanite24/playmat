package mage.server.career;

import com.google.gson.Gson;
import mage.cards.Card;
import mage.cards.ExpansionSet;
import mage.cards.Sets;
import mage.cards.decks.DeckCardInfo;
import mage.cards.decks.DeckCardLists;
import mage.cards.repository.CardInfo;
import mage.cards.repository.CardRepository;
import mage.constants.SetType;
import mage.constants.SuperType;
import org.apache.log4j.Logger;

import javax.crypto.Mac;
import javax.crypto.spec.SecretKeySpec;
import java.nio.charset.StandardCharsets;
import java.security.GeneralSecurityException;
import java.security.MessageDigest;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Base64;
import java.util.Collections;
import java.util.Comparator;
import java.util.Date;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.Function;
import java.util.stream.Collectors;

/**
 * Career rules over the store: opening a Career, owning cards, paying out matches once, the shop, crafting, and
 * export files. Off unless the server has accounts and XMAGE_CAREER (or -Dxmage.career) isn't "off".
 */
public final class CareerService {

    private static final Logger logger = Logger.getLogger(CareerService.class);
    private static final CareerService INSTANCE = new CareerService();
    private static final Set<String> BASIC_NAMES = new java.util.HashSet<>(java.util.Arrays.asList(
            "Plains", "Island", "Swamp", "Mountain", "Forest", "Wastes",
            "Snow-Covered Plains", "Snow-Covered Island", "Snow-Covered Swamp", "Snow-Covered Mountain", "Snow-Covered Forest",
            "Snow-Covered Wastes"));

    /** Career tables running now: table id to who plays whom */
    private final Map<UUID, CareerTable> tables = new ConcurrentHashMap<>();
    private volatile Function<String, CardInfo> cardLookup = CareerService::lookupPrinting;
    /** each set's card names (no basic lands, only what its packs can hold), read once */
    private final Map<String, Set<String>> setNames = new ConcurrentHashMap<>();

    public static class CareerTable implements java.io.Serializable {
        private static final long serialVersionUID = 1L;
        public final String user;
        /** the roster opponent's id, or the mode and its reference ("campaign:five-paths/white-1") */
        public final String opponentId;
        public final int tier;
        /** "duel" (the roster), "campaign", "trial" (a campaign's set position), "gauntlet", "puzzle", "challenge" or "limited" */
        public final String kind;
        /** what the mode needs to finish the game: a campaign node, a run id, a puzzle id, a week */
        public final String ref;
        /** how the game starts, or null for a plain duel */
        public final CareerSetup setup;
        /** hints the player asked for in this game (S010) */
        public volatile int hintsUsed;
        /** turns the player took back in this game (S011) */
        public volatile int takeBacks;
        /** what helped the player ("a hint", "a take-back"); an assisted game pays half and counts for no quests */
        public volatile String assisted;
        /** when the table was made: a timed mode (the daily puzzle) counts from here */
        public final long created = System.currentTimeMillis();

        /** whether the player may get help here: a puzzle or a trial is a test of its own, with its own retries */
        public boolean allowsAssists() {
            return !"puzzle".equals(kind) && !"trial".equals(kind);
        }

        CareerTable(String user, String opponentId, int tier) {
            this(user, opponentId, tier, "duel", null, null);
        }

        CareerTable(String user, String opponentId, int tier, String kind, String ref, CareerSetup setup) {
            this.user = user;
            this.opponentId = opponentId;
            this.tier = tier;
            this.kind = kind;
            this.ref = ref;
            this.setup = setup;
        }
    }

    /** One opponent of the roster as a player sees it. */
    public static class CareerOpponent {
        public String id;
        public String name;
        public int tier;
        public String tierName;
        public String colors;
        public String tagline;
        public boolean unlocked;
        public int wins;
        /** coins a win pays */
        public int winCoins;
        /** the card whose art is the opponent's portrait */
        public CareerContent.CareerCover cover;
        /** what the opponent says during a game */
        public CareerContent.CareerLines lines;
        /** how hard it measured, 1 to 5 (S039) */
        public int pips;
        /** what the opponent starts with: the tier's edge, its own, and a nemesis's (S037, S040, S048) */
        public CareerSetup.CareerTwists twists;
        /** the player's nemesis: an edge for them, a bounty for you (S040) */
        public boolean nemesis;
        public int bounty;
        /** the rival arc's chapter (S041), and what the rival says before it */
        public Integer rivalChapter;
        public String rivalBefore;
        /** a raid boss (S048) */
        public boolean raid;
        /** how the opponent plays, in words (S042): "aggressive", "patient", "reckless", "careful" */
        public List<String> style = new ArrayList<>();
        /** lists the opponent rotates through (S038) */
        public int lists;
        /** the AI: "archmage" when tier five's hybrid AI is meant to play it (S050) */
        public String aiType;
        /** "constructed" or "commander" */
        public String format;
        /** true for 4-player Commander pod encounters */
        public boolean pod;
        /** display names of the 3 AI pod opponents, when pod is true */
        public List<String> podNames;
    }

    /** How much of a set the collection has: different cards owned (any printing) of the set's cards. */
    public static class CareerSetProgress {
        public String setCode;
        public String name;
        public int owned;
        public int total;
    }

    /** A set the shop sells packs of. */
    public static class CareerShopSet {
        public String setCode;
        public String name;
        public String releaseDate;
        public int price;
        /** "standard" (core & expansion), "commander" (Commander Legends / Battlebond), or "masters" (Horizons / Remastered) */
        public String category;
        /** a campaign chapter has to open it first */
        public boolean locked;
        /** the campaign chapter that opens it (to beat while locked, beaten once open); null for the newest sets */
        public String unlockedBy;
        /** the card whose art is on the set's packs; null without a card database */
        public CareerContent.CareerCover cover;
    }

    /** What claiming or buying a Commander precon gave. */
    public static class CareerPreconResult {
        public String starterId;
        public boolean free;
        public CareerSavedDeck deck;
        public CareerProfile profile;
    }

    /** What a pack gave, and the Career afterwards. */
    public static class CareerPackResult {
        public String setCode;
        public List<CareerCard> cards;
        public CareerProfile profile;
    }

    private CareerService() {
    }

    public static CareerService get() {
        return INSTANCE;
    }

    /** the server switch: Career runs only with accounts, and can be turned off (for good from the environment) */
    public static boolean enabled(boolean accounts) {
        return accounts && !offByEnvironment() && mage.server.admin.RuntimeSettings.get().careerOn();
    }

    public static boolean offByEnvironment() {
        String setting = System.getProperty("xmage.career", System.getenv("XMAGE_CAREER"));
        return "off".equalsIgnoreCase(setting == null ? "" : setting.trim());
    }

    /** for tests: resolve cards without the card database (null puts the database back) */
    void useCardLookup(Function<String, CardInfo> lookup) {
        cardLookup = lookup == null ? CareerService::lookupPrinting : lookup;
    }

    // ---- opening a Career

    public CareerProfile start(String user, String starterId) throws SQLException, CareerException {
        CareerContent content = CareerContent.get();
        if (content.starter(starterId) == null) {
            throw new CareerException("There is no starter deck " + starterId + ".");
        }
        DeckCardLists deck;
        try {
            deck = content.starterDeck(starterId);
        } catch (java.io.IOException e) {
            throw new CareerException("The starter deck can't be read: " + e.getMessage());
        }
        CareerStore store = CareerStore.get();
        boolean isCommander = content.isCommanderStarter(starterId);
        boolean created = store.transaction(() -> {
            // progression made the profile with the account's first counted game; picking a starter opens the collection
            if (!store.createProfile(user, starterId, CareerRules.START_COINS, System.currentTimeMillis())
                    && !store.openCollection(user, starterId, CareerRules.START_COINS)) {
                return false;
            }
            if (isCommander) {
                store.setQuestPref(user, "commander");
            }
            for (DeckCardInfo info : CareerContent.allCards(deck)) {
                CareerCard card = resolve(info);
                if (card != null && !CareerRules.LAND.equals(card.rarity)) {
                    store.addCard(user, card, info.getAmount());
                }
            }
            return true;
        });
        if (!created) {
            throw new CareerException("You already have a Career.");
        }
        return store.profile(user);
    }

    /** True when an existing non-Commander Career account can still claim one free 100-card Commander precon. */
    public boolean commanderPreconGiftAvailable(String user) throws SQLException {
        CareerStore store = CareerStore.get();
        CareerProfile profile = store.profile(user);
        if (profile == null || !profile.collection) {
            return false;
        }
        if (CareerContent.get().isCommanderStarter(profile.starter)) {
            return false;
        }
        return store.stats(user).getOrDefault("commander_precon_gift", 0) == 0;
    }

    /** Commander starter IDs this account already owns (from starting Career with one or acquiring a precon). */
    public List<String> ownedPrecons(String user) throws SQLException {
        CareerStore store = CareerStore.get();
        CareerProfile profile = store.profile(user);
        if (profile == null || !profile.collection) {
            return Collections.emptyList();
        }
        Map<String, Integer> stats = store.stats(user);
        List<String> owned = new ArrayList<>();
        for (CareerContent.CareerStarter starter : CareerContent.get().starters()) {
            if (!"commander".equals(starter.track)) {
                continue;
            }
            if (starter.id.equals(profile.starter)
                    || stats.getOrDefault("precon:" + starter.id, 0) > 0
                    || store.careerDeckBySource(user, "starter:" + starter.id) != null) {
                owned.add(starter.id);
            }
        }
        return owned;
    }

    /**
     * Claims a 100-card Commander precon for free (one-time gift for existing non-Commander accounts) or buys an
     * additional Commander precon bundle for {@link CareerRules#PRECON_PRICE} coins. Adds every non-basic card (up to
     * playset) to the collection and saves an editable Commander deck in {@code career_decks}.
     */
    public CareerPreconResult acquirePrecon(String user, String starterId) throws SQLException, CareerException {
        CareerContent content = CareerContent.get();
        CareerContent.CareerStarter starter = content.starter(starterId);
        if (starter == null || !"commander".equals(starter.track)) {
            throw new CareerException("There is no Commander precon " + starterId + ".");
        }
        DeckCardLists deck;
        try {
            deck = content.starterDeck(starterId);
        } catch (java.io.IOException e) {
            throw new CareerException("The Commander precon can't be read: " + e.getMessage());
        }
        CareerStore store = CareerStore.get();
        long now = System.currentTimeMillis();
        boolean[] wasFree = new boolean[1];
        CareerStore.CareerDeckRow[] savedRow = new CareerStore.CareerDeckRow[1];
        Gson gson = new Gson();
        refusing(() -> store.transaction(() -> {
            CareerProfile profile = store.profile(user);
            if (profile == null || !profile.collection) {
                throw new SQLException(new CareerException("Open your Career collection first."));
            }
            ensureSavedDecksInitialized(store, user, profile, now);
            Map<String, Integer> stats = store.stats(user);
            String source = "starter:" + starterId;
            if (starterId.equals(profile.starter)
                    || stats.getOrDefault("precon:" + starterId, 0) > 0
                    || store.careerDeckBySource(user, source) != null) {
                throw new SQLException(new CareerException("You already own " + starter.name + "."));
            }
            boolean free = !content.isCommanderStarter(profile.starter)
                    && stats.getOrDefault("commander_precon_gift", 0) == 0;
            if (!free) {
                if (profile.coins < CareerRules.PRECON_PRICE) {
                    throw new SQLException(new CareerException(
                            starter.name + " costs " + CareerRules.PRECON_PRICE + " coins; you have " + profile.coins + "."));
                }
                store.addCoins(user, -CareerRules.PRECON_PRICE);
            } else {
                store.addStat(user, "commander_precon_gift", 1);
            }
            wasFree[0] = free;
            for (DeckCardInfo info : CareerContent.allCards(deck)) {
                CareerCard card = resolve(info);
                if (card == null || CareerRules.LAND.equals(card.rarity)) {
                    continue;
                }
                int have = store.owned(user, card.name);
                int room = Math.max(0, CareerRules.PLAYSET - have);
                int add = Math.min(info.getAmount(), room);
                if (add > 0) {
                    store.addCard(user, card, add);
                }
            }
            store.addStat(user, "precon:" + starterId, 1);
            String cleanName = cleanDeckName(starter.name, deck.getName());
            DeckCardLists copy = copyDeckWithName(deck, cleanName);
            CareerStore.CareerDeckRow row = new CareerStore.CareerDeckRow();
            row.id = UUID.randomUUID().toString();
            row.name = cleanName;
            row.deck = gson.toJson(copy);
            row.source = source;
            row.format = CareerRules.FORMAT_COMMANDER;
            row.active = store.activeCareerDeck(user) == null;
            row.updated = now;
            if (store.careerDeckCount(user) < MAX_SAVED_DECKS) {
                store.putCareerDeck(user, row);
                if (row.active) {
                    store.putDeck(user, row.deck, now);
                }
                savedRow[0] = store.careerDeck(user, row.id);
            }
            return null;
        }));
        CareerPreconResult result = new CareerPreconResult();
        result.starterId = starterId;
        result.free = wasFree[0];
        result.profile = store.profile(user);
        if (savedRow[0] != null) {
            result.deck = toSavedDeckView(savedRow[0], ownedCounts(store, user), gson);
        }
        return result;
    }

    /** Sets the account's quest track preference ("any", "constructed", or "commander"). */
    public String setQuestPref(String user, String pref) throws SQLException {
        CareerStore store = CareerStore.get();
        return store.transaction(() -> {
            ensureProfile(store, user, System.currentTimeMillis());
            store.setQuestPref(user, pref);
            return store.questPref(user);
        });
    }

    /**
     * What {@link #start} adds for each starter deck, for the Rust platform service (CareerViewsExport): the cards
     * other than basic lands, resolved as start resolves them, with their amounts. A deck that can't be read, or has
     * a card the card database doesn't know (not loaded yet, say), is left out, and Java answers for it.
     */
    public Map<String, List<Map<String, Object>>> starterCardsExport() {
        CareerContent content = CareerContent.get();
        Map<String, List<Map<String, Object>>> starters = new LinkedHashMap<>();
        for (CareerContent.CareerStarter starter : content.starters()) {
            DeckCardLists deck;
            try {
                deck = content.starterDeck(starter.id);
            } catch (java.io.IOException e) {
                continue;
            }
            List<Map<String, Object>> cards = new ArrayList<>();
            boolean whole = true;
            for (DeckCardInfo info : CareerContent.allCards(deck)) {
                CareerCard card = resolve(info);
                whole &= card != null;
                if (card != null && !CareerRules.LAND.equals(card.rarity)) {
                    Map<String, Object> entry = new LinkedHashMap<>();
                    entry.put("name", card.name);
                    entry.put("setCode", card.setCode);
                    entry.put("cardNumber", card.cardNumber);
                    entry.put("rarity", card.rarity);
                    entry.put("amount", info.getAmount());
                    cards.add(entry);
                }
            }
            if (whole && !cards.isEmpty()) {
                starters.put(starter.id, cards);
            }
        }
        return starters;
    }

    /**
     * The account's progression profile, made on its first counted game (B264): levels, quests and achievements need
     * no starter deck. Runs inside the caller's transaction.
     */
    static void ensureProfile(CareerStore store, String user, long now) throws SQLException {
        if (store.profile(user) == null) {
            store.createProfile(user, "", 0, now);
        }
    }

    /** the account's profile, made now if it has none: opening the Career's pages starts progression too */
    public CareerProfile profileOrNew(String user) throws SQLException {
        CareerStore store = CareerStore.get();
        return store.transaction(() -> {
            ensureProfile(store, user, System.currentTimeMillis());
            return store.profile(user);
        });
    }

    /** the Career card for a deck line: its printing, or the preferred printing of its name; null when unknown */
    CareerCard resolveCard(DeckCardInfo info) {
        return resolve(info);
    }

    private CareerCard resolve(DeckCardInfo info) {
        CardInfo card = info.getSetCode() != null && info.getCardNumber() != null
                ? cardLookup.apply(info.getSetCode() + "|" + info.getCardNumber())
                : null;
        if (card == null) {
            card = cardLookup.apply("name|" + info.getCardName());
        }
        if (card == null) {
            logger.warn("Career: no card found for " + info.getCardName());
            return null;
        }
        return toCareerCard(card);
    }

    static CareerCard toCareerCard(CardInfo card) {
        boolean basic = card.getSupertypes().contains(SuperType.BASIC) || BASIC_NAMES.contains(card.getName());
        String rarity = basic ? CareerRules.LAND : CareerRules.rarityOf(card.getRarity());
        return new CareerCard(card.getName(), card.getSetCode(), card.getCardNumber(), rarity);
    }

    private static CardInfo lookupPrinting(String key) {
        if (key.startsWith("name|")) {
            return CardRepository.instance.findPreferredCoreExpansionCard(key.substring(5));
        }
        String[] parts = key.split("\\|", 2);
        return CardRepository.instance.findCard(parts[0], parts[1]);
    }

    // ---- owning cards

    public static boolean isBasic(String name) {
        return BASIC_NAMES.contains(name);
    }

    /**
     * Cards the deck has more copies of than the account owns, as "Name: 3 in the deck, 2 owned" lines. Basic lands are
     * free. Empty when the deck can be played.
     */
    public List<String> ownershipProblems(String user, DeckCardLists deck) throws SQLException {
        Map<String, Integer> needed = new LinkedHashMap<>();
        for (DeckCardInfo info : CareerContent.allCards(deck)) {
            if (!isBasic(info.getCardName())) {
                needed.merge(info.getCardName(), info.getAmount(), Integer::sum);
            }
        }
        List<String> problems = new ArrayList<>();
        CareerStore store = CareerStore.get();
        for (Map.Entry<String, Integer> entry : needed.entrySet()) {
            int owned = store.owned(user, entry.getKey());
            if (owned < entry.getValue()) {
                problems.add(entry.getKey() + ": " + entry.getValue() + " in the deck, " + owned + " owned");
            }
        }
        return problems;
    }

    // ---- opponents

    public List<CareerOpponent> opponents(String user) throws SQLException {
        CareerContent content = CareerContent.get();
        Map<String, Integer> wins = CareerStore.get().winsByOpponent(user);
        Map<Integer, Integer> winsByTier = new LinkedHashMap<>();
        for (CareerContent.Opponent opponent : content.opponents()) {
            winsByTier.merge(opponent.tier, wins.getOrDefault(opponent.id, 0), Integer::sum);
        }
        List<CareerOpponent> views = new ArrayList<>();
        CareerLadder.Nemesis nemesis = CareerLadder.nemesis(user);
        for (CareerContent.Opponent opponent : ordered(content)) {
            views.add(opponentView(content, opponent, wins, winsByTier, nemesis != null && opponent.id.equals(nemesis.opponentId)));
        }
        return views;
    }

    /** each tier in order of measured difficulty, easiest first (S039); the rival after the rest of its tier */
    public static List<CareerContent.Opponent> ordered(CareerContent content) {
        List<CareerContent.Opponent> ordered = new ArrayList<>(content.opponents());
        ordered.sort(java.util.Comparator.comparingInt((CareerContent.Opponent opponent) -> opponent.tier)
                .thenComparing(opponent -> opponent.rival != null)
                .thenComparing(opponent -> opponent.raid)
                .thenComparingInt(opponent -> -CareerContent.measuredWinRate(opponent)));
        return ordered;
    }

    /** an opponent as a player sees it, with their wins against each opponent and each tier, and whether it's their nemesis */
    public static CareerOpponent opponentView(CareerContent content, CareerContent.Opponent opponent, Map<String, Integer> wins,
                                              Map<Integer, Integer> winsByTier, boolean nemesis) {
        CareerOpponent view = new CareerOpponent();
        view.id = opponent.id;
        view.name = opponent.name;
        view.tier = opponent.tier;
        view.tierName = content.tier(opponent.tier).name;
        view.colors = opponent.colors;
        view.tagline = opponent.tagline;
        view.wins = wins.getOrDefault(opponent.id, 0);
        view.unlocked = (opponent.tier == 1 || winsByTier.getOrDefault(opponent.tier - 1, 0) >= CareerRules.WINS_TO_UNLOCK_NEXT_TIER)
                && CareerLadder.rivalOpen(opponent, wins);
        view.winCoins = CareerRules.winCoins(opponent.tier);
        view.cover = content.cover(opponent);
        view.lines = opponent.lines;
        view.pips = CareerLadder.pips(opponent);
        view.nemesis = nemesis;
        view.bounty = view.nemesis ? CareerLadder.NEMESIS_BOUNTY : 0;
        if (opponent.rival != null) {
            view.rivalChapter = opponent.rival.chapter;
            view.rivalBefore = opponent.rival.before;
        }
        view.raid = opponent.raid;
        view.lists = opponent.decks == null ? 1 : opponent.decks.size();
        CareerContent.Tier tier = content.tier(opponent.tier);
        view.aiType = opponent.aiType != null ? opponent.aiType : tier.aiType;
        CareerSetup.CareerTwists twists = CareerSetup.combine(tier.twists, opponent.twists);
        if (view.nemesis) {
            CareerSetup.CareerTwists edge = new CareerSetup.CareerTwists();
            edge.lifeBonus = CareerLadder.NEMESIS_LIFE;
            twists = CareerSetup.combine(twists, edge);
        }
        view.twists = CareerSetup.empty(twists) ? null : twists;
        if (opponent.personality != null) {
            if (opponent.personality.aggression >= 40) {
                view.style.add("aggressive");
            } else if (opponent.personality.aggression <= -20) {
                view.style.add("defensive");
            }
            if (opponent.personality.risk >= 40) {
                view.style.add("reckless");
            } else if (opponent.personality.risk <= -20) {
                view.style.add("careful");
            }
            if (opponent.personality.patience >= 40) {
                view.style.add("patient");
            }
        }
        return view;
    }

    public List<CareerOpponent> commanderOpponents(String user) throws SQLException {
        CareerContent content = CareerContent.get();
        List<CareerContent.Opponent> roster = content.commanderOpponents();
        Map<String, Integer> wins = CareerStore.get().winsByOpponent(user);
        Map<Integer, Integer> winsByTier = new LinkedHashMap<>();
        for (CareerContent.Opponent opponent : roster) {
            winsByTier.merge(opponent.tier, wins.getOrDefault(opponent.id, 0), Integer::sum);
        }
        List<CareerContent.Opponent> sorted = new ArrayList<>(roster);
        sorted.sort(java.util.Comparator.comparingInt((CareerContent.Opponent opponent) -> opponent.tier)
                .thenComparing(opponent -> opponent.rival != null)
                .thenComparing(opponent -> opponent.raid)
                .thenComparingInt(opponent -> -CareerContent.measuredWinRate(opponent)));
        List<CareerOpponent> views = new ArrayList<>();
        CareerLadder.Nemesis nemesis = CareerLadder.nemesis(user);
        for (CareerContent.Opponent opponent : sorted) {
            CareerOpponent view = opponentView(content, opponent, wins, winsByTier, nemesis != null && opponent.id.equals(nemesis.opponentId));
            view.tierName = content.commanderTier(opponent.tier).name;
            view.unlocked = (opponent.tier == 1 || winsByTier.getOrDefault(opponent.tier - 1, 0) >= CareerRules.WINS_TO_UNLOCK_NEXT_TIER)
                    && CareerLadder.rivalOpen(opponent, roster, wins);
            view.format = "commander";
            view.pod = opponent.podDecks != null && opponent.podDecks.size() > 1;
            view.podNames = opponent.podNames;
            views.add(view);
        }
        return views;
    }

    public boolean commanderUnlocked(String user, String opponentId) throws SQLException {
        return commanderOpponents(user).stream().anyMatch(view -> view.id.equals(opponentId) && view.unlocked);
    }

    public boolean unlocked(String user, String opponentId) throws SQLException {
        return opponents(user).stream().anyMatch(view -> view.id.equals(opponentId) && view.unlocked);
    }

    // ---- Career tables and payouts

    public void register(UUID tableId, String user, String opponentId, int tier) {
        keep(tableId, new CareerTable(user, opponentId, tier));
    }

    /** a roster duel with a setup: twists, a nemesis's edge, a personality (S037–S042) */
    public void register(UUID tableId, String user, String opponentId, int tier, CareerSetup setup) {
        keep(tableId, new CareerTable(user, opponentId, tier, "duel", opponentId, setup));
    }

    /** a Career table of a mode other than the roster duel */
    public void register(UUID tableId, String user, String kind, String ref, CareerSetup setup) {
        keep(tableId, new CareerTable(user, kind + ":" + ref, 1, kind, ref, setup));
    }

    /** Career tables are kept in the store too, so a restart in the middle of a match still pays it (B270) */
    private void keep(UUID tableId, CareerTable table) {
        tables.put(tableId, table);
        save(tableId, table);
    }

    private static void save(UUID tableId, CareerTable table) {
        try {
            CareerStore.get().putCareerTable(tableId.toString(), new Gson().toJson(table), table.created);
        } catch (SQLException | RuntimeException e) {
            logger.warn("Career: table " + tableId + " can't be kept for a restart: " + e.getMessage());
        }
    }

    private static void drop(UUID tableId) {
        try {
            CareerStore.get().deleteCareerTable(tableId.toString());
        } catch (SQLException | RuntimeException e) {
            logger.warn("Career: table " + tableId + " can't be dropped from the store: " + e.getMessage());
        }
    }

    /** a day: Career tables older than this when the server starts are gone for good */
    private static final long TABLE_KEEP_MILLIS = 24L * 60 * 60 * 1000;

    /** brings back the Career tables kept before a restart (their games resume from a save, or end and pay) */
    public int restoreTables() {
        int restored = 0;
        try {
            for (String[] row : CareerStore.get().careerTables(System.currentTimeMillis() - TABLE_KEEP_MILLIS)) {
                try {
                    CareerTable table = new Gson().fromJson(row[1], CareerTable.class);
                    if (table != null && table.user != null) {
                        tables.putIfAbsent(UUID.fromString(row[0]), table);
                        restored++;
                    }
                } catch (RuntimeException e) {
                    logger.warn("Career: kept table " + row[0] + " can't be read: " + e.getMessage());
                }
            }
        } catch (SQLException | RuntimeException e) {
            logger.warn("Career: kept tables can't be read: " + e.getMessage());
        }
        return restored;
    }

    /** what a restart does to the tables in memory (tests) */
    void dropTablesInMemory() {
        tables.clear();
    }

    /** how a Career table's game starts; null for a plain game */
    public CareerSetup setupFor(UUID tableId) {
        CareerTable table = tableId == null ? null : tables.get(tableId);
        return table == null ? null : table.setup;
    }

    /** the Career table's mode ("duel", "campaign", ...), or null when it isn't a Career table */
    public String kindOf(UUID tableId) {
        CareerTable table = tableId == null ? null : tables.get(tableId);
        return table == null ? null : table.kind;
    }

    public boolean isCareerTable(UUID tableId) {
        return tableId != null && tables.containsKey(tableId);
    }

    /** what a Career table plays for, to carry over when its game resumes at another table; null for other tables */
    public CareerTable tableInfo(UUID tableId) {
        return tableId == null ? null : tables.get(tableId);
    }

    /** a resumed Career game: the new table plays for what the saved one did, and pays the same way */
    public void adopt(UUID tableId, CareerTable table) {
        if (tableId != null && table != null) {
            keep(tableId, table);
        }
    }

    /** hints a Career game allows (S010) */
    public static final int HINTS_PER_GAME = 3;
    /** turns a Career game may take back (S011) */
    public static final int TAKE_BACKS_PER_GAME = 1;

    /**
     * The player got help in a Career game: the game is marked, and pays half when it ends.
     *
     * @param what "hint" or "take-back"
     */
    public void assisted(UUID tableId, UUID gameId, String what) {
        CareerTable table = tableInfo(tableId);
        if (table == null) {
            return;
        }
        if ("hint".equals(what)) {
            table.hintsUsed++;
            table.assisted = table.assisted == null ? "a hint" : table.assisted;
        } else {
            table.takeBacks++;
            table.assisted = "a take-back";
        }
        if (gameId != null) {
            CareerTally.taint(gameId, "the player was helped by " + table.assisted);
        }
        save(tableId, table);
    }

    public void forget(UUID tableId) {
        if (tableId != null) {
            tables.remove(tableId);
            drop(tableId);
        }
    }

    /** a Career match ended without a game tally (tests): pays the match itself */
    public CareerPayout matchEnded(UUID tableId, boolean won, int turns) {
        return matchEnded(tableId, null, won, turns, null);
    }

    /**
     * A Career match ended: pay it, once, with the game's quests, achievements and level rewards. Called by the server
     * when the match is over; the payout is keyed by the table, so a second call (or a reconnect) pays nothing.
     *
     * @param turns the turn the last game ended on
     * @param tally what the player did in the game, or null
     * @return the payout, or null when the table isn't a Career table or was paid before
     */
    public CareerPayout matchEnded(UUID tableId, mage.game.Game game, boolean won, int turns, CareerTally tally) {
        CareerTable table = tables.remove(tableId);
        if (table == null) {
            return null;
        }
        drop(tableId);
        if (mage.server.Main.isTestMode()) {
            // test mode pays nothing, Career tables included (B270)
            return null;
        }
        CareerStore store = CareerStore.get();
        try {
            long now = System.currentTimeMillis();
            CareerTally counted = tally != null && tally.taint() == null && table.assisted == null ? tally : CareerTally.forTest(null);
            if ("puzzle".equals(table.kind) || "trial".equals(table.kind)) {
                // a puzzle or a trial is a turn, not a game: it counts for itself and nothing else
                counted = CareerTally.forTest(null);
            } else {
                counted.finish(game, won, turns);
            }
            CareerPayoutEvent event = CareerPayoutEvent.match(tableId, table, won, turns, counted, now);
            CareerProgress.CareerGameResult result = payMatch(store, event);
            if (result == null && !CareerPayouts.waitsForRust(event)) {
                return null;
            }
            if (!"puzzle".equals(table.kind) && !"trial".equals(table.kind) && turns >= CareerRewards.get().turnFloor()) {
                boolean commanderCast = counted.counters().getOrDefault("commanders_cast", 0) > 0;
                boolean prestigeMode = "horde".equals(table.kind) || "gauntlet".equals(table.kind)
                        || "archenemy".equals(table.kind) || "commander".equals(table.kind);
                awardPetBond(store, table.user, won, commanderCast, prestigeMode, result, now);
            }
            CareerRules.Reward reward = matchReward(event);
            CareerPayout payout = new CareerPayout();
            payout.matchKey = tableId.toString();
            payout.at = now;
            payout.opponent = table.opponentId;
            payout.won = won;
            payout.coins = reward.coins;
            payout.xp = reward.xp;
            payout.note = reward.note;
            return payout;
        } catch (SQLException | RuntimeException e) {
            logger.error("Career payout failed for " + table.user + " on table " + tableId, e);
            return null;
        }
    }

    /** what a Career match pays itself, before the mode, quests and the rest */
    private static CareerRules.Reward matchReward(CareerPayoutEvent event) {
        CareerPayoutEvent.Career table = event.career;
        return assistedReward(modeReward(table.kind, table.tier, event.won, event.turns), table.assisted);
    }

    /** Awards Bond XP to the player's equipped pet (if any) and attaches the summary to the saved game result. */
    private void awardPetBond(CareerStore store, String user, boolean won, boolean commanderCast,
                              boolean prestigeMode, CareerProgress.CareerGameResult result, long now) {
        try {
            mage.server.support.PlayerLook look = mage.server.support.Support.get().store().look(user);
            if (look == null || look.pet == null || look.pet.isEmpty()) {
                return;
            }
            CareerPets.CareerPetBondResult bond = CareerPets.get()
                    .recordGame(store, user, look.pet, won, commanderCast, prestigeMode, now);
            if (bond != null && result != null && result.gameKey != null) {
                result.petBond = bond;
                store.setGameDetails(result.gameKey, new Gson().toJson(result));
            }
        } catch (SQLException | RuntimeException e) {
            logger.warn("Pet bond XP update skipped for " + user + ": " + e.getMessage());
        }
    }

    /**
     * Pays a Career match from its event, once, as the payouts route says (CareerPayouts): Java pays it here, or the
     * event waits for the gateway.
     *
     * @return what the match paid; null when it was paid before or waits for Rust
     */
    CareerProgress.CareerGameResult payMatch(CareerStore store, CareerPayoutEvent event) throws SQLException {
        return CareerPayouts.pay(store, event, paying -> payMatchIn(store, paying));
    }

    private CareerProgress.CareerGameResult payMatchIn(CareerStore store, CareerPayoutEvent event) throws SQLException {
        CareerPayoutEvent.Career table = event.career;
        boolean won = event.won;
        int turns = event.turns;
        long now = event.at;
        CareerRules.Reward reward = matchReward(event);
        CareerTally played = CareerPayoutEvent.tally(event.counters, event.moments);
        CareerProfile before = store.profile(table.user);
        if (before == null || !store.payoutIn(event.key, table.user, table.opponent, won,
                reward.coins, reward.xp, reward.note, now)) {
            return null;
        }
        store.claimGame(event.key, table.user, now);
        CareerProgress.CareerGameResult paid = new CareerProgress.CareerGameResult();
        paid.gameKey = event.key;
        paid.kind = CareerRewards.CAREER;
        paid.career = true;
        paid.won = won;
        paid.opponent = table.opponent;
        paid.coins = reward.coins;
        paid.xp = reward.xp;
        paid.note = reward.note;
        paid.assisted = table.assisted;
        paid.xpBefore = before.xp;
        Set<Integer> tiersBefore = openTiers(table.user);
        finishMode(store, table, won, turns, played, paid, now);
        if (!"puzzle".equals(table.kind) && !"trial".equals(table.kind) && turns >= CareerRewards.get().turnFloor()) {
            dailyBonuses(store, table.user, CareerRewards.CAREER, won, paid);
        }
        CareerProgress.get().apply(store, table.user, played, paid, before.xp, now);
        for (CareerOpponent opponent : opponents(table.user)) {
            if (opponent.unlocked && !tiersBefore.contains(opponent.tier) && !paid.opened.contains(opponent.tierName)) {
                paid.opened.add(opponent.tierName);
            }
        }
        store.setGameDetails(paid.gameKey, new Gson().toJson(paid));
        store.setGameRecord(paid.gameKey, CareerRewards.CAREER, won, paid.coins, paid.xp,
                paid.note != null ? paid.note : table.kind + " " + table.opponent);
        return paid;
    }

    /** an assisted game pays half, and says why */
    static CareerRules.Reward assistedReward(CareerRules.Reward reward, String assisted) {
        if (assisted == null) {
            return reward;
        }
        String note = "You had help (" + assisted + "), so this game pays half and counts for no quests.";
        return new CareerRules.Reward(reward.coins / 2, reward.xp / 2, reward.note == null ? note : reward.note + " " + note);
    }

    /** what the match itself pays: a roster duel by tier; a mode pays through its own rewards */
    private static CareerRules.Reward modeReward(String kind, int tier, boolean won, int turns) {
        switch (kind) {
            case "duel":
                return CareerRules.reward(tier, won, turns);
            case "campaign":
            case "gauntlet":
            case "limited":
                // a win pays through the mode; a real loss still pays a little, like a roster loss
                return won ? new CareerRules.Reward(0, 50, null) : CareerRules.reward(1, false, turns);
            default:
                return new CareerRules.Reward(0, 0, null);
        }
    }

    private void finishMode(CareerStore store, CareerPayoutEvent.Career table, boolean won, int turns, CareerTally tally,
                            CareerProgress.CareerGameResult result, long now) throws SQLException {
        switch (table.kind) {
            case "campaign":
            case "trial":
                CareerCampaigns.get().finished(store, table.user, table.ref, won, result, now);
                break;
            case "gauntlet":
                store.addStat(table.user, "games_mode_gauntlet", 1);
                CareerGauntlet.get().finished(store, table.user, table.ref, won, result, now);
                break;
            case "puzzle":
                CareerPuzzles.get().finished(store, table.user, table.ref, won, result, now, table.created);
                break;
            case "challenge":
                CareerChallenges.get().finished(store, table.user, table.ref, won, tally, result, now);
                break;
            case "limited":
                CareerLimited.get().finished(store, table.user, table.ref, won, result, now);
                break;
            case "horde":
                store.addStat(table.user, "games_mode_horde", 1);
                CareerHordes.get().finished(store, table.user, table.ref, won, tally, result, now);
                break;
            case "archenemy":
                store.addStat(table.user, "games_mode_archenemy", 1);
                CareerArchenemy.get().finished(store, table.user, table.ref, won, result, now);
                break;
            case "thg":
                store.addStat(table.user, "games_mode_thg", 1);
                CareerTwoHeadedGiant.get().finished(store, table.user, table.ref, won, result);
                break;
            case "duel":
                CareerLadder.finished(store, table.user, table.opponent, won, result, now);
                break;
            default:
                break;
        }
    }

    private Set<Integer> openTiers(String user) throws SQLException {
        Set<Integer> tiers = new java.util.HashSet<>();
        for (CareerOpponent opponent : opponents(user)) {
            if (opponent.unlocked) {
                tiers.add(opponent.tier);
            }
        }
        return tiers;
    }

    /** One human seat of a finished game, for progression. */
    public static class Seat {
        public final String user;
        public final boolean won;
        /** what the player did, or null when nothing was tallied */
        public final CareerTally tally;
        /** the deck's colors (W, U, B, R, G, multicolor or colorless), for masteries and quests */
        public java.util.Set<String> colors = new java.util.LinkedHashSet<>();
        /** won the whole match (the last game of a best of three) */
        public boolean matchWon;

        public Seat(String user, boolean won, CareerTally tally) {
            this.user = user;
            this.won = won;
            this.tally = tally;
        }
    }

    /** A finished game outside Career's own tables. */
    public static class FinishedGame {
        public final UUID gameId;
        /** {@link CareerRewards#AI}, PERSON, POD or EVENT */
        public final String kind;
        public final int players;
        public final int turns;
        public final List<Seat> seats = new ArrayList<>();
        /** the strongest AI's skill at an AI table, or 0 */
        public int aiSkill;
        /** the game, for how it ended; null in tests */
        public mage.game.Game game;
        /** the match was a best of three or longer */
        public boolean bo3;
        /** {@link #formatOf} the table's format, or null */
        public String format;

        public FinishedGame(UUID gameId, String kind, int players, int turns) {
            this.gameId = gameId;
            this.kind = kind;
            this.players = players;
            this.turns = turns;
        }
    }

    /** the kind of a game outside Career tables, from its table */
    public static String kindOf(boolean event, int players, int humans) {
        if (event) {
            return CareerRewards.EVENT;
        }
        if (players >= 3) {
            return CareerRewards.POD;
        }
        return humans >= 2 ? CareerRewards.PERSON : CareerRewards.AI;
    }

    /** the formats masteries and quests know, from a table's deck type (B290) */
    public static final List<String> FORMATS = java.util.Arrays.asList(
            "standard", "pioneer", "modern", "legacy", "vintage", "pauper", "commander", "brawl", "limited");

    /** a table's format, one of {@link #FORMATS}, or "other" */
    public static String formatOf(String deckType, String gameType, boolean limited) {
        if (limited) {
            return "limited";
        }
        String text = ((deckType == null ? "" : deckType) + " " + (gameType == null ? "" : gameType)).toLowerCase(java.util.Locale.ROOT);
        if (text.contains("brawl")) {
            return "brawl";
        }
        if (text.contains("commander") || text.contains("oathbreaker")) {
            return "commander";
        }
        for (String format : FORMATS) {
            if (text.contains(format)) {
                return format;
            }
        }
        return text.contains("limited") || text.contains("draft") || text.contains("sealed") ? "limited" : "other";
    }

    /** a deck's colors as masteries count them: each of W, U, B, R, G it uses, plus multicolor, or colorless */
    public static java.util.Set<String> colorsOf(mage.cards.decks.Deck deck) {
        java.util.Set<String> colors = new java.util.LinkedHashSet<>();
        if (deck == null) {
            return colors;
        }
        mage.ObjectColor all = new mage.ObjectColor();
        for (mage.cards.Card card : deck.getCards()) {
            all.addColor(card.getColor());
        }
        if (all.isWhite()) {
            colors.add("W");
        }
        if (all.isBlue()) {
            colors.add("U");
        }
        if (all.isBlack()) {
            colors.add("B");
        }
        if (all.isRed()) {
            colors.add("R");
        }
        if (all.isGreen()) {
            colors.add("G");
        }
        if (colors.size() >= 2) {
            colors.add("multicolor");
        } else if (colors.isEmpty()) {
            colors.add("colorless");
        }
        return colors;
    }

    /** the key a seat's game is kept under */
    static String gameKey(UUID gameId, String user) {
        return gameId + "/" + CareerStore.key(user);
    }

    /**
     * A game outside Career's own tables ended: every human seat with a clean tally gets progression from it, once
     * (B265). A game that ended before the turn floor pays nobody; people who already played each other the day's
     * capped number of games get nothing more from each other (B268). Only XP: coins come from the daily wins ladder.
     *
     * @return what each seat got, by account name; seats that got nothing are left out
     */
    public Map<String, CareerProgress.CareerGameResult> gameEnded(FinishedGame finished) {
        Map<String, CareerProgress.CareerGameResult> results = new LinkedHashMap<>();
        if (mage.server.Main.isTestMode()) {
            return results;
        }
        CareerStore store = CareerStore.get();
        try {
            CareerPayoutEvent event = CareerPayoutEvent.game(finished, System.currentTimeMillis());
            Map<String, CareerProgress.CareerGameResult> paid = payGame(store, event);
            if (paid != null) {
                results.putAll(paid);
            } else if (CareerPayouts.waitsForRust(event)) {
                // Rust pays in a moment: the players hear the keys their rewards will be under
                for (CareerPayoutEvent.Seat seat : event.finished.seats) {
                    if (seat.counted) {
                        CareerProgress.CareerGameResult waiting = new CareerProgress.CareerGameResult();
                        waiting.gameKey = gameKey(finished.gameId, seat.user);
                        waiting.kind = finished.kind;
                        waiting.won = seat.won;
                        results.put(seat.user, waiting);
                    }
                }
            }
            if (finished.turns >= CareerRewards.get().turnFloor()) {
                long bondNow = System.currentTimeMillis();
                for (CareerPayoutEvent.Seat seat : event.finished.seats) {
                    if (!seat.counted) {
                        continue;
                    }
                    boolean commanderCast = seat.counters != null && seat.counters.getOrDefault("commanders_cast", 0) > 0;
                    boolean prestigeMode = "event".equals(finished.kind) || "pod".equals(finished.kind);
                    awardPetBond(store, seat.user, seat.won, commanderCast, prestigeMode, paid == null ? null : paid.get(seat.user), bondNow);
                }
            }
        } catch (SQLException | RuntimeException e) {
            logger.error("Career progress failed for game " + finished.gameId, e);
        }
        return results;
    }

    /**
     * Pays a game outside Career tables from its event, as the payouts route says (CareerPayouts).
     *
     * @return what each seat got, by account name; null when the event waits for Rust
     */
    Map<String, CareerProgress.CareerGameResult> payGame(CareerStore store, CareerPayoutEvent event) throws SQLException {
        return CareerPayouts.pay(store, event, paying -> payGameIn(store, paying));
    }

    private Map<String, CareerProgress.CareerGameResult> payGameIn(CareerStore store, CareerPayoutEvent event) throws SQLException {
        Map<String, CareerProgress.CareerGameResult> results = new LinkedHashMap<>();
        CareerPayoutEvent.Finished finished = event.finished;
        CareerRewards rewards = CareerRewards.get();
        long now = event.at;
        String day = CareerProgress.get().day();
        List<CareerPayoutEvent.Seat> humans = finished.seats;
        boolean capped = CareerRewards.PERSON.equals(finished.kind) || CareerRewards.POD.equals(finished.kind);
        for (CareerPayoutEvent.Seat seat : humans) {
            if (!seat.counted) {
                continue;
            }
            String key = event.key + "/" + CareerStore.key(seat.user);
            ensureProfile(store, seat.user, now);
            if (!store.claimGame(key, seat.user, now)) {
                continue;
            }
            CareerProgress.CareerGameResult result = new CareerProgress.CareerGameResult();
            result.gameKey = key;
            result.kind = finished.kind;
            result.won = seat.won;
            if (CareerRewards.PERSON.equals(finished.kind) && humans.size() == 2) {
                // a duel between two people: the results name the other one
                result.opponent = humans.get(humans.get(0) == seat ? 1 : 0).user;
            }
            CareerProfile before = store.profile(seat.user);
            result.xpBefore = before.xp;
            String reason = null;
            if (finished.turns < rewards.turnFloor()) {
                reason = "The game ended before turn " + rewards.turnFloor() + ", so it paid nothing.";
            } else if (capped) {
                for (CareerPayoutEvent.Seat other : humans) {
                    if (other != seat && store.pairGames(day, pair(seat.user, other.user)) >= rewards.pairCapPerDay()) {
                        reason = "You've played " + other.user + " " + rewards.pairCapPerDay()
                                + " times today; more games together pay nothing until tomorrow.";
                        break;
                    }
                }
            }
            if (reason != null) {
                result.note = reason;
                CareerProgress.get().levelsIn(store, seat.user, before.xp, result, now);
                store.setGameDetails(key, new Gson().toJson(result));
                store.setGameRecord(key, finished.kind, seat.won, 0, 0, reason);
                results.put(seat.user, result);
                continue;
            }
            int xp = rewards.gameXp(finished.kind, seat.won, finished.players, finished.aiSkill);
            if (finished.bo3 && seat.matchWon && rewards.bo3MatchWinXp() > 0) {
                xp += rewards.bo3MatchWinXp();
                result.bonuses.add(new CareerProgress.CareerBonus("You won the match", rewards.bo3MatchWinXp(), 0));
            }
            store.addProfileInt(seat.user, "xp", xp);
            result.xp = xp;
            dailyBonuses(store, seat.user, finished.kind, seat.won, result);
            CareerProgress.get().apply(store, seat.user, CareerPayoutEvent.tally(seat.counters, seat.moments), result, before.xp, now);
            store.setGameDetails(key, new Gson().toJson(result));
            store.setGameRecord(key, finished.kind, seat.won, result.coins, result.xp,
                    finished.kind + (seat.won ? " win" : " loss") + " on turn " + finished.turns);
            results.put(seat.user, result);
        }
        if (capped && finished.turns >= rewards.turnFloor()) {
            for (int i = 0; i < humans.size(); i++) {
                for (int j = i + 1; j < humans.size(); j++) {
                    store.addPairGame(day, pair(humans.get(i).user, humans.get(j).user));
                }
            }
        }
        return results;
    }

    /**
     * An event ended (B289): each person who finished it gets Career packs by their match wins (only when at least one
     * other person played) and XP, once per event.
     *
     * @return the packs each account got, by name
     */
    public Map<String, Integer> eventEnded(UUID tournamentId, boolean limited, Map<String, Integer> matchWins) {
        Map<String, Integer> paid = new LinkedHashMap<>();
        if (mage.server.Main.isTestMode() || matchWins.isEmpty()) {
            return paid;
        }
        CareerRewards rewards = CareerRewards.get();
        CareerStore store = CareerStore.get();
        long now = System.currentTimeMillis();
        boolean withPeople = matchWins.size() >= 2;
        try {
            store.transaction(() -> {
                for (Map.Entry<String, Integer> entry : matchWins.entrySet()) {
                    String user = entry.getKey();
                    int wins = entry.getValue();
                    ensureProfile(store, user, now);
                    String key = "event/" + tournamentId + "/" + CareerStore.key(user);
                    if (!store.claimGame(key, user, now)) {
                        continue;
                    }
                    int packs = rewards.eventPacks(limited, wins, withPeople);
                    int xp = rewards.eventXp(wins);
                    CareerProfile before = store.profile(user);
                    store.addProfileInt(user, "xp", xp);
                    if (packs > 0) {
                        store.addProfileInt(user, "pack_tokens", packs);
                    }
                    store.addStat(user, "events_finished", 1);
                    if (limited) {
                        store.addStat(user, "limited_events_finished", 1);
                    }
                    store.maxStat(user, "event_best_wins", wins);
                    CareerProgress.CareerGameResult result = new CareerProgress.CareerGameResult();
                    result.gameKey = key;
                    result.kind = CareerRewards.EVENT;
                    result.xp = xp;
                    result.packs = packs;
                    result.xpBefore = before.xp;
                    result.bonuses.add(new CareerProgress.CareerBonus((limited ? "Limited event" : "Event") + " finished, "
                            + wins + (wins == 1 ? " match win" : " match wins"), xp, 0));
                    CareerProgress.get().achievementsIn(store, user, null, result, now);
                    CareerProgress.get().levelsIn(store, user, before.xp, result, now);
                    store.setGameDetails(key, new Gson().toJson(result));
                    store.setGameRecord(key, CareerRewards.EVENT, wins > 0, 0, result.xp,
                            "event finished with " + wins + " match wins, " + packs + " packs");
                    paid.put(user, packs);
                }
                return null;
            });
        } catch (SQLException | RuntimeException e) {
            logger.error("Career event prizes failed for " + tournamentId, e);
        }
        return paid;
    }

    /** two accounts as one key, the same either way round */
    static String pair(String a, String b) {
        String x = CareerStore.key(a);
        String y = CareerStore.key(b);
        return x.compareTo(y) <= 0 ? x + "|" + y : y + "|" + x;
    }

    /**
     * The day's extras, for any kind of game: the daily wins ladder's coins (B285), the first win of the day and the
     * first game against a person (B286). Inside the caller's transaction; the game is counted for the day here.
     */
    void dailyBonuses(CareerStore store, String user, String kind, boolean won, CareerProgress.CareerGameResult result) throws SQLException {
        CareerRewards rewards = CareerRewards.get();
        String day = CareerProgress.get().day();
        int[] today = store.daily(user, day);
        boolean person = CareerRewards.PERSON.equals(kind) || CareerRewards.POD.equals(kind) || CareerRewards.EVENT.equals(kind);
        if (won) {
            int nth = today[0] + 1;
            int coins = rewards.dailyWinCoins(nth);
            if (coins > 0) {
                store.addCoins(user, coins);
                result.coins += coins;
                result.bonuses.add(new CareerProgress.CareerBonus("Daily win " + nth + " of " + rewards.dailyWinLadder().size(), 0, coins));
            }
            if (today[0] == 0 && rewards.firstWinXp() > 0) {
                store.addProfileInt(user, "xp", rewards.firstWinXp());
                result.xp += rewards.firstWinXp();
                CareerProgress.CareerBonus bonus = new CareerProgress.CareerBonus("First win of the day", rewards.firstWinXp(), 0);
                bonus.tokens = rewards.capsules().firstWinTokens;
                if (bonus.tokens > 0) {
                    CareerCapsules.addTokens(store, user, bonus.tokens, "first win of the day", System.currentTimeMillis());
                    result.tokens += bonus.tokens;
                }
                result.bonuses.add(bonus);
            }
        }
        if (person && today[2] == 0 && rewards.firstPersonGameXp() > 0) {
            store.addProfileInt(user, "xp", rewards.firstPersonGameXp());
            result.xp += rewards.firstPersonGameXp();
            result.bonuses.add(new CareerProgress.CareerBonus("First game against a person today", rewards.firstPersonGameXp(), 0));
        }
        store.addDaily(user, day, won, person);
    }

    /** a regular game against the AI with one player (tests and older callers) */
    public CareerProgress.CareerGameResult aiGameEnded(UUID gameId, String user, mage.game.Game game, boolean won, int turns, CareerTally tally) {
        FinishedGame finished = new FinishedGame(gameId, CareerRewards.AI, 2, turns);
        finished.game = game;
        finished.seats.add(new Seat(user, won, tally));
        return gameEnded(finished).get(user);
    }

    /** One account's progression for /admin (B272). */
    public static class CareerAdminView {
        public int level;
        public int xp;
        public int coins;
        public boolean collection;
        public List<CareerStore.PaidGame> games;
        /** pairs with this account that hit the day's cap, and on how many of the last 14 days */
        public Map<String, Integer> cappedPairs;
    }

    public CareerAdminView adminView(String user) throws SQLException {
        CareerStore store = CareerStore.get();
        CareerProfile profile = store.profile(user);
        if (profile == null) {
            return null;
        }
        CareerAdminView view = new CareerAdminView();
        view.level = profile.level;
        view.xp = profile.xp;
        view.coins = profile.coins;
        view.collection = profile.collection;
        view.games = store.paidGames(user, 50);
        view.cappedPairs = store.cappedPairs(user, java.time.LocalDate.now(java.time.ZoneOffset.UTC).minusDays(14).toString(),
                CareerRewards.get().pairCapPerDay());
        return view;
    }

    /**
     * Takes back what a counted game paid (B272): its XP and coins (never below zero coins). Quests and achievements it
     * finished stay. False when it was revoked before or isn't the account's.
     */
    public boolean revoke(String user, String gameKey) throws SQLException {
        CareerStore store = CareerStore.get();
        return store.transaction(() -> {
            CareerStore.PaidGame game = store.paidGame(user, gameKey);
            if (game == null || !store.revokeGame(gameKey)) {
                return false;
            }
            CareerProfile profile = store.profile(user);
            store.addProfileInt(user, "xp", -Math.min(game.xp, profile.xp));
            store.addCoins(user, -Math.min(game.coins, profile.coins));
            return true;
        });
    }

    /** whether the player wants progression shown (B312): kept with the account, so it holds on every device */
    public CareerProfile setShowProgress(String user, boolean show) throws SQLException {
        CareerStore store = CareerStore.get();
        return store.transaction(() -> {
            ensureProfile(store, user, System.currentTimeMillis());
            store.setProfileInt(user, "show_progress", show ? 1 : 0);
            return store.profile(user);
        });
    }

    /** Saved Career decks one account can keep */
    public static final int MAX_SAVED_DECKS = 40;
    private static final int MAX_DECK_NAME = 60;

    /** One saved Career deck, ready for the deck switcher and editor. */
    public static class CareerSavedDeck {
        public String id;
        public String name;
        public String format;
        public DeckCardLists deck;
        public boolean active;
        /** where the deck came from ("starter:<id>", "campaign:<campaign>/<chapter>"), or null */
        public String source;
        public long updated;
        /** cards the deck has more copies of than the collection owns (empty when playable) */
        public List<String> missing;
    }

    /**
     * Saves the account's active Career deck (B311), checked against the collection: every card the deck has more
     * copies of than the account owns is a problem, and nothing is saved. Also keeps the active saved Career deck row
     * in sync when multi-deck storage is initialized.
     */
    public void saveDeck(String user, DeckCardLists deck) throws SQLException, CareerException {
        CareerStore store = CareerStore.get();
        CareerProfile profile = store.profile(user);
        if (profile == null || !profile.collection) {
            throw new CareerException("Open your Career collection first.");
        }
        List<String> problems = ownershipProblems(user, deck);
        if (!problems.isEmpty()) {
            throw new CareerException("The deck has cards you don't own: " + String.join("; ", problems) + ".");
        }
        long now = System.currentTimeMillis();
        Gson gson = new Gson();
        String json = gson.toJson(deck);
        store.transaction(() -> {
            store.putDeck(user, json, now);
            CareerStore.CareerDeckRow active = store.activeCareerDeck(user);
            if (active != null) {
                active.name = cleanDeckName(deck.getName(), active.name);
                active.deck = json;
                active.updated = now;
                store.putCareerDeck(user, active);
            }
            return null;
        });
    }

    /** the account's saved Career deck, or null */
    public DeckCardLists deck(String user) throws SQLException {
        String json = CareerStore.get().deck(user);
        return json == null ? null : new Gson().fromJson(json, DeckCardLists.class);
    }

    /**
     * Every saved Career deck the account keeps, active first then newest first. On first access for an existing
     * account, migrates the legacy single Career deck (or starter) and any chapter decks already earned from campaigns.
     */
    public List<CareerSavedDeck> savedDecks(String user) throws SQLException, CareerException {
        CareerStore store = CareerStore.get();
        CareerProfile profile = store.profile(user);
        if (profile == null || !profile.collection) {
            return Collections.emptyList();
        }
        long now = System.currentTimeMillis();
        List<CareerStore.CareerDeckRow> rows = store.transaction(() -> {
            ensureSavedDecksInitialized(store, user, profile, now);
            syncActiveFromLegacyIfNewer(store, user);
            return store.careerDecks(user);
        });
        Map<String, Integer> owned = ownedCounts(store, user);
        Gson gson = new Gson();
        List<CareerSavedDeck> out = new ArrayList<>(rows.size());
        for (CareerStore.CareerDeckRow row : rows) {
            out.add(toSavedDeckView(row, owned, gson));
        }
        return out;
    }

    /**
     * Creates (id null/blank) or updates a saved Career deck. Checked against the collection like {@link #saveDeck}.
     * When {@code activate} is true (or this is already the active deck), updates the active slot and mirrors to {@code decks}.
     */
    public CareerSavedDeck saveNamedDeck(String user, String id, String name, DeckCardLists deck, boolean activate)
            throws SQLException, CareerException {
        return saveNamedDeck(user, id, name, deck, activate, null);
    }

    /**
     * Creates (id null/blank) or updates a saved Career deck with an optional deck format ("Constructed - Freeform" or
     * "Variant Magic - Commander"). Checked against the collection like {@link #saveDeck}.
     */
    public CareerSavedDeck saveNamedDeck(String user, String id, String name, DeckCardLists deck, boolean activate, String format)
            throws SQLException, CareerException {
        CareerStore store = CareerStore.get();
        CareerProfile profile = store.profile(user);
        if (profile == null || !profile.collection) {
            throw new CareerException("Open your Career collection first.");
        }
        if (deck == null) {
            throw new CareerException("Send a deck to save.");
        }
        List<String> problems = ownershipProblems(user, deck);
        if (!problems.isEmpty()) {
            throw new CareerException("The deck has cards you don't own: " + String.join("; ", problems) + ".");
        }
        long now = System.currentTimeMillis();
        String trimmedId = id == null ? "" : id.trim();
        String requestedFormat = normalizeDeckFormat(format);
        Gson gson = new Gson();
        CareerStore.CareerDeckRow[] saved = new CareerStore.CareerDeckRow[1];
        refusing(() -> store.transaction(() -> {
            ensureSavedDecksInitialized(store, user, profile, now);
            CareerStore.CareerDeckRow existing = trimmedId.isEmpty() ? null : store.careerDeck(user, trimmedId);
            if (!trimmedId.isEmpty() && existing == null) {
                throw new SQLException(new CareerException("That Career deck doesn't exist."));
            }
            if (existing == null && store.careerDeckCount(user) >= MAX_SAVED_DECKS) {
                throw new SQLException(new CareerException("You already have " + MAX_SAVED_DECKS + " Career decks. Delete one first."));
            }
            String cleanName = cleanDeckName(name, existing != null ? existing.name : deck.getName());
            deck.setName(cleanName);
            String json = gson.toJson(deck);

            CareerStore.CareerDeckRow row = existing != null ? existing : new CareerStore.CareerDeckRow();
            if (existing == null) {
                row.id = UUID.randomUUID().toString();
            }
            row.name = cleanName;
            row.deck = json;
            if (requestedFormat != null) {
                row.format = requestedFormat;
            } else if (row.format == null || row.format.isEmpty()) {
                row.format = CareerRules.FORMAT_CONSTRUCTED;
            }
            row.updated = now;
            boolean makeActive = activate || (existing != null && existing.active) || store.activeCareerDeck(user) == null;
            row.active = makeActive;
            store.putCareerDeck(user, row);
            if (makeActive) {
                store.setActiveCareerDeck(user, row.id);
                store.putDeck(user, json, now);
            }
            saved[0] = store.careerDeck(user, row.id);
            return null;
        }));
        Map<String, Integer> owned = ownedCounts(store, user);
        return toSavedDeckView(saved[0], owned, gson);
    }

    private static String normalizeDeckFormat(String format) {
        if (format == null || format.trim().isEmpty()) {
            return null;
        }
        String trimmed = format.trim();
        if (CareerRules.FORMAT_COMMANDER.equalsIgnoreCase(trimmed) || "commander".equalsIgnoreCase(trimmed)) {
            return CareerRules.FORMAT_COMMANDER;
        }
        return CareerRules.FORMAT_CONSTRUCTED;
    }

    /** Makes one of the account's saved Career decks the active deck for Career duels. */
    public CareerSavedDeck selectDeck(String user, String id) throws SQLException, CareerException {
        CareerStore store = CareerStore.get();
        CareerProfile profile = store.profile(user);
        if (profile == null || !profile.collection) {
            throw new CareerException("Open your Career collection first.");
        }
        String trimmedId = id == null ? "" : id.trim();
        long now = System.currentTimeMillis();
        CareerStore.CareerDeckRow[] chosen = new CareerStore.CareerDeckRow[1];
        refusing(() -> store.transaction(() -> {
            ensureSavedDecksInitialized(store, user, profile, now);
            CareerStore.CareerDeckRow row = store.careerDeck(user, trimmedId);
            if (row == null) {
                throw new SQLException(new CareerException("That Career deck doesn't exist."));
            }
            store.setActiveCareerDeck(user, row.id);
            store.putDeck(user, row.deck, now);
            chosen[0] = store.careerDeck(user, row.id);
            return null;
        }));
        Map<String, Integer> owned = ownedCounts(store, user);
        return toSavedDeckView(chosen[0], owned, new Gson());
    }

    /** Deletes a saved Career deck (keeping the cards in the collection). Refuses to delete the last deck. */
    public boolean deleteSavedDeck(String user, String id) throws SQLException, CareerException {
        CareerStore store = CareerStore.get();
        CareerProfile profile = store.profile(user);
        if (profile == null || !profile.collection) {
            throw new CareerException("Open your Career collection first.");
        }
        String trimmedId = id == null ? "" : id.trim();
        long now = System.currentTimeMillis();
        boolean[] deleted = new boolean[1];
        refusing(() -> store.transaction(() -> {
            ensureSavedDecksInitialized(store, user, profile, now);
            CareerStore.CareerDeckRow row = store.careerDeck(user, trimmedId);
            if (row == null) {
                return null;
            }
            if (store.careerDeckCount(user) <= 1) {
                throw new SQLException(new CareerException("Keep at least one Career deck."));
            }
            deleted[0] = store.deleteCareerDeck(user, row.id);
            if (deleted[0] && row.active) {
                List<CareerStore.CareerDeckRow> remaining = store.careerDecks(user);
                if (!remaining.isEmpty()) {
                    CareerStore.CareerDeckRow next = remaining.get(0);
                    store.setActiveCareerDeck(user, next.id);
                    store.putDeck(user, next.deck, now);
                }
            }
            return null;
        }));
        return deleted[0];
    }

    /**
     * Records an earned campaign chapter deck in the account's saved Career decks (idempotent per source, keeping the
     * player's currently active deck selected). Called inside a campaign payout transaction.
     */
    void saveEarnedDeck(CareerStore store, String user, String source, String name, DeckCardLists deck, long now) throws SQLException {
        CareerProfile profile = store.profile(user);
        if (profile == null || !profile.collection) {
            return;
        }
        ensureSavedDecksInitialized(store, user, profile, now);
        if (store.careerDeckBySource(user, source) != null) {
            return;
        }
        if (store.careerDeckCount(user) >= MAX_SAVED_DECKS) {
            return;
        }
        String cleanName = cleanDeckName(name, deck.getName());
        DeckCardLists copy = copyDeckWithName(deck, cleanName);
        CareerStore.CareerDeckRow row = new CareerStore.CareerDeckRow();
        row.id = UUID.randomUUID().toString();
        row.name = cleanName;
        row.deck = new Gson().toJson(copy);
        row.source = source;
        row.active = store.activeCareerDeck(user) == null;
        row.updated = now;
        store.putCareerDeck(user, row);
        if (row.active) {
            store.putDeck(user, row.deck, now);
        }
    }

    private void ensureSavedDecksInitialized(CareerStore store, String user, CareerProfile profile, long now) throws SQLException {
        if (store.careerDeckCount(user) > 0) {
            return;
        }
        Gson gson = new Gson();
        String legacyJson = store.deck(user);
        long legacyUpdated = store.deckUpdated(user);
        if (legacyJson != null && !legacyJson.isEmpty()) {
            DeckCardLists legacy = gson.fromJson(legacyJson, DeckCardLists.class);
            String starterName = starterNameOf(profile.starter);
            String name = cleanDeckName(legacy != null ? legacy.getName() : null, starterName);
            CareerStore.CareerDeckRow row = new CareerStore.CareerDeckRow();
            row.id = UUID.randomUUID().toString();
            row.name = name;
            row.deck = legacyJson;
            row.source = profile.starter != null && !profile.starter.isEmpty() ? "starter:" + profile.starter : null;
            row.format = CareerContent.get().isCommanderStarter(profile.starter)
                    ? CareerRules.FORMAT_COMMANDER : CareerRules.FORMAT_CONSTRUCTED;
            row.active = true;
            row.updated = legacyUpdated > 0 ? legacyUpdated : now;
            store.putCareerDeck(user, row);
        } else if (profile.starter != null && !profile.starter.isEmpty()) {
            try {
                DeckCardLists starterDeck = CareerContent.get().starterDeck(profile.starter);
                String name = cleanDeckName(starterNameOf(profile.starter), starterDeck.getName());
                DeckCardLists copy = copyDeckWithName(starterDeck, name);
                String json = gson.toJson(copy);
                CareerStore.CareerDeckRow row = new CareerStore.CareerDeckRow();
                row.id = UUID.randomUUID().toString();
                row.name = name;
                row.deck = json;
                row.source = "starter:" + profile.starter;
                row.format = CareerContent.get().isCommanderStarter(profile.starter)
                        ? CareerRules.FORMAT_COMMANDER : CareerRules.FORMAT_CONSTRUCTED;
                row.active = true;
                row.updated = now;
                store.putCareerDeck(user, row);
                store.putDeck(user, json, now);
            } catch (java.io.IOException e) {
                logger.warn("Starter deck can't be read during Career deck migration: " + profile.starter, e);
            }
        }
        // Backfill any chapter decks already earned from completed campaign bosses on this account
        long order = now - 1;
        for (CareerCampaigns.EarnedDeck earned : CareerCampaigns.get().earnedDecks(store, user)) {
            if (store.careerDeckBySource(user, earned.source) != null || store.careerDeckCount(user) >= MAX_SAVED_DECKS) {
                continue;
            }
            String cleanName = cleanDeckName(earned.name, earned.deck.getName());
            DeckCardLists copy = copyDeckWithName(earned.deck, cleanName);
            CareerStore.CareerDeckRow row = new CareerStore.CareerDeckRow();
            row.id = UUID.randomUUID().toString();
            row.name = cleanName;
            row.deck = gson.toJson(copy);
            row.source = earned.source;
            row.active = store.activeCareerDeck(user) == null;
            row.updated = order--;
            store.putCareerDeck(user, row);
            if (row.active) {
                store.putDeck(user, row.deck, now);
            }
        }
    }

    private void syncActiveFromLegacyIfNewer(CareerStore store, String user) throws SQLException {
        long legacyUpdated = store.deckUpdated(user);
        if (legacyUpdated <= 0) {
            return;
        }
        CareerStore.CareerDeckRow active = store.activeCareerDeck(user);
        if (active == null || legacyUpdated <= active.updated) {
            return;
        }
        String legacyJson = store.deck(user);
        if (legacyJson == null || legacyJson.isEmpty()) {
            return;
        }
        DeckCardLists parsed = new Gson().fromJson(legacyJson, DeckCardLists.class);
        active.name = cleanDeckName(parsed != null ? parsed.getName() : null, active.name);
        active.deck = legacyJson;
        active.updated = legacyUpdated;
        store.putCareerDeck(user, active);
    }

    private static String starterNameOf(String starterId) {
        if (starterId == null || starterId.isEmpty()) {
            return "Career deck";
        }
        CareerContent.CareerStarter starter = CareerContent.get().starter(starterId);
        return starter != null && starter.name != null && !starter.name.isEmpty() ? starter.name : "Career deck";
    }

    private static String cleanDeckName(String primary, String fallback) {
        String candidate = primary != null ? primary.trim() : "";
        if (candidate.isEmpty()) {
            candidate = fallback != null ? fallback.trim() : "";
        }
        if (candidate.isEmpty()) {
            candidate = "Career deck";
        }
        return candidate.length() > MAX_DECK_NAME ? candidate.substring(0, MAX_DECK_NAME).trim() : candidate;
    }

    private static DeckCardLists copyDeckWithName(DeckCardLists deck, String name) {
        DeckCardLists copy = new DeckCardLists();
        copy.setName(name);
        if (deck.getCards() != null) {
            copy.setCards(new ArrayList<>(deck.getCards()));
        }
        if (deck.getSideboard() != null) {
            copy.setSideboard(new ArrayList<>(deck.getSideboard()));
        }
        return copy;
    }

    private Map<String, Integer> ownedCounts(CareerStore store, String user) throws SQLException {
        Map<String, Integer> owned = new java.util.HashMap<>();
        for (CareerCard card : store.collection(user)) {
            if (card.name != null) {
                owned.merge(card.name, card.count, Integer::sum);
            }
        }
        return owned;
    }

    private CareerSavedDeck toSavedDeckView(CareerStore.CareerDeckRow row, Map<String, Integer> owned, Gson gson) {
        CareerSavedDeck view = new CareerSavedDeck();
        view.id = row.id;
        view.name = row.name;
        view.format = row.format == null || row.format.isEmpty() ? CareerRules.FORMAT_CONSTRUCTED : row.format;
        view.deck = gson.fromJson(row.deck, DeckCardLists.class);
        if (view.deck != null && (view.deck.getName() == null || view.deck.getName().isEmpty())) {
            view.deck.setName(row.name);
        }
        view.active = row.active;
        view.source = row.source;
        view.updated = row.updated;
        view.missing = ownershipProblemsFrom(owned, view.deck);
        return view;
    }

    private List<String> ownershipProblemsFrom(Map<String, Integer> owned, DeckCardLists deck) {
        if (deck == null) {
            return Collections.emptyList();
        }
        Map<String, Integer> needed = new LinkedHashMap<>();
        for (DeckCardInfo info : CareerContent.allCards(deck)) {
            if (BASIC_NAMES.contains(info.getCardName())) {
                continue;
            }
            needed.merge(info.getCardName(), info.getAmount(), Integer::sum);
        }
        List<String> problems = new ArrayList<>();
        for (Map.Entry<String, Integer> entry : needed.entrySet()) {
            int have = owned.getOrDefault(entry.getKey(), 0);
            if (have < entry.getValue()) {
                problems.add(entry.getKey() + ": " + entry.getValue() + " in the deck, " + have + " owned");
            }
        }
        return problems;
    }

    /** what a game paid, for the rewards screen; the latest game when the key is null */
    public CareerProgress.CareerGameResult gameResult(String user, String gameKey) throws SQLException {
        CareerStore store = CareerStore.get();
        // a ranked table's rewards are its latest game's, paid as any game against the AI
        UUID rankedGame = mage.server.ladder.RankedService.lastGame(gameKey);
        if (rankedGame != null) {
            gameKey = gameKey(rankedGame, user);
        }
        if (gameKey != null && !gameKey.isEmpty()) {
            // a payout the gateway makes (XMAGE_ROUTE_CAREERPAYOUTS=rust) lands a moment after the game
            CareerPayouts.awaitPayout(store, gameKey);
        }
        String json = gameKey == null || gameKey.isEmpty() ? store.latestGameDetails(user) : store.gameDetailsFor(gameKey, user);
        return json == null ? null : new Gson().fromJson(json, CareerProgress.CareerGameResult.class);
    }

    // ---- collection progress

    /** progress in every set the collection has cards from, the most complete first */
    public List<CareerSetProgress> setProgress(String user) throws SQLException {
        List<CareerCard> collection = CareerStore.get().collection(user);
        Set<String> ownedNames = new java.util.HashSet<>();
        Set<String> setCodes = new java.util.LinkedHashSet<>();
        for (CareerCard card : collection) {
            ownedNames.add(card.name);
            setCodes.add(card.setCode);
        }
        List<CareerSetProgress> progress = new ArrayList<>();
        for (String setCode : setCodes) {
            ExpansionSet set = Sets.findSet(setCode);
            if (set == null) {
                continue;
            }
            Set<String> names = setNames.computeIfAbsent(set.getCode(), code -> namesIn(set));
            if (names.isEmpty()) {
                continue;
            }
            CareerSetProgress view = new CareerSetProgress();
            view.setCode = set.getCode();
            view.name = set.getName();
            view.total = names.size();
            view.owned = (int) names.stream().filter(ownedNames::contains).count();
            progress.add(view);
        }
        progress.sort(Comparator.comparingDouble((CareerSetProgress view) -> -(double) view.owned / view.total)
                .thenComparing(view -> view.name));
        return progress;
    }

    private static Set<String> namesIn(ExpansionSet set) {
        mage.cards.repository.CardCriteria criteria = new mage.cards.repository.CardCriteria()
                .setCodes(set.getCode())
                .nightCard(false)
                .notSupertypes(SuperType.BASIC);
        if (set.getMaxCardNumberInBooster() > 0 && set.getMaxCardNumberInBooster() < Integer.MAX_VALUE) {
            criteria.maxCardNumber(set.getMaxCardNumberInBooster());
        }
        Set<String> names = new java.util.TreeSet<>();
        for (CardInfo card : CardRepository.instance.findCards(criteria)) {
            names.add(card.getName());
        }
        return names;
    }

    // ---- the shop

    private static final Set<String> COMMANDER_SHOP_SETS = new java.util.HashSet<>(java.util.Arrays.asList("CMR", "BBD"));
    private static final Set<String> MASTERS_SHOP_SETS = new java.util.HashSet<>(java.util.Arrays.asList(
            "MH3", "MH2", "2X2", "DMR", "RVR", "LTR"));

    /** sets with boosters, newest first: core sets, expansions, and curated Commander & Masters booster sets */
    public List<CareerShopSet> shop() {
        Date now = new Date();
        return Sets.getInstance().values().stream()
                .filter(ExpansionSet::hasBoosters)
                .filter(set -> set.getSetType() == SetType.EXPANSION || set.getSetType() == SetType.CORE
                        || COMMANDER_SHOP_SETS.contains(set.getCode()) || MASTERS_SHOP_SETS.contains(set.getCode()))
                .filter(set -> set.getReleaseDate() != null && !set.getReleaseDate().after(now))
                .sorted(Comparator.comparing(ExpansionSet::getReleaseDate).reversed())
                .map(set -> {
                    CareerShopSet view = new CareerShopSet();
                    view.setCode = set.getCode();
                    view.name = set.getName();
                    view.releaseDate = new java.text.SimpleDateFormat("yyyy-MM-dd").format(set.getReleaseDate());
                    view.price = CareerRules.PACK_PRICE;
                    view.category = COMMANDER_SHOP_SETS.contains(set.getCode()) ? "commander"
                            : MASTERS_SHOP_SETS.contains(set.getCode()) ? "masters" : "standard";
                    view.cover = packArt(set.getCode());
                    return view;
                })
                .collect(Collectors.toList());
    }

    private static final Map<String, java.util.Optional<CareerContent.CareerCover>> PACK_ART = new ConcurrentHashMap<>();

    /** the card on a set's packs (looked up once per set) */
    static CareerContent.CareerCover packArt(String setCode) {
        return PACK_ART.computeIfAbsent(setCode, code -> {
            try {
                return java.util.Optional.ofNullable(pickPackArt(CardRepository.instance.findCards(new mage.cards.repository.CardCriteria()
                        .setCodes(code).rarities(mage.constants.Rarity.MYTHIC, mage.constants.Rarity.RARE))));
            } catch (RuntimeException e) {
                return java.util.Optional.empty();
            }
        }).orElse(null);
    }

    /**
     * A set's face, the way its packs show one: a legendary creature or planeswalker before anything else, mythic
     * before rare, a creature before other spells, then the lowest collector number of the main set (no showcase or
     * extended-art reprints, no back faces).
     */
    static CareerContent.CareerCover pickPackArt(List<CardInfo> cards) {
        CardInfo best = null;
        int[] bestScore = null;
        for (CardInfo card : cards) {
            if (card.isNightCard() || card.getTypes().contains(mage.constants.CardType.LAND) || !card.getCardNumber().matches("\\d+")) {
                continue;
            }
            boolean legend = card.getSupertypes().contains(SuperType.LEGENDARY);
            boolean walker = card.getTypes().contains(mage.constants.CardType.PLANESWALKER);
            boolean creature = card.getTypes().contains(mage.constants.CardType.CREATURE);
            int[] score = {
                    (legend && (creature || walker)) ? 1 : 0,
                    card.getRarity() == mage.constants.Rarity.MYTHIC ? 1 : 0,
                    creature || walker ? 1 : 0,
                    -card.getCardNumberAsInt()
            };
            if (bestScore == null || ahead(score, bestScore)) {
                best = card;
                bestScore = score;
            }
        }
        if (best == null) {
            return null;
        }
        CareerContent.CareerCover cover = new CareerContent.CareerCover();
        cover.name = best.getName();
        cover.setCode = best.getSetCode();
        cover.cardNumber = best.getCardNumber();
        return cover;
    }

    private static boolean ahead(int[] score, int[] other) {
        for (int i = 0; i < score.length; i++) {
            if (score[i] != other[i]) {
                return score[i] > other[i];
            }
        }
        return false;
    }

    /**
     * The shop as one account sees it: the newest sets, then the sets campaign chapters opened for it. Sets a chapter
     * opens and the account hasn't opened yet are listed last, locked.
     */
    public List<CareerShopSet> shop(String user) throws SQLException {
        Map<String, String> openedBy = new LinkedHashMap<>();
        for (CareerCampaigns.Campaign campaign : CareerCampaigns.get().campaigns().values()) {
            for (CareerCampaigns.Chapter chapter : campaign.chapters) {
                if (chapter.unlockSet != null) {
                    openedBy.putIfAbsent(chapter.unlockSet, chapter.name);
                }
            }
        }
        return shelf(shop(), openedBy, CareerStore.get().setUnlocks(user));
    }

    /**
     * Every released expansion and core set with packs is for sale, newest first; a set a campaign chapter opens
     * stays locked (at the end of the list) until that chapter is beaten.
     */
    static List<CareerShopSet> shelf(List<CareerShopSet> all, Map<String, String> openedBy, Set<String> unlocked) {
        List<CareerShopSet> open = new ArrayList<>();
        List<CareerShopSet> locked = new ArrayList<>();
        for (CareerShopSet set : all) {
            if (openedBy.containsKey(set.setCode)) {
                // which chapter opens it: still to beat when locked, the one beaten when open (the hub announces it)
                set.unlockedBy = openedBy.get(set.setCode);
                if (unlocked.contains(set.setCode)) {
                    open.add(set);
                } else {
                    set.locked = true;
                    locked.add(set);
                }
            } else {
                open.add(set);
            }
        }
        open.addAll(locked);
        return open;
    }

    public CareerPackResult buyPack(String user, String setCode) throws SQLException, CareerException {
        ExpansionSet set = Sets.findSet(setCode);
        if (set == null || shop(user).stream().noneMatch(offer -> offer.setCode.equals(set.getCode()) && !offer.locked)) {
            throw new CareerException("The shop has no packs of " + setCode + ".");
        }
        return open(user, set.getCode(), booster(set));
    }

    /** a fresh booster of the set as Career cards */
    List<CareerCard> booster(ExpansionSet set) throws CareerException {
        List<Card> booster = set.createBooster();
        if (booster.isEmpty()) {
            throw new CareerException("Packs of " + set.getName() + " can't be opened right now.");
        }
        List<CareerCard> cards = new ArrayList<>();
        for (Card card : booster) {
            CardInfo info = cardLookup.apply(card.getExpansionSetCode() + "|" + card.getCardNumber());
            CareerCard careerCard = info != null ? toCareerCard(info)
                    : new CareerCard(card.getName(), card.getExpansionSetCode(), card.getCardNumber(), CareerRules.rarityOf(card.getRarity()));
            cards.add(careerCard);
        }
        return cards;
    }

    /** pays for a pack and adds its cards; copies past the playset become coins or wildcards */
    CareerPackResult open(String user, String setCode, List<CareerCard> cards) throws SQLException, CareerException {
        CareerStore store = CareerStore.get();
        refusing(() -> store.transaction(() -> {
            CareerProfile profile = store.profile(user);
            if (profile == null) {
                throw new SQLException(new CareerException("Open a Career first."));
            }
            if (profile.packTokens > 0) {
                // a free pack from a level or the weekly goal goes first
                store.addProfileInt(user, "pack_tokens", -1);
            } else if (profile.coins < CareerRules.PACK_PRICE) {
                throw new SQLException(new CareerException("A pack costs " + CareerRules.PACK_PRICE + " coins; you have " + profile.coins + "."));
            } else {
                store.addCoins(user, -CareerRules.PACK_PRICE);
            }
            addCards(store, user, cards);
            store.addStat(user, "packs_opened", 1);
            CareerProgress.get().achievementsIn(store, user, null, null, System.currentTimeMillis());
            return null;
        }));
        CareerPackResult result = new CareerPackResult();
        result.setCode = setCode;
        result.cards = cards;
        result.profile = store.profile(user);
        return result;
    }

    /**
     * Adds cards to the collection inside the caller's transaction. Copies past the playset become coins (commons,
     * uncommons) or a wildcard (rares, mythics), and say so in convertedTo. Basic lands are free and skipped.
     */
    void addCards(CareerStore store, String user, List<CareerCard> cards) throws SQLException {
        for (CareerCard card : cards) {
            if (CareerRules.LAND.equals(card.rarity)) {
                continue;
            }
            if (store.owned(user, card.name) >= CareerRules.PLAYSET) {
                int coins = CareerRules.spareCoins(card.rarity);
                if (coins > 0) {
                    store.addCoins(user, coins);
                    card.convertedTo = "coins";
                } else {
                    store.addWildcard(user, card.rarity, 1);
                    card.convertedTo = "wildcard";
                }
            } else {
                store.addCard(user, card, 1);
            }
        }
    }

    // ---- crafting

    public CareerProfile craft(String user, String setCode, String cardNumber) throws SQLException, CareerException {
        CardInfo info = cardLookup.apply(setCode + "|" + cardNumber);
        if (info == null) {
            throw new CareerException("There is no card " + setCode + " " + cardNumber + ".");
        }
        CareerCard card = toCareerCard(info);
        if (CareerRules.LAND.equals(card.rarity)) {
            throw new CareerException("Basic lands are free; there's nothing to craft.");
        }
        CareerStore store = CareerStore.get();
        refusing(() -> store.transaction(() -> {
            CareerProfile profile = store.profile(user);
            if (profile == null) {
                throw new SQLException(new CareerException("Open a Career first."));
            }
            if (store.owned(user, card.name) >= CareerRules.PLAYSET) {
                throw new SQLException(new CareerException("You already have " + CareerRules.PLAYSET + " copies of " + card.name + "."));
            }
            int coins = CareerRules.craftCoins(card.rarity);
            int ownWildcards = wildcardsOf(profile, card.rarity);
            if (coins > 0 && ownWildcards > 0) {
                // a common or uncommon wildcard (levels and campaign choices pay them) is spent before coins
                store.addWildcard(user, card.rarity, -1);
            } else if (coins > 0) {
                if (profile.coins < coins) {
                    throw new SQLException(new CareerException(card.name + " costs " + coins + " coins to craft; you have " + profile.coins + "."));
                }
                store.addCoins(user, -coins);
            } else {
                if (ownWildcards < 1) {
                    throw new SQLException(new CareerException(card.name + " takes a " + card.rarity + " wildcard; you have none."));
                }
                store.addWildcard(user, card.rarity, -1);
            }
            store.addCard(user, card, 1);
            store.addStat(user, "cards_crafted", 1);
            CareerProgress.get().achievementsIn(store, user, null, null, System.currentTimeMillis());
            return null;
        }));
        return store.profile(user);
    }

    static int wildcardsOf(CareerProfile profile, String rarity) {
        if (profile.wildcards == null) {
            return 0;
        }
        switch (rarity) {
            case CareerRules.COMMON:
                return profile.wildcards.common;
            case CareerRules.UNCOMMON:
                return profile.wildcards.uncommon;
            case CareerRules.RARE:
                return profile.wildcards.rare;
            case CareerRules.MYTHIC:
                return profile.wildcards.mythic;
            default:
                return 0;
        }
    }

    // ---- export and import

    static class ExportFile {
        /** 2 since the Show progression switch (M24) */
        int version = 2;
        String user;
        long exportedAt;
        CareerProfile profile;
        List<CareerCard> cards;
        List<CareerPayout> payouts;
        // since M8
        Map<String, Integer> stats;
        Map<String, Long> achievements;
        List<String[]> unlocks;
        int levelPaid;
        // since M29
        Map<String, String> cardStyles;
        // saved Career decks
        List<CareerStore.CareerDeckRow> savedDecks;
    }

    /** The account's Career, gone for good (an account deletion). False when it had none. */
    public boolean delete(String user) throws SQLException {
        CareerStore store = CareerStore.get();
        if (store.profile(user) == null) {
            return false;
        }
        store.transaction(() -> {
            store.deleteProfile(user);
            store.deleteLeague(user);
            return null;
        });
        return true;
    }

    /** the whole Career as a signed text file, for keeping a copy or moving it back after a loss */
    public String export(String user) throws SQLException, CareerException {
        CareerStore store = CareerStore.get();
        ExportFile file = new ExportFile();
        file.user = user;
        file.exportedAt = System.currentTimeMillis();
        file.profile = store.profile(user);
        if (file.profile == null) {
            throw new CareerException("There's no Career to export.");
        }
        file.cards = store.collection(user);
        file.payouts = store.payouts(user, Integer.MAX_VALUE);
        file.stats = store.stats(user);
        file.achievements = store.achievements(user);
        file.unlocks = store.unlocks(user);
        file.levelPaid = store.profileInt(user, "level_paid");
        file.cardStyles = store.cardStyles(user);
        file.savedDecks = store.careerDecks(user);
        String body = Base64.getEncoder().encodeToString(new Gson().toJson(file).getBytes(StandardCharsets.UTF_8));
        return "career1." + body + "." + sign(body, store.secret());
    }

    /**
     * Brings back an exported Career for an account that has none (lost, or moved to a new database). Only files this
     * server signed, for this account, are taken. An existing Career is never replaced, so an old file can't be used to
     * undo pack openings.
     */
    public CareerProfile importFile(String user, String text) throws SQLException, CareerException {
        CareerStore store = CareerStore.get();
        if (store.profile(user) != null) {
            throw new CareerException("You already have a Career; a Career file only brings one back to an account without one.");
        }
        String[] parts = text == null ? new String[0] : text.trim().split("\\.");
        if (parts.length != 3 || !"career1".equals(parts[0])) {
            throw new CareerException("That isn't a Career file.");
        }
        byte[] expected = sign(parts[1], store.secret()).getBytes(StandardCharsets.US_ASCII);
        if (!MessageDigest.isEqual(expected, parts[2].getBytes(StandardCharsets.US_ASCII))) {
            throw new CareerException("That Career file wasn't made by this server, or it was changed.");
        }
        ExportFile file;
        try {
            file = new Gson().fromJson(new String(Base64.getDecoder().decode(parts[1]), StandardCharsets.UTF_8), ExportFile.class);
        } catch (RuntimeException e) {
            throw new CareerException("That Career file can't be read.");
        }
        if (file == null || file.profile == null || file.user == null
                || !file.user.toLowerCase(Locale.ENGLISH).equals(user.toLowerCase(Locale.ENGLISH))) {
            throw new CareerException("That Career file belongs to another account.");
        }
        store.transaction(() -> {
            store.deleteProfile(user);
            if (!store.createProfile(user, file.profile.starter, file.profile.coins, file.profile.created)) {
                throw new SQLException("A Career appeared for " + user + " during the import");
            }
            store.restoreProgress(user, file.profile);
            for (CareerCard card : file.cards == null ? Collections.<CareerCard>emptyList() : file.cards) {
                store.addCard(user, card, card.count);
            }
            for (CareerPayout payout : file.payouts == null ? Collections.<CareerPayout>emptyList() : file.payouts) {
                store.restorePayout(user, payout);
            }
            if (file.stats != null) {
                for (Map.Entry<String, Integer> stat : file.stats.entrySet()) {
                    store.addStat(user, stat.getKey(), stat.getValue());
                }
            }
            if (file.achievements != null) {
                for (Map.Entry<String, Long> achievement : file.achievements.entrySet()) {
                    store.addAchievement(user, achievement.getKey(), achievement.getValue());
                }
            }
            if (file.unlocks != null) {
                for (String[] unlock : file.unlocks) {
                    store.addUnlock(user, unlock[0], unlock[1], file.exportedAt);
                }
            }
            // levels an old file reached were paid; newer files say so exactly
            store.setProfileInt(user, "level_paid", file.levelPaid > 0 ? file.levelPaid : CareerRules.levelFor(file.profile.xp));
            store.setProfileInt(user, "pack_tokens", file.profile.packTokens);
            store.setProfileInt(user, "capsule_tokens", file.profile.capsuleTokens);
            if (file.cardStyles != null) {
                for (Map.Entry<String, String> style : file.cardStyles.entrySet()) {
                    store.setCardStyle(user, style.getKey(), style.getValue());
                }
            }
            if (file.savedDecks != null) {
                for (CareerStore.CareerDeckRow savedDeck : file.savedDecks) {
                    store.putCareerDeck(user, savedDeck);
                    if (savedDeck.active) {
                        store.putDeck(user, savedDeck.deck, savedDeck.updated);
                    }
                }
            }
            store.setProfileInt(user, "show_progress", file.profile.showProgress || file.version < 2 ? 1 : 0);
            return null;
        });
        return store.profile(user);
    }

    static String sign(String body, String secret) {
        try {
            Mac mac = Mac.getInstance("HmacSHA256");
            mac.init(new SecretKeySpec(secret.getBytes(StandardCharsets.UTF_8), "HmacSHA256"));
            return Base64.getUrlEncoder().withoutPadding().encodeToString(mac.doFinal(body.getBytes(StandardCharsets.UTF_8)));
        } catch (GeneralSecurityException e) {
            throw new IllegalStateException(e);
        }
    }

    /** A Career rule said no; the message is for the player. */
    public static class CareerException extends Exception {
        public CareerException(String message) {
            super(message);
        }
    }

    interface Refusable {
        void run() throws SQLException;
    }

    /** a refusal is thrown out of a store transaction wrapped in an SQLException (to roll it back); unwrap it */
    static void refusing(Refusable work) throws SQLException, CareerException {
        try {
            work.run();
        } catch (SQLException e) {
            if (e.getCause() instanceof CareerException) {
                throw (CareerException) e.getCause();
            }
            throw e;
        }
    }
}
