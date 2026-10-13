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
            if (!store.createProfile(user, starterId, CareerRules.START_COINS, System.currentTimeMillis())) {
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
        for (CareerContent.Opponent opponent : content.opponents()) {
            CareerOpponent view = new CareerOpponent();
            view.id = opponent.id;
            view.name = opponent.name;
            view.tier = opponent.tier;
            view.tierName = content.tier(opponent.tier).name;
            view.colors = opponent.colors;
            view.tagline = opponent.tagline;
            view.wins = wins.getOrDefault(opponent.id, 0);
            view.unlocked = opponent.tier == 1 || winsByTier.getOrDefault(opponent.tier - 1, 0) >= CareerRules.WINS_TO_UNLOCK_NEXT_TIER;
            view.winCoins = CareerRules.winCoins(opponent.tier);
            view.cover = content.cover(opponent);
            view.lines = opponent.lines;
            views.add(view);
        }
        return views;
    }

    public boolean unlocked(String user, String opponentId) throws SQLException {
        return opponents(user).stream().anyMatch(view -> view.id.equals(opponentId) && view.unlocked);
    }

    // ---- Career tables and payouts

    public void register(UUID tableId, String user, String opponentId, int tier) {
        tables.put(tableId, new CareerTable(user, opponentId, tier));
    }

    /** a Career table of a mode other than the roster duel */
    public void register(UUID tableId, String user, String kind, String ref, CareerSetup setup) {
        tables.put(tableId, new CareerTable(user, kind + ":" + ref, 1, kind, ref, setup));
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
            tables.put(tableId, table);
        }
    }

    public void forget(UUID tableId) {
        if (tableId != null) {
            tables.remove(tableId);
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
        CareerRules.Reward reward = modeReward(table.kind, table.tier, won, turns);
        CareerStore store = CareerStore.get();
        try {
            long now = System.currentTimeMillis();
            CareerTally counted = tally != null && tally.taint() == null ? tally : CareerTally.forTest(null);
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
                paid.career = true;
                paid.won = won;
                paid.opponent = table.opponentId;
                paid.coins = reward.coins;
                paid.xp = reward.xp;
                paid.note = reward.note;
                Set<Integer> tiersBefore = openTiers(table.user);
                finishMode(store, table, won, turns, played, paid, now);
                CareerProgress.get().apply(store, table.user, played, paid, before.xp, now);
                for (CareerOpponent opponent : opponents(table.user)) {
                    if (opponent.unlocked && !tiersBefore.contains(opponent.tier) && !paid.opened.contains(opponent.tierName)) {
                        paid.opened.add(opponent.tierName);
                    }
                }
                store.setGameDetails(paid.gameKey, new Gson().toJson(paid));
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
                CareerPuzzles.get().finished(store, table.user, table.ref, won, result, now);
                break;
            case "challenge":
                CareerChallenges.get().finished(store, table.user, table.ref, won, tally, result, now);
                break;
            case "limited":
                CareerLimited.get().finished(store, table.user, table.ref, won, result, now);
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

    /**
     * A regular game against the AI ended. When the player opted in (and the game wasn't practice, a goldfish or a
     * cheat), it counts for quests, achievements and stats, once per game. It never pays the match itself.
     *
     * @return what it paid, or null when it doesn't count
     */
    public CareerProgress.CareerGameResult aiGameEnded(UUID gameId, String user, mage.game.Game game, boolean won, int turns, CareerTally tally) {
        if (tally == null || tally.taint() != null) {
            return null;
        }
        CareerStore store = CareerStore.get();
        try {
            CareerProfile profile = store.profile(user);
            if (profile == null || !profile.countAiGames) {
                return null;
            }
            tally.finish(game, won, turns);
            long now = System.currentTimeMillis();
            return store.transaction(() -> {
                if (!store.claimGame(gameId.toString(), user, now)) {
                    return null;
                }
                CareerProgress.CareerGameResult result = new CareerProgress.CareerGameResult();
                result.gameKey = gameId.toString();
                result.won = won;
                CareerProgress.get().apply(store, user, tally, result, store.profile(user).xp, now);
                store.setGameDetails(result.gameKey, new Gson().toJson(result));
                return result;
            });
        } catch (SQLException | RuntimeException e) {
            logger.error("Career progress failed for " + user + " in game " + gameId, e);
            return null;
        }
    }

    /** what a game paid, for the rewards screen; the latest game when the key is null */
    public CareerProgress.CareerGameResult gameResult(String user, String gameKey) throws SQLException {
        CareerStore store = CareerStore.get();
        String json = gameKey == null || gameKey.isEmpty() ? store.latestGameDetails(user) : store.gameDetails(gameKey, user);
        return json == null ? null : new Gson().fromJson(json, CareerProgress.CareerGameResult.class);
    }

    /** whether regular games against the AI count for quests and achievements */
    public CareerProfile setCountAiGames(String user, boolean count) throws SQLException, CareerException {
        CareerStore store = CareerStore.get();
        if (store.profile(user) == null) {
            throw new CareerException("Open a Career first.");
        }
        store.setProfileInt(user, "count_ai_games", count ? 1 : 0);
        return store.profile(user);
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
        int version = 1;
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
            store.setProfileInt(user, "count_ai_games", file.profile.countAiGames ? 1 : 0);
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

    private interface Refusable {
        void run() throws SQLException;
    }

    /** a refusal is thrown out of a store transaction wrapped in an SQLException (to roll it back); unwrap it */
    private static void refusing(Refusable work) throws SQLException, CareerException {
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
