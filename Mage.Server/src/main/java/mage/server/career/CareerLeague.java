package mage.server.career;

import com.google.gson.Gson;
import mage.cards.ExpansionSet;
import mage.cards.Sets;
import mage.cards.decks.Constructed;
import mage.cards.decks.DeckCardInfo;
import mage.cards.decks.DeckCardLists;
import mage.cards.decks.DeckValidator;
import mage.server.ladder.BotRoster;
import mage.server.ladder.LadderRank;
import mage.server.ladder.RankedService;
import mage.server.ladder.DeckLegality;
import org.apache.log4j.Logger;

import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.ThreadLocalRandom;
import java.util.stream.Collectors;

/**
 * The Career League (#433): the ranked ladder played with cards the account owns, per format. Starting a League in a
 * format adds two of its starter decks to the collection (and as League decks to edit), the format's packs and, with
 * the account's first League, a few wildcards. Packs are opened from the format's own sets. League games are ranked
 * games in the League queue (see {@link RankedService#LEAGUE}); what climbing pays is here: a pack of the format for
 * each tier gained, a rare wildcard per rank (mythic at Gold), an uncommon one every five League wins.
 * <p>
 * The collection is the one Career collection: League packs add to it and League decks are built from it. The Story
 * deck and its shop are untouched, and the League's packs ignore the campaign set locks (they're the format's sets).
 */
public final class CareerLeague {

    private static final Logger logger = Logger.getLogger(CareerLeague.class);

    /** a League's starting decks: two of the format's starters */
    public static final int STARTERS = 2;
    /** League decks one account can keep */
    public static final int MAX_DECKS = 40;
    static final int MAX_NAME = 60;
    /** an uncommon wildcard every this many League wins */
    public static final int WINS_PER_WILDCARD = 5;
    /** the pack that draws from every set of the format */
    public static final String MIXED = "mixed";

    /** league.json */
    static final class Config {
        Map<String, Integer> startWildcards = new LinkedHashMap<>();
        Map<String, FormatConfig> formats = new LinkedHashMap<>();
    }

    static final class FormatConfig {
        List<String> starters = new ArrayList<>();
        int packs;
        boolean mixed;
    }

    // ---- views

    /** A deck a League can start from */
    public static class LeagueStarter {
        public String id;
        public String name;
        /** WUBRG letters */
        public String colors;
        public int cards;
    }

    /** One format's League, as the account sees it */
    public static class LeagueFormat {
        public String format;
        public boolean started;
        /** pack tokens of this format */
        public int tokens;
        public int decks;
        /** the starters to pick two of, while not started */
        public List<LeagueStarter> starters;
        public int startPacks;
        public boolean mixed;
    }

    /** The account's Leagues */
    public static class LeagueState {
        public List<LeagueFormat> formats;
        /** the account's first League also gives these */
        public CareerProfile.Wildcards startWildcards;
        /** whether the next League start is the first (and so gives the wildcards) */
        public boolean first;
        public CareerProfile.Wildcards wildcards;
        public int coins;
        /** Career's free pack tokens, spent on a League pack when the format has none */
        public int packTokens;
        public int packPrice;
    }

    /** A set whose packs a format's League opens */
    public static class LeagueSet {
        public String setCode;
        public String name;
        public String releaseDate;
        public CareerContent.CareerCover cover;
    }

    /** A League deck: the list, and what keeps it from being played */
    public static class LeagueDeck {
        public String id;
        public String name;
        public String format;
        public long updated;
        public DeckCardLists deck;
        /** main deck cards */
        public int cards;
        /** cards the account owns fewer copies of ("Name: 3 in the deck, 2 owned") */
        public List<String> missing;
        /** why the format's rules refuse it (the first few) */
        public List<String> illegal;
        /** what crafting the missing cards takes */
        public LeagueCraft craft;
    }

    /**
     * What crafting a deck's missing cards takes: wildcards by rarity and, for commons and uncommons once their
     * wildcards run out, coins (as Career crafting spends them)
     */
    public static class LeagueCraft {
        /** missing copies, all rarities */
        public int copies;
        public int common;
        public int uncommon;
        public int rare;
        public int mythic;
        public int coins;
        /** whether the account has all of it now */
        public boolean affordable;
        /** cards the server can't find, which can't be crafted */
        public List<String> unknown = new ArrayList<>();
    }

    /** What a League pack gave */
    public static class LeaguePack {
        public String format;
        public String setCode;
        public List<CareerCard> cards;
        public CareerProfile profile;
        /** the format's pack tokens left */
        public int tokens;
        /** what paid for it: league, career or coins */
        public String paidWith;
    }

    /** What starting a League gave */
    public static class LeagueStart {
        public String format;
        public List<LeagueDeck> decks;
        public int packs;
        public CareerProfile.Wildcards wildcards;
        public LeagueState state;
    }

    // ---- the service

    private static volatile CareerLeague instance;

    private final Config config;
    private final Gson gson = new Gson();

    CareerLeague(Config config) {
        this.config = config;
    }

    public static CareerLeague get() {
        CareerLeague current = instance;
        if (current == null) {
            synchronized (CareerLeague.class) {
                current = instance;
                if (current == null) {
                    current = new CareerLeague(load());
                    instance = current;
                }
            }
        }
        return current;
    }

    /** for tests */
    public static void use(CareerLeague league) {
        instance = league;
    }

    private static Config load() {
        InputStream stream = CareerLeague.class.getResourceAsStream("/league/league.json");
        if (stream == null) {
            logger.warn("League: league.json is missing; no League formats");
            return new Config();
        }
        try (Reader reader = new InputStreamReader(stream, StandardCharsets.UTF_8)) {
            Config config = new Gson().fromJson(reader, Config.class);
            return config == null ? new Config() : config;
        } catch (IOException | RuntimeException e) {
            logger.error("League: league.json can't be read", e);
            return new Config();
        }
    }

    /** the formats with a League: configured here and with a ranked ladder */
    public List<String> formats() {
        List<String> out = new ArrayList<>();
        for (String format : config.formats.keySet()) {
            if (RankedService.isFormat(format)) {
                out.add(format);
            }
        }
        return out;
    }

    public boolean isFormat(String format) {
        return format != null && formats().contains(format);
    }

    private FormatConfig formatConfig(String format) throws CareerService.CareerException {
        if (!isFormat(format)) {
            throw new CareerService.CareerException("There is no League for " + format + ".");
        }
        return config.formats.get(format);
    }

    CareerProfile.Wildcards startWildcards() {
        Map<String, Integer> w = config.startWildcards;
        return new CareerProfile.Wildcards(w.getOrDefault("common", 0), w.getOrDefault("uncommon", 0),
                w.getOrDefault("rare", 0), w.getOrDefault("mythic", 0));
    }

    public LeagueState state(String user) throws SQLException {
        CareerStore store = CareerStore.get();
        Map<String, Long> started = store.leagues(user);
        LeagueState view = new LeagueState();
        view.formats = new ArrayList<>();
        Map<String, Integer> deckCounts = new LinkedHashMap<>();
        for (CareerStore.LeagueDeckRow row : store.leagueDecks(user, null)) {
            deckCounts.merge(row.format, 1, Integer::sum);
        }
        for (String format : formats()) {
            FormatConfig config = this.config.formats.get(format);
            LeagueFormat f = new LeagueFormat();
            f.format = format;
            f.started = started.containsKey(format);
            f.tokens = store.leagueTokens(user, format);
            f.decks = deckCounts.getOrDefault(format, 0);
            f.startPacks = config.packs;
            f.mixed = config.mixed;
            if (!f.started) {
                f.starters = starters(format);
            }
            view.formats.add(f);
        }
        view.first = started.isEmpty();
        view.startWildcards = startWildcards();
        CareerProfile profile = store.profile(user);
        view.wildcards = profile == null ? new CareerProfile.Wildcards(0, 0, 0, 0) : profile.wildcards;
        view.coins = profile == null ? 0 : profile.coins;
        view.packTokens = profile == null ? 0 : profile.packTokens;
        view.packPrice = CareerRules.PACK_PRICE;
        return view;
    }

    /** the decks a format's League starts from, as the player picks among them */
    public List<LeagueStarter> starters(String format) {
        FormatConfig config = this.config.formats.get(format);
        List<LeagueStarter> out = new ArrayList<>();
        if (config == null) {
            return out;
        }
        for (String file : config.starters) {
            try {
                DeckCardLists deck = CareerCampaigns.get().deck(file);
                LeagueStarter starter = new LeagueStarter();
                starter.id = file;
                starter.name = deck.getName() == null || deck.getName().isEmpty() ? file : deck.getName();
                starter.colors = colors(file);
                starter.cards = mainCount(deck);
                out.add(starter);
            } catch (IOException e) {
                logger.warn("League: starter " + file + " can't be read", e);
            }
        }
        return out;
    }

    private static String colors(String file) {
        for (BotRoster.Bot bot : RankedService.get().roster().bots()) {
            if (file.equals(bot.deck) && bot.colors != null) {
                return bot.colors;
            }
        }
        return "";
    }

    static int mainCount(DeckCardLists deck) {
        int n = 0;
        for (DeckCardInfo info : deck.getCards()) {
            n += info.getAmount();
        }
        return n;
    }

    /**
     * Starts the account's League in a format with two of its starters: their cards join the collection (up to a
     * playset of each name; nothing past it turns into coins or wildcards), each becomes a League deck, the format's
     * packs are added and, with the account's first League, the starting wildcards. All of it in one transaction, once
     * per format.
     */
    public LeagueStart start(String user, String format, List<String> starterIds) throws SQLException, CareerService.CareerException {
        FormatConfig config = formatConfig(format);
        Set<String> picked = new HashSet<>(starterIds == null ? Collections.<String>emptyList() : starterIds);
        if (picked.size() != STARTERS || !config.starters.containsAll(picked)) {
            throw new CareerService.CareerException("Pick two of the " + name(format) + " starter decks.");
        }
        List<String> ordered = config.starters.stream().filter(picked::contains).collect(Collectors.toList());
        Map<String, DeckCardLists> decks = new LinkedHashMap<>();
        for (String id : ordered) {
            try {
                decks.put(id, CareerCampaigns.get().deck(id));
            } catch (IOException e) {
                throw new CareerService.CareerException("The starter deck " + id + " can't be read right now.");
            }
        }
        CareerStore store = CareerStore.get();
        CareerService service = CareerService.get();
        long now = System.currentTimeMillis();
        LeagueStart result = new LeagueStart();
        result.format = format;
        result.decks = new ArrayList<>();
        result.packs = config.packs;
        CareerService.refusing(() -> store.transaction(() -> {
            CareerService.ensureProfile(store, user, now);
            boolean first = store.leagues(user).isEmpty();
            if (!store.startLeague(user, format, String.join(",", ordered), now)) {
                throw new SQLException(new CareerService.CareerException("You already started the " + name(format) + " League."));
            }
            for (DeckCardLists deck : decks.values()) {
                for (DeckCardInfo info : CareerContent.allCards(deck)) {
                    CareerCard card = service.resolveCard(info);
                    if (card == null || CareerRules.LAND.equals(card.rarity)) {
                        continue;
                    }
                    int room = CareerRules.PLAYSET - store.owned(user, card.name);
                    int add = Math.min(info.getAmount(), room);
                    if (add > 0) {
                        store.addCard(user, card, add);
                    }
                }
            }
            store.addLeagueTokens(user, format, config.packs);
            if (first) {
                CareerProfile.Wildcards w = startWildcards();
                store.addWildcard(user, CareerRules.COMMON, w.common);
                store.addWildcard(user, CareerRules.UNCOMMON, w.uncommon);
                store.addWildcard(user, CareerRules.RARE, w.rare);
                store.addWildcard(user, CareerRules.MYTHIC, w.mythic);
                result.wildcards = w;
            }
            for (Map.Entry<String, DeckCardLists> entry : decks.entrySet()) {
                CareerStore.LeagueDeckRow row = new CareerStore.LeagueDeckRow();
                row.id = UUID.randomUUID().toString();
                row.name = clipName(entry.getValue().getName(), name(format) + " starter");
                row.format = format;
                entry.getValue().setName(row.name);
                row.deck = gson.toJson(entry.getValue());
                row.updated = now;
                store.putLeagueDeck(user, row);
            }
            return null;
        }));
        for (CareerStore.LeagueDeckRow row : store.leagueDecks(user, format)) {
            result.decks.add(view(user, row));
        }
        result.state = state(user);
        logger.info("League " + format + ": " + user + " started with " + String.join(", ", ordered));
        return result;
    }

    // ---- packs

    /** the sets a format's League opens packs of: released expansions and core sets with packs that the format allows */
    public List<LeagueSet> sets(String format) throws CareerService.CareerException {
        FormatConfig config = formatConfig(format);
        Set<String> legal = new HashSet<>(legalSets(format));
        List<LeagueSet> out = new ArrayList<>();
        for (CareerService.CareerShopSet offer : CareerService.get().shop()) {
            if (legal.contains(offer.setCode)) {
                LeagueSet set = new LeagueSet();
                set.setCode = offer.setCode;
                set.name = offer.name;
                set.releaseDate = offer.releaseDate;
                set.cover = offer.cover;
                out.add(set);
            }
        }
        if (config.mixed && out.size() > 1) {
            LeagueSet mixed = new LeagueSet();
            mixed.setCode = MIXED;
            mixed.name = name(format) + " pack";
            mixed.cover = out.get(0).cover;
            out.add(0, mixed);
        }
        return out;
    }

    /** the set codes the format's validator allows */
    static List<String> legalSets(String format) {
        DeckValidator validator = DeckLegality.validator(RankedService.deckType(format));
        if (validator instanceof Constructed) {
            return ((Constructed) validator).getSetCodes();
        }
        return Collections.emptyList();
    }

    /**
     * Opens a pack of the format's League: a set of the format, or the mixed pack (a pack of a random set of the
     * format). Paid with the format's pack token first, then Career's free pack token, then coins.
     */
    public LeaguePack openPack(String user, String format, String setCode) throws SQLException, CareerService.CareerException {
        CareerStore store = CareerStore.get();
        if (!store.leagues(user).containsKey(format)) {
            formatConfig(format);
            throw new CareerService.CareerException("Start the " + name(format) + " League first.");
        }
        List<LeagueSet> sets = sets(format);
        List<String> real = sets.stream().map(s -> s.setCode).filter(code -> !MIXED.equals(code)).collect(Collectors.toList());
        String code;
        if (MIXED.equals(setCode) && sets.stream().anyMatch(s -> MIXED.equals(s.setCode))) {
            code = real.get(ThreadLocalRandom.current().nextInt(real.size()));
        } else if (real.contains(setCode)) {
            code = setCode;
        } else {
            throw new CareerService.CareerException("The " + name(format) + " League has no packs of " + setCode + ".");
        }
        ExpansionSet set = Sets.findSet(code);
        if (set == null) {
            throw new CareerService.CareerException("Packs of " + code + " can't be opened right now.");
        }
        CareerService service = CareerService.get();
        List<CareerCard> cards = service.booster(set);
        LeaguePack result = new LeaguePack();
        CareerService.refusing(() -> store.transaction(() -> {
            CareerProfile profile = store.profile(user);
            if (store.leagueTokens(user, format) > 0) {
                store.addLeagueTokens(user, format, -1);
                result.paidWith = "league";
            } else if (profile != null && profile.packTokens > 0) {
                store.addProfileInt(user, "pack_tokens", -1);
                result.paidWith = "career";
            } else if (profile != null && profile.coins >= CareerRules.PACK_PRICE) {
                store.addCoins(user, -CareerRules.PACK_PRICE);
                result.paidWith = "coins";
            } else {
                throw new SQLException(new CareerService.CareerException("A pack costs " + CareerRules.PACK_PRICE + " coins, or a pack token."));
            }
            service.addCards(store, user, cards);
            store.addStat(user, "packs_opened", 1);
            CareerProgress.get().achievementsIn(store, user, null, null, System.currentTimeMillis());
            return null;
        }));
        result.format = format;
        result.setCode = set.getCode();
        result.cards = cards;
        result.profile = store.profile(user);
        result.tokens = store.leagueTokens(user, format);
        return result;
    }

    // ---- decks

    public List<LeagueDeck> decks(String user, String format) throws SQLException {
        List<LeagueDeck> out = new ArrayList<>();
        for (CareerStore.LeagueDeckRow row : CareerStore.get().leagueDecks(user, format)) {
            out.add(view(user, row));
        }
        return out;
    }

    public LeagueDeck deck(String user, String id) throws SQLException, CareerService.CareerException {
        CareerStore.LeagueDeckRow row = CareerStore.get().leagueDeck(user, id);
        if (row == null) {
            throw new CareerService.CareerException("There is no such League deck.");
        }
        return view(user, row);
    }

    /**
     * Saves a League deck (a new one when id is null). It may be unfinished: what keeps it from being played comes
     * back with it, and play checks again.
     */
    public LeagueDeck saveDeck(String user, String id, String name, String format, DeckCardLists deck) throws SQLException, CareerService.CareerException {
        formatConfig(format);
        if (deck == null) {
            throw new CareerService.CareerException("The deck is empty.");
        }
        CareerStore store = CareerStore.get();
        CareerStore.LeagueDeckRow row;
        if (id == null || id.isEmpty()) {
            if (store.leagueDeckCount(user) >= MAX_DECKS) {
                throw new CareerService.CareerException("You can keep " + MAX_DECKS + " League decks; delete one first.");
            }
            row = new CareerStore.LeagueDeckRow();
            row.id = UUID.randomUUID().toString();
        } else {
            row = store.leagueDeck(user, id);
            if (row == null) {
                throw new CareerService.CareerException("There is no such League deck.");
            }
        }
        row.name = clipName(name, name(format) + " deck");
        row.format = format;
        deck.setName(row.name);
        row.deck = gson.toJson(deck);
        row.updated = System.currentTimeMillis();
        store.putLeagueDeck(user, row);
        return view(user, row);
    }

    public boolean deleteDeck(String user, String id) throws SQLException {
        return CareerStore.get().deleteLeagueDeck(user, id);
    }

    /**
     * The stored League deck the player wants to play in a format, checked: owned and legal there. Throws with what's
     * wrong otherwise. The server's copy is played, never a list the client sends.
     */
    public LeagueDeck playable(String user, String format, String id) throws SQLException, CareerService.CareerException {
        CareerStore.LeagueDeckRow row = CareerStore.get().leagueDeck(user, id);
        if (row == null) {
            throw new CareerService.CareerException("There is no such League deck.");
        }
        if (!row.format.equals(format)) {
            throw new CareerService.CareerException("That deck is a " + name(row.format) + " deck.");
        }
        if (!CareerStore.get().leagues(user).containsKey(format)) {
            throw new CareerService.CareerException("Start the " + name(format) + " League first.");
        }
        LeagueDeck view = view(user, row);
        if (!view.missing.isEmpty()) {
            throw new CareerService.CareerException("You don't own every card in " + view.name + ": "
                    + String.join("; ", view.missing.subList(0, Math.min(3, view.missing.size()))) + ".");
        }
        if (!view.illegal.isEmpty()) {
            throw new CareerService.CareerException(view.name + " isn't legal in " + name(format) + ": " + String.join("; ", view.illegal) + ".");
        }
        return view;
    }

    private LeagueDeck view(String user, CareerStore.LeagueDeckRow row) throws SQLException {
        LeagueDeck view = new LeagueDeck();
        view.id = row.id;
        view.name = row.name;
        view.format = row.format;
        view.updated = row.updated;
        view.deck = gson.fromJson(row.deck, DeckCardLists.class);
        if (view.deck == null) {
            view.deck = new DeckCardLists();
        }
        view.cards = mainCount(view.deck);
        view.missing = CareerService.get().ownershipProblems(user, view.deck);
        view.craft = craftPlan(user, view.deck, CareerStore.get().profile(user));
        try {
            view.illegal = DeckLegality.problems(RankedService.deckType(row.format), view.deck);
        } catch (RuntimeException e) {
            view.illegal = Collections.singletonList(e.getMessage());
        }
        return view;
    }

    // ---- crafting what a deck lacks

    /** the copies of each card the deck has more of than the account owns, as the deck's printings */
    private static Map<CareerCard, Integer> shortfall(String user, DeckCardLists deck) throws SQLException {
        CareerStore store = CareerStore.get();
        CareerService service = CareerService.get();
        Map<String, Integer> needed = new LinkedHashMap<>();
        Map<String, DeckCardInfo> printing = new LinkedHashMap<>();
        for (DeckCardInfo info : CareerContent.allCards(deck)) {
            if (CareerService.isBasic(info.getCardName())) {
                continue;
            }
            needed.merge(info.getCardName(), info.getAmount(), Integer::sum);
            printing.putIfAbsent(info.getCardName(), info);
        }
        Map<CareerCard, Integer> out = new LinkedHashMap<>();
        for (Map.Entry<String, Integer> entry : needed.entrySet()) {
            int lacking = Math.min(CareerRules.PLAYSET, entry.getValue()) - store.owned(user, entry.getKey());
            if (lacking <= 0) {
                continue;
            }
            CareerCard card = service.resolveCard(printing.get(entry.getKey()));
            if (card == null) {
                card = new CareerCard(entry.getKey(), null, null, null);
            }
            out.put(card, lacking);
        }
        return out;
    }

    static LeagueCraft craftPlan(String user, DeckCardLists deck, CareerProfile profile) throws SQLException {
        return plan(shortfall(user, deck), profile);
    }

    static LeagueCraft planForTests(Map<CareerCard, Integer> lacking, CareerProfile profile) {
        return plan(lacking, profile);
    }

    private static LeagueCraft plan(Map<CareerCard, Integer> lacking, CareerProfile profile) {
        LeagueCraft plan = new LeagueCraft();
        int common = 0;
        int uncommon = 0;
        for (Map.Entry<CareerCard, Integer> entry : lacking.entrySet()) {
            CareerCard card = entry.getKey();
            int n = entry.getValue();
            if (card.rarity == null || CareerRules.LAND.equals(card.rarity)) {
                plan.unknown.add(card.name);
                continue;
            }
            plan.copies += n;
            switch (card.rarity) {
                case CareerRules.COMMON:
                    common += n;
                    break;
                case CareerRules.UNCOMMON:
                    uncommon += n;
                    break;
                case CareerRules.MYTHIC:
                    plan.mythic += n;
                    break;
                default:
                    plan.rare += n;
                    break;
            }
        }
        // commons and uncommons spend their wildcards first, then coins
        int haveCommon = profile == null ? 0 : CareerService.wildcardsOf(profile, CareerRules.COMMON);
        int haveUncommon = profile == null ? 0 : CareerService.wildcardsOf(profile, CareerRules.UNCOMMON);
        plan.common = Math.min(common, haveCommon);
        plan.uncommon = Math.min(uncommon, haveUncommon);
        plan.coins = (common - plan.common) * CareerRules.craftCoins(CareerRules.COMMON)
                + (uncommon - plan.uncommon) * CareerRules.craftCoins(CareerRules.UNCOMMON);
        plan.affordable = profile != null && plan.unknown.isEmpty()
                && plan.rare <= CareerService.wildcardsOf(profile, CareerRules.RARE)
                && plan.mythic <= CareerService.wildcardsOf(profile, CareerRules.MYTHIC)
                && plan.coins <= profile.coins;
        return plan;
    }

    /**
     * Crafts every card a League deck lacks, all at once or not at all: wildcards by rarity, and coins for commons
     * and uncommons once their wildcards run out.
     */
    public LeagueDeck craftMissing(String user, String id) throws SQLException, CareerService.CareerException {
        CareerStore store = CareerStore.get();
        CareerStore.LeagueDeckRow row = store.leagueDeck(user, id);
        if (row == null) {
            throw new CareerService.CareerException("There is no such League deck.");
        }
        DeckCardLists deck = gson.fromJson(row.deck, DeckCardLists.class);
        int[] crafted = new int[1];
        CareerService.refusing(() -> store.transaction(() -> {
            CareerProfile profile = store.profile(user);
            Map<CareerCard, Integer> lacking = shortfall(user, deck);
            LeagueCraft plan = plan(lacking, profile);
            if (plan.copies == 0 && plan.unknown.isEmpty()) {
                throw new SQLException(new CareerService.CareerException("You already own every card in " + row.name + "."));
            }
            if (!plan.unknown.isEmpty()) {
                throw new SQLException(new CareerService.CareerException("These can't be crafted: " + String.join(", ", plan.unknown) + "."));
            }
            if (!plan.affordable) {
                throw new SQLException(new CareerService.CareerException(shortOf(plan, profile)));
            }
            store.addWildcard(user, CareerRules.COMMON, -plan.common);
            store.addWildcard(user, CareerRules.UNCOMMON, -plan.uncommon);
            store.addWildcard(user, CareerRules.RARE, -plan.rare);
            store.addWildcard(user, CareerRules.MYTHIC, -plan.mythic);
            if (plan.coins > 0) {
                store.addCoins(user, -plan.coins);
            }
            for (Map.Entry<CareerCard, Integer> entry : lacking.entrySet()) {
                store.addCard(user, entry.getKey(), entry.getValue());
            }
            store.addStat(user, "cards_crafted", plan.copies);
            CareerProgress.get().achievementsIn(store, user, null, null, System.currentTimeMillis());
            crafted[0] = plan.copies;
            return null;
        }));
        logger.info("League: " + user + " crafted " + crafted[0] + " missing cards for " + row.name);
        return view(user, store.leagueDeck(user, id));
    }

    private static String shortOf(LeagueCraft plan, CareerProfile profile) {
        List<String> short_ = new ArrayList<>();
        int rare = profile == null ? 0 : CareerService.wildcardsOf(profile, CareerRules.RARE);
        int mythic = profile == null ? 0 : CareerService.wildcardsOf(profile, CareerRules.MYTHIC);
        int coins = profile == null ? 0 : profile.coins;
        if (plan.mythic > mythic) {
            short_.add(plan.mythic + " mythic wildcards (you have " + mythic + ")");
        }
        if (plan.rare > rare) {
            short_.add(plan.rare + " rare wildcards (you have " + rare + ")");
        }
        if (plan.coins > coins) {
            short_.add(plan.coins + " coins (you have " + coins + ")");
        }
        return "Crafting the missing cards takes " + String.join(", ", short_) + ".";
    }

    private static String clipName(String name, String fallback) {
        String clean = name == null ? "" : name.trim();
        if (clean.isEmpty()) {
            clean = fallback;
        }
        return clean.length() > MAX_NAME ? clean.substring(0, MAX_NAME) : clean;
    }

    static String name(String format) {
        return format == null || format.isEmpty() ? "" : Character.toUpperCase(format.charAt(0)) + format.substring(1).toLowerCase(Locale.ENGLISH);
    }

    // ---- what climbing pays

    /**
     * Pays a League game's climb into the Career: a pack token of the format per tier gained for the first time (the
     * ladder makes sure each tier pays once a season), for each rank reached the rank's cosmetics and a rare wildcard
     * (a mythic one at Gold; Bronze, reached by the first game, pays no wildcard), and an uncommon wildcard every
     * {@link #WINS_PER_WILDCARD} League wins.
     */
    public static RankedRewards.Paid pay(String user, String format, List<Integer> tiers, List<String> ranks, int leagueWins, boolean won, long now) throws SQLException {
        CareerStore store = CareerStore.get();
        RankedRewards.Paid paid = new RankedRewards.Paid();
        store.transaction(() -> {
            CareerService.ensureProfile(store, user, now);
            if (!tiers.isEmpty()) {
                store.addLeagueTokens(user, format, tiers.size());
                paid.packs += tiers.size();
            }
            for (String rank : ranks) {
                for (CareerCosmetics.CareerCatalogItem item : CareerCosmetics.get().items()) {
                    if (rank.equals(item.ladder) && store.addUnlock(user, item.kind, item.id, now)) {
                        paid.cosmetics.add(new CareerProgress.CareerCosmetic(item.kind, item.id));
                    }
                }
                if ("bronze".equals(rank)) {
                    continue;
                }
                String rarity = "gold".equals(rank) ? CareerRules.MYTHIC : CareerRules.RARE;
                store.addWildcard(user, rarity, 1);
                paid.wildcards.merge(rarity, 1, Integer::sum);
            }
            if (won && leagueWins > 0 && leagueWins % WINS_PER_WILDCARD == 0) {
                store.addWildcard(user, CareerRules.UNCOMMON, 1);
                paid.wildcards.merge(CareerRules.UNCOMMON, 1, Integer::sum);
            }
            return null;
        });
        return paid;
    }

    /** the tiers (step / pips: 0 is Bronze 4) a peak moving from `before` to `after` enters for the first time */
    public static List<Integer> tiersGained(int peakBefore, int peakAfter, boolean first) {
        List<Integer> out = new ArrayList<>();
        int from = first ? 0 : tierIndex(peakBefore) + 1;
        for (int t = Math.max(1, from); t <= tierIndex(peakAfter); t++) {
            out.add(t);
        }
        return out;
    }

    /** a step's tier, counted from 0 (Bronze 4) up; Mythic is the last */
    static int tierIndex(int step) {
        return Math.min(LadderRank.MYTHIC_STEP, Math.max(0, step)) / LadderRank.PIPS;
    }

    // ---- export

    /** the account's Leagues, tokens and decks, for its data export */
    public Map<String, Object> export(String user) throws SQLException {
        CareerStore store = CareerStore.get();
        Map<String, Object> out = new LinkedHashMap<>();
        Map<String, Object> formats = new LinkedHashMap<>();
        for (Map.Entry<String, Long> entry : store.leagues(user).entrySet()) {
            Map<String, Object> f = new LinkedHashMap<>();
            f.put("started", entry.getValue());
            f.put("tokens", store.leagueTokens(user, entry.getKey()));
            formats.put(entry.getKey(), f);
        }
        out.put("formats", formats);
        List<Map<String, Object>> decks = new ArrayList<>();
        for (CareerStore.LeagueDeckRow row : store.leagueDecks(user, null)) {
            Map<String, Object> d = new LinkedHashMap<>();
            d.put("id", row.id);
            d.put("name", row.name);
            d.put("format", row.format);
            d.put("updated", row.updated);
            d.put("deck", gson.fromJson(row.deck, Object.class));
            decks.add(d);
        }
        out.put("decks", decks);
        return out;
    }
}
