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
        /** a campaign chapter has to open it first */
        public boolean locked;
        /** the campaign chapter that opens it (to beat while locked, beaten once open); null for the newest sets */
        public String unlockedBy;
        /** the card whose art is on the set's packs; null without a card database */
        public CareerContent.CareerCover cover;
    }

    /** the newest sets are always in the shop; campaign chapters open older ones */
    public static final int SHOP_NEWEST_SETS = 6;

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
        boolean created = store.transaction(() -> {
            // progression made the profile with the account's first counted game; picking a starter opens the collection
            if (!store.createProfile(user, starterId, CareerRules.START_COINS, System.currentTimeMillis())
                    && !store.openCollection(user, starterId, CareerRules.START_COINS)) {
                return false;
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
        // each tier in order of measured difficulty, easiest first (S039); the rival after the rest of its tier
        List<CareerContent.Opponent> ordered = new ArrayList<>(content.opponents());
        ordered.sort(java.util.Comparator.comparingInt((CareerContent.Opponent opponent) -> opponent.tier)
                .thenComparing(opponent -> opponent.rival != null)
                .thenComparing(opponent -> opponent.raid)
                .thenComparingInt(opponent -> -CareerContent.measuredWinRate(opponent)));
        for (CareerContent.Opponent opponent : ordered) {
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
            view.nemesis = nemesis != null && opponent.id.equals(nemesis.opponentId);
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
            views.add(view);
        }
        return views;
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
        CareerRules.Reward reward = assistedReward(modeReward(table.kind, table.tier, won, turns), table.assisted);
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
            final CareerTally played = counted;
            CareerProgress.CareerGameResult result = store.transaction(() -> {
                CareerProfile before = store.profile(table.user);
                if (before == null || !store.payoutIn(tableId.toString(), table.user, table.opponentId, won,
                        reward.coins, reward.xp, reward.note, now)) {
                    return null;
                }
                store.claimGame(tableId.toString(), table.user, now);
                CareerProgress.CareerGameResult paid = new CareerProgress.CareerGameResult();
                paid.gameKey = tableId.toString();
                paid.kind = CareerRewards.CAREER;
                paid.career = true;
                paid.won = won;
                paid.opponent = table.opponentId;
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
                        paid.note != null ? paid.note : table.kind + " " + table.opponentId);
                return paid;
            });
            if (result == null) {
                return null;
            }
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

    private void finishMode(CareerStore store, CareerTable table, boolean won, int turns, CareerTally tally,
                            CareerProgress.CareerGameResult result, long now) throws SQLException {
        switch (table.kind) {
            case "campaign":
            case "trial":
                CareerCampaigns.get().finished(store, table.user, table.ref, won, result, now);
                break;
            case "gauntlet":
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
                CareerHordes.get().finished(store, table.user, table.ref, won, tally, result, now);
                break;
            case "duel":
                CareerLadder.finished(store, table.user, table.opponentId, won, result, now);
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
        CareerRewards rewards = CareerRewards.get();
        CareerStore store = CareerStore.get();
        long now = System.currentTimeMillis();
        String day = CareerProgress.get().day();
        List<Seat> humans = finished.seats;
        boolean capped = CareerRewards.PERSON.equals(finished.kind) || CareerRewards.POD.equals(finished.kind);
        try {
            store.transaction(() -> {
                for (Seat seat : humans) {
                    if (seat.tally == null || seat.tally.taint() != null) {
                        continue;
                    }
                    String key = gameKey(finished.gameId, seat.user);
                    ensureProfile(store, seat.user, now);
                    if (!store.claimGame(key, seat.user, now)) {
                        continue;
                    }
                    CareerProgress.CareerGameResult result = new CareerProgress.CareerGameResult();
                    result.gameKey = key;
                    result.kind = finished.kind;
                    result.won = seat.won;
                    CareerProfile before = store.profile(seat.user);
                    result.xpBefore = before.xp;
                    String reason = null;
                    if (finished.turns < rewards.turnFloor()) {
                        reason = "The game ended before turn " + rewards.turnFloor() + ", so it paid nothing.";
                    } else if (capped) {
                        for (Seat other : humans) {
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
                    seat.tally.context(finished.kind, finished.format, seat.colors, seat.won, finished.bo3 && seat.matchWon);
                    seat.tally.finish(finished.game, seat.won, finished.turns);
                    CareerProgress.get().apply(store, seat.user, seat.tally, result, before.xp, now);
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
                return null;
            });
        } catch (SQLException | RuntimeException e) {
            logger.error("Career progress failed for game " + finished.gameId, e);
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

    /**
     * Saves the account's Career deck (B311), checked against the collection: every card the deck has more copies of
     * than the account owns is a problem, and nothing is saved.
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
        store.putDeck(user, new Gson().toJson(deck), System.currentTimeMillis());
    }

    /** the account's saved Career deck, or null */
    public DeckCardLists deck(String user) throws SQLException {
        String json = CareerStore.get().deck(user);
        return json == null ? null : new Gson().fromJson(json, DeckCardLists.class);
    }

    /** what a game paid, for the rewards screen; the latest game when the key is null */
    public CareerProgress.CareerGameResult gameResult(String user, String gameKey) throws SQLException {
        CareerStore store = CareerStore.get();
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

    /** sets with boosters, newest first: core sets and expansions that are out */
    public List<CareerShopSet> shop() {
        Date now = new Date();
        return Sets.getInstance().values().stream()
                .filter(ExpansionSet::hasBoosters)
                .filter(set -> set.getSetType() == SetType.EXPANSION || set.getSetType() == SetType.CORE)
                .filter(set -> set.getReleaseDate() != null && !set.getReleaseDate().after(now))
                .sorted(Comparator.comparing(ExpansionSet::getReleaseDate).reversed())
                .map(set -> {
                    CareerShopSet view = new CareerShopSet();
                    view.setCode = set.getCode();
                    view.name = set.getName();
                    view.releaseDate = new java.text.SimpleDateFormat("yyyy-MM-dd").format(set.getReleaseDate());
                    view.price = CareerRules.PACK_PRICE;
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
        Set<String> unlocked = CareerStore.get().setUnlocks(user);
        List<CareerShopSet> all = shop();
        List<CareerShopSet> open = new ArrayList<>();
        List<CareerShopSet> locked = new ArrayList<>();
        int newest = 0;
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
            } else if (newest < SHOP_NEWEST_SETS) {
                newest++;
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
        return open(user, set.getCode(), cards);
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

    private static int wildcardsOf(CareerProfile profile, String rarity) {
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
    }

    /** The account's Career, gone for good (an account deletion). False when it had none. */
    public boolean delete(String user) throws SQLException {
        CareerStore store = CareerStore.get();
        if (store.profile(user) == null) {
            return false;
        }
        store.transaction(() -> {
            store.deleteProfile(user);
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
