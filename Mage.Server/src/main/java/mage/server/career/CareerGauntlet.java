package mage.server.career;

import com.google.gson.Gson;
import mage.cards.decks.DeckCardInfo;
import mage.cards.decks.DeckCardLists;

import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Random;
import java.util.Set;
import java.util.UUID;

/**
 * Gauntlet runs, a roguelike evening (resources/career/gauntlet.json). A run starts from two of three packs (a
 * 40-card deck) and climbs a branching map (S022): fights, elites, shops, rests and events up to the last boss; one
 * loss ends it. Elites leave a wound and hand out relics, run-wide edges (S023). Beating the last boss opens the next
 * ascension level, up to ten, each adding a twist (S024). A run's seed fixes its map and every offer on it, so friends
 * can play the same run and compare (S025). The packs can be Jumpstart, commons only, singletons, a cube or a draft
 * (S026, S027), and two score modes skip the map: boss rush (the roster back to back) and endless (S028). The run's
 * deck is its own: the cards don't join the collection.
 */
public final class CareerGauntlet {

    public static final String KIND = "gauntlet";

    public static final String MODE_MAP = "map";
    public static final String MODE_BOSS_RUSH = "bossrush";
    public static final String MODE_ENDLESS = "endless";
    /** runs started before the map: eight bosses in a line */
    static final String MODE_CLASSIC = "classic";

    public static final int MAX_ASCENSION = 10;

    static final String FIGHT = "fight";
    static final String ELITE = "elite";
    static final String BOSS = "boss";
    static final String SHOP = "shop";
    static final String REST = "rest";
    static final String EVENT = "event";

    // ---- content

    static class Rules {
        int entryCoins;
        int startPacks;
        int pickStart;
        int bundleSize;
        int bundles;
        int maxWins;
        List<Payout> rewards;
        List<CareerCampaigns.NodeOpponent> bosses;
        MapRules map;
        List<Relic> relics = new ArrayList<>();
        List<Event> events = new ArrayList<>();
        List<Ascension> ascension = new ArrayList<>();
        Endless endless;
        List<CareerRunChoice> variants = new ArrayList<>();
        List<CareerRunChoice> modes = new ArrayList<>();
    }

    static class Payout {
        int wins;
        int coins;
        int packs;
        String wildcard;
    }

    static class MapRules {
        int rows;
        int minNodes;
        int maxNodes;
        int fightCoins;
        int eliteCoins;
        int bundlePrice;
        int relicPrice;
        int healPrice;
        int woundLife;
        CareerSetup.CareerTwists eliteTwists;
    }

    /** a run-wide edge: twists for the player or every opponent, or a rule of the run ("purse", "discount", "bandages") */
    public static class CareerRelic {
        public String id;
        public String name;
        public String text;
        public String run;
        public CareerSetup.CareerTwists you;
        public CareerSetup.CareerTwists opponent;
    }

    static class Relic extends CareerRelic {
    }

    static class Event {
        String id;
        String name;
        String text;
        List<EventOption> options = new ArrayList<>();
    }

    static class EventOption {
        String id;
        String label;
        int coins;
        boolean relic;
        boolean bundle;
        boolean gamble;
        int wound;
        int heal;
    }

    static class Ascension {
        int level;
        String title;
        String text;
        /** "boss", "elite", "all" (every opponent), "you" or "run" */
        String target;
        String run;
        CareerSetup.CareerTwists twists;
        int skill;
    }

    static class Endless {
        int lifeStep;
        int cardEvery;
        int skillStep;
        int maxSkill;
    }

    /** a kind of run to pick when starting one: a mode or a variant */
    public static class CareerRunChoice {
        public String id;
        public String name;
        public String text;
    }

    // ---- run state, kept as JSON in the run's row

    public static class CareerRunCard {
        public String name;
        public String setCode;
        public String cardNumber;
        public int amount;

        CareerRunCard() {
        }

        CareerRunCard(DeckCardInfo info, int amount) {
            this.name = info.getCardName();
            this.setCode = info.getSetCode();
            this.cardNumber = info.getCardNumber();
            this.amount = amount;
        }
    }

    public static class CareerOfferOption {
        public String id;
        public String name;
        public String color;
        public List<CareerRunCard> cards = new ArrayList<>();
        /** run coins it costs, in a shop */
        public Integer price;
        /** what it is: "pack", "bundle", "relic", "heal", "train", "leave", "choice" or "card" */
        public String kind;
        /** a relic's or an event choice's explanation */
        public String text;
    }

    public static class CareerOffer {
        /** "start", "bundle", "relic", "shop", "rest", "event" or "draft" */
        public String kind;
        public int pick;
        public List<CareerOfferOption> options = new ArrayList<>();
        /** an event's or a shop's name, and what it says */
        public String title;
        public String text;
        /** the map node it belongs to */
        public String node;
        /** an event's id */
        public String event;
        /** while drafting: the round and the pick */
        public Integer round;
        public Integer draftPick;
    }

    static class Node {
        String id;
        int row;
        String type;
        /** for a fight: which of the gauntlet's bosses */
        int opponent;
        List<String> links = new ArrayList<>();
    }

    static class RunData {
        String colors = "";
        List<CareerRunCard> deck = new ArrayList<>();
        CareerOffer offer;
        // the map and its modes (absent in runs started before them)
        String mode;
        String variant;
        long seed;
        int ascension;
        int runCoins;
        int wounds;
        List<String> relics = new ArrayList<>();
        List<List<Node>> map;
        /** the node the player stands on */
        String at;
        List<String> cleared = new ArrayList<>();
        String cubeId;
        /** a deck sent as the cube: its card names */
        List<String> cubeCards;
        String setCode;
        CareerLimited.Draft draft;
    }

    // ---- views

    public static class CareerRun {
        public String id;
        /** "active", "won", "lost" or "abandoned" */
        public String state;
        public int wins;
        public int losses;
        public int maxWins;
        public String colors;
        public List<CareerRunCard> deck;
        public int deckSize;
        /** a pick waiting before the next game */
        public CareerOffer offer;
        /** the next opponent, while the run is on and a fight is waiting */
        public CareerRunBoss next;
        /** the classic run's eight bosses, or boss rush's and endless's line-up so far */
        public List<CareerRunBoss> bosses = new ArrayList<>();
        public long started;
        public Long ended;
        /** "map", "bossrush", "endless" or "classic" */
        public String mode;
        public String variant;
        /** the run's seed, to share: the same seed plays the same map and offers */
        public String seed;
        public int ascension;
        public String ascensionTitle;
        public int runCoins;
        public int wounds;
        /** life each wound costs at the start of a fight */
        public int woundLife;
        public List<CareerRelic> relics = new ArrayList<>();
        /** the map, row by row */
        public List<List<CareerRunNode>> map;
        /** the node the player stands on */
        public String at;
        /** where the player may go next */
        public List<String> choices = new ArrayList<>();
    }

    public static class CareerRunNode {
        public String id;
        public int row;
        /** "fight", "elite", "boss", "shop", "rest" or "event" */
        public String type;
        public List<String> links;
        /** "cleared", "here", "open" (a choice now), "ahead" or "missed" */
        public String state;
        /** for a fight */
        public CareerRunBoss opponent;
    }

    public static class CareerRunBoss {
        public int number;
        public String name;
        /** the card whose art is the boss's portrait */
        public CareerContent.CareerCover cover;
        public int skill;
        public CareerSetup.CareerTwists twists;
        public boolean beaten;
        /** "fight", "elite" or "boss" */
        public String type;
    }

    public static class CareerGauntletState {
        public int entryCoins;
        public int maxWins;
        public List<CareerRunReward> rewards = new ArrayList<>();
        /** the run in progress, or null */
        public CareerRun run;
        /** the best finished run */
        public int bestWins;
        public List<CareerRun> recent = new ArrayList<>();
        /** the highest ascension level open to start (0 until the map's last boss falls) */
        public int ascensionOpen;
        public List<CareerAscension> ascension = new ArrayList<>();
        public List<CareerRunChoice> modes = new ArrayList<>();
        public List<CareerRunChoice> variants = new ArrayList<>();
        public List<CareerRelic> relics = new ArrayList<>();
    }

    public static class CareerAscension {
        public int level;
        public String title;
        public String text;
    }

    public static class CareerRunReward {
        public int wins;
        public int coins;
        public int packs;
        public String wildcard;
    }

    /** How a new run is set up; every field is optional. */
    public static class StartOptions {
        public String mode;
        public String variant;
        public int ascension;
        /** a shared seed to play */
        public String seed;
        /** the cube for the cube variant: one of the server's draft cubes */
        public String cubeId;
        /** or a deck's card names sent as the cube */
        public List<String> cubeCards;
        /** the set to draft for the draft variant */
        public String setCode;
    }

    private static volatile CareerGauntlet instance;

    private final Rules rules;
    private final Gson gson = new Gson();
    private Random random = new Random();

    private CareerGauntlet() throws IOException {
        InputStream stream = CareerGauntlet.class.getResourceAsStream("/career/gauntlet.json");
        if (stream == null) {
            throw new IOException("Missing career resource gauntlet.json");
        }
        try (Reader reader = new InputStreamReader(stream, StandardCharsets.UTF_8)) {
            rules = gson.fromJson(reader, Rules.class);
        }
    }

    public static CareerGauntlet get() {
        CareerGauntlet current = instance;
        if (current == null) {
            synchronized (CareerGauntlet.class) {
                current = instance;
                if (current == null) {
                    try {
                        current = new CareerGauntlet();
                    } catch (IOException | RuntimeException e) {
                        throw new IllegalStateException("The gauntlet can't be read: " + e.getMessage(), e);
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    void useRandom(Random random) {
        this.random = random;
    }

    List<CareerCampaigns.NodeOpponent> bosses() {
        return Collections.unmodifiableList(rules.bosses);
    }

    Rules rules() {
        return rules;
    }

    // ---- seeds

    private static final String SEED_LETTERS = "23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

    /** a seed as eight letters and digits, like "K7Q2-9F3M" */
    static String seedCode(long seed) {
        StringBuilder code = new StringBuilder();
        long value = seed & 0xFFFFFFFFFFL; // 40 bits, eight letters of five
        for (int i = 0; i < 8; i++) {
            code.append(SEED_LETTERS.charAt((int) (value % 32)));
            value /= 32;
            if (i == 3) {
                code.append('-');
            }
        }
        return code.toString();
    }

    /** the seed a code stands for; null when it isn't one */
    static Long parseSeed(String code) {
        if (code == null) {
            return null;
        }
        String letters = code.trim().toUpperCase(Locale.ENGLISH).replace("-", "").replace(" ", "");
        if (letters.length() != 8) {
            return null;
        }
        long value = 0;
        for (int i = 7; i >= 0; i--) {
            int digit = SEED_LETTERS.indexOf(letters.charAt(i));
            if (digit < 0) {
                return null;
            }
            value = value * 32 + digit;
        }
        return value;
    }

    /** the randomness for one thing in a run: the same seed and purpose always give the same draws */
    static Random rng(RunData data, String purpose) {
        return new Random(data.seed * 1_000_003L + purpose.hashCode());
    }

    // ---- reading

    public CareerGauntletState state(String user) throws SQLException {
        CareerStore store = CareerStore.get();
        CareerGauntletState state = new CareerGauntletState();
        state.entryCoins = rules.entryCoins;
        state.maxWins = rules.maxWins;
        for (Payout payout : rules.rewards) {
            CareerRunReward view = new CareerRunReward();
            view.wins = payout.wins;
            view.coins = payout.coins;
            view.packs = payout.packs;
            view.wildcard = payout.wildcard;
            state.rewards.add(view);
        }
        for (CareerStore.RunRow row : store.runs(user, KIND, false, 10)) {
            CareerRun run = view(row);
            if ("active".equals(row.state)) {
                state.run = run;
            } else {
                state.recent.add(run);
                state.bestWins = Math.max(state.bestWins, row.wins);
            }
        }
        state.ascensionOpen = ascensionOpen(store, user);
        for (Ascension level : rules.ascension) {
            CareerAscension view = new CareerAscension();
            view.level = level.level;
            view.title = level.title;
            view.text = level.text;
            state.ascension.add(view);
        }
        state.modes.addAll(rules.modes);
        state.variants.addAll(rules.variants);
        state.relics.addAll(rules.relics);
        return state;
    }

    /** the highest level a player may start: one past the highest cleared, up to ten */
    static int ascensionOpen(CareerStore store, String user) throws SQLException {
        // the stat is the highest level cleared plus one, so a clear at level 0 counts
        Integer cleared = store.stats(user).get("gauntlet_ascension_cleared");
        return cleared == null ? 0 : Math.min(MAX_ASCENSION, cleared);
    }

    private static String modeOf(RunData data) {
        return data.mode == null ? MODE_CLASSIC : data.mode;
    }

    private CareerRun view(CareerStore.RunRow row) {
        RunData data = gson.fromJson(row.data, RunData.class);
        CareerRun run = new CareerRun();
        run.id = row.id;
        run.state = row.state;
        run.wins = row.wins;
        run.losses = row.losses;
        run.maxWins = rules.maxWins;
        run.colors = data.colors;
        run.deck = data.deck;
        run.deckSize = data.deck.stream().mapToInt(card -> card.amount).sum();
        run.offer = data.offer;
        run.started = row.started;
        run.ended = row.ended;
        run.mode = modeOf(data);
        run.variant = data.variant == null ? GauntletPacks.JUMPSTART : data.variant;
        run.seed = data.mode == null ? null : seedCode(data.seed);
        run.ascension = data.ascension;
        run.ascensionTitle = data.ascension > 0 ? ascension(data.ascension).title : null;
        run.runCoins = data.runCoins;
        run.wounds = data.wounds;
        run.woundLife = woundLife(data);
        for (String id : data.relics == null ? Collections.<String>emptyList() : data.relics) {
            CareerRelic relic = relic(id);
            if (relic != null) {
                run.relics.add(relic);
            }
        }
        boolean active = "active".equals(row.state);
        switch (run.mode) {
            case MODE_CLASSIC:
                for (int i = 0; i < rules.bosses.size(); i++) {
                    CareerRunBoss bossView = bossView(rules.bosses.get(i), i + 1, null, i < row.wins, BOSS);
                    run.bosses.add(bossView);
                }
                if (active && row.wins < run.bosses.size()) {
                    run.next = run.bosses.get(row.wins);
                }
                break;
            case MODE_MAP:
                run.at = data.at;
                run.map = new ArrayList<>();
                Set<String> open = new HashSet<>(choices(data));
                for (List<Node> nodes : data.map) {
                    List<CareerRunNode> rowView = new ArrayList<>();
                    for (Node node : nodes) {
                        CareerRunNode nodeView = new CareerRunNode();
                        nodeView.id = node.id;
                        nodeView.row = node.row;
                        nodeView.type = node.type;
                        nodeView.links = node.links;
                        nodeView.state = data.cleared.contains(node.id) ? "cleared" : node.id.equals(data.at) ? "here"
                                : open.contains(node.id) ? "open" : node.row <= currentRow(data) ? "missed" : "ahead";
                        if (isFight(node)) {
                            Opponent opponent = mapOpponent(data, node);
                            nodeView.opponent = bossView(opponent.boss, node.row + 1, opponent.twists, data.cleared.contains(node.id), node.type);
                            nodeView.opponent.skill = opponent.skill;
                        }
                        rowView.add(nodeView);
                    }
                    run.map.add(rowView);
                }
                if (active) {
                    run.choices.addAll(choices(data));
                    Node here = node(data, data.at);
                    if (here != null && isFight(here) && !data.cleared.contains(here.id) && data.offer == null) {
                        Opponent opponent = mapOpponent(data, here);
                        run.next = bossView(opponent.boss, here.row + 1, opponent.twists, false, here.type);
                        run.next.skill = opponent.skill;
                    }
                }
                break;
            default:
                // boss rush and endless: the opponents beaten, then the next one
                for (int i = 0; i <= row.wins && (active || i < row.wins); i++) {
                    if (MODE_BOSS_RUSH.equals(run.mode) && i >= rosterOrder().size()) {
                        break;
                    }
                    Opponent opponent = lineOpponent(data, i);
                    CareerRunBoss view = bossView(opponent.boss, i + 1, opponent.twists, i < row.wins, i == rosterOrder().size() - 1 ? BOSS : FIGHT);
                    view.skill = opponent.skill;
                    run.bosses.add(view);
                }
                if (active && row.wins < run.bosses.size() && data.offer == null) {
                    run.next = run.bosses.get(row.wins);
                }
        }
        return run;
    }

    private CareerRunBoss bossView(CareerCampaigns.NodeOpponent boss, int number, CareerSetup.CareerTwists twists, boolean beaten, String type) {
        CareerRunBoss view = new CareerRunBoss();
        view.number = number;
        view.name = boss.name;
        view.cover = boss.deck.startsWith("roster:")
                ? CareerContent.get().cover(CareerContent.get().opponent(boss.deck.substring("roster:".length())))
                : CareerContent.coverOf("campaigns/decks/" + boss.deck);
        view.skill = boss.skill;
        view.twists = twists == null ? boss.twists : twists;
        view.beaten = beaten;
        view.type = type;
        return view;
    }

    // ---- the map

    static boolean isFight(Node node) {
        return FIGHT.equals(node.type) || ELITE.equals(node.type) || BOSS.equals(node.type);
    }

    static Node node(RunData data, String id) {
        if (id == null || data.map == null) {
            return null;
        }
        for (List<Node> nodes : data.map) {
            for (Node node : nodes) {
                if (node.id.equals(id)) {
                    return node;
                }
            }
        }
        return null;
    }

    private static int currentRow(RunData data) {
        Node here = node(data, data.at);
        return here == null ? -1 : here.row;
    }

    /** the nodes the player may move to now: the first row to begin with, then the links of a cleared node */
    static List<String> choices(RunData data) {
        List<String> choices = new ArrayList<>();
        if (data.map == null || data.offer != null) {
            return choices;
        }
        if (data.at == null) {
            for (Node node : data.map.get(0)) {
                choices.add(node.id);
            }
            return choices;
        }
        Node here = node(data, data.at);
        if (here != null && data.cleared.contains(here.id)) {
            choices.addAll(here.links);
        }
        return choices;
    }

    /**
     * A map from a seed: {@code rows} rows of two or three nodes and the last boss alone at the top. The first row is
     * fights; elites and shops come from the third row, rests from the fourth, and the row before the boss always has
     * a rest. Each node links to one or two nodes of the next row, and every node can be reached.
     */
    static List<List<Node>> generateMap(MapRules map, int bossCount, Random random) {
        List<List<Node>> rows = new ArrayList<>();
        int last = map.rows - 1;
        for (int r = 0; r < map.rows; r++) {
            int count = r == last ? 1 : map.minNodes + random.nextInt(map.maxNodes - map.minNodes + 1);
            List<Node> nodes = new ArrayList<>();
            boolean restPlaced = false;
            for (int i = 0; i < count; i++) {
                Node node = new Node();
                node.id = "r" + r + "n" + i;
                node.row = r;
                if (r == last) {
                    node.type = BOSS;
                } else if (r == 0) {
                    node.type = FIGHT;
                } else if (r == last - 1 && !restPlaced) {
                    node.type = REST;
                    restPlaced = true;
                } else {
                    node.type = rollType(r, random);
                }
                if (isFight(node)) {
                    int top = bossCount - 2;
                    int base = Math.round((float) r * top / Math.max(1, last - 1));
                    int jitter = random.nextInt(3) - 1;
                    node.opponent = r == last ? bossCount - 1 : Math.max(0, Math.min(top, base + jitter + (ELITE.equals(node.type) ? 1 : 0)));
                }
                nodes.add(node);
            }
            // the row before the boss: put the rest at a random place
            if (r == last - 1 && nodes.size() > 1) {
                Collections.shuffle(nodes, random);
                for (int i = 0; i < nodes.size(); i++) {
                    nodes.get(i).id = "r" + r + "n" + i;
                }
            }
            rows.add(nodes);
        }
        for (int r = 0; r < last; r++) {
            List<Node> from = rows.get(r);
            List<Node> to = rows.get(r + 1);
            int a = from.size();
            int b = to.size();
            for (int i = 0; i < a; i++) {
                int j = a == 1 ? 0 : Math.round((float) i * (b - 1) / (a - 1));
                link(from.get(i), to.get(j));
                if (b > 1 && random.nextInt(2) == 0) {
                    int k = j + 1 < b ? j + 1 : j - 1;
                    link(from.get(i), to.get(k));
                }
            }
            for (int j = 0; j < b; j++) {
                String id = to.get(j).id;
                boolean reached = from.stream().anyMatch(node -> node.links.contains(id));
                if (!reached) {
                    int i = b == 1 ? 0 : Math.min(a - 1, Math.round((float) j * (a - 1) / (b - 1)));
                    link(from.get(i), to.get(j));
                }
            }
            for (Node node : from) {
                node.links.sort(Comparator.naturalOrder());
            }
        }
        return rows;
    }

    private static void link(Node from, Node to) {
        if (!from.links.contains(to.id)) {
            from.links.add(to.id);
        }
    }

    private static String rollType(int row, Random random) {
        int roll = random.nextInt(100);
        if (row >= 2 && roll < 15) {
            return ELITE;
        }
        if (roll < 55) {
            return FIGHT;
        }
        if (roll < 75) {
            return EVENT;
        }
        if (row >= 2 && roll < 88) {
            return SHOP;
        }
        if (row >= 3) {
            return REST;
        }
        return roll < 88 ? EVENT : FIGHT;
    }

    // ---- opponents

    /** who a fight is against: the boss, its twists for this fight and the skill it plays at */
    static class Opponent {
        CareerCampaigns.NodeOpponent boss;
        CareerSetup.CareerTwists twists;
        int skill;
    }

    Opponent mapOpponent(RunData data, Node node) {
        Opponent opponent = new Opponent();
        opponent.boss = rules.bosses.get(Math.min(node.opponent, rules.bosses.size() - 1));
        CareerSetup.CareerTwists twists = opponent.boss.twists;
        int skill = opponent.boss.skill;
        if (ELITE.equals(node.type)) {
            twists = CareerSetup.combine(twists, rules.map.eliteTwists);
            skill++;
        }
        for (Ascension level : levels(data)) {
            boolean applies = "all".equals(level.target) || (BOSS.equals(level.target) && BOSS.equals(node.type))
                    || (ELITE.equals(level.target) && ELITE.equals(node.type));
            if (applies) {
                twists = CareerSetup.combine(twists, level.twists);
                skill += level.skill;
            }
        }
        for (CareerRelic relic : relicsOf(data)) {
            twists = CareerSetup.combine(twists, relic.opponent);
        }
        opponent.twists = twists;
        opponent.skill = Math.max(1, Math.min(10, skill));
        return opponent;
    }

    /** the roster in order of difficulty: by tier, then as the calibration measured it (S039) */
    static List<CareerContent.Opponent> rosterOrder() {
        List<CareerContent.Opponent> roster = new ArrayList<>();
        for (CareerContent.Opponent opponent : CareerContent.get().opponents()) {
            // the rival's arc and raid bosses stay on the ladder
            if (opponent.rival == null && !opponent.id.startsWith("rival") && !opponent.raid) {
                roster.add(opponent);
            }
        }
        roster.sort(Comparator.comparingInt((CareerContent.Opponent opponent) -> opponent.tier)
                .thenComparingInt(opponent -> -CareerContent.measuredWinRate(opponent)));
        return roster;
    }

    /** boss rush's and endless's opponent number {@code index} (from 0) */
    Opponent lineOpponent(RunData data, int index) {
        List<CareerContent.Opponent> roster = rosterOrder();
        CareerContent.Opponent pick = roster.get(index % roster.size());
        int loop = index / roster.size();
        Opponent opponent = new Opponent();
        CareerCampaigns.NodeOpponent boss = new CareerCampaigns.NodeOpponent();
        boss.name = pick.name;
        boss.deck = "roster:" + pick.id;
        boss.aiType = pick.aiType;
        boss.skill = pick.skill != null ? pick.skill : CareerContent.get().tier(pick.tier).skill;
        opponent.boss = boss;
        int skill = boss.skill;
        CareerSetup.CareerTwists twists = null;
        if (MODE_ENDLESS.equals(data.mode) && loop > 0) {
            Endless endless = rules.endless;
            twists = new CareerSetup.CareerTwists();
            twists.lifeBonus = endless.lifeStep * loop;
            twists.extraCards = endless.cardEvery <= 0 ? 0 : loop / endless.cardEvery;
            skill = Math.min(endless.maxSkill, skill + endless.skillStep * loop);
        }
        for (Ascension level : levels(data)) {
            if ("all".equals(level.target)) {
                twists = CareerSetup.combine(twists, level.twists);
                skill += level.skill;
            }
        }
        for (CareerRelic relic : relicsOf(data)) {
            twists = CareerSetup.combine(twists, relic.opponent);
        }
        opponent.twists = twists;
        opponent.skill = Math.max(1, Math.min(10, skill));
        return opponent;
    }

    /** the player's own edges in a fight: relics, wounds and ascension */
    CareerSetup.CareerTwists playerTwists(RunData data) {
        CareerSetup.CareerTwists twists = null;
        for (CareerRelic relic : relicsOf(data)) {
            twists = CareerSetup.combine(twists, relic.you);
        }
        for (Ascension level : levels(data)) {
            if ("you".equals(level.target)) {
                twists = CareerSetup.combine(twists, level.twists);
            }
        }
        if (data.wounds > 0) {
            CareerSetup.CareerTwists wounds = new CareerSetup.CareerTwists();
            wounds.lifeBonus = -data.wounds * woundLife(data);
            twists = CareerSetup.combine(twists, wounds);
        }
        return twists;
    }

    private List<Ascension> levels(RunData data) {
        List<Ascension> levels = new ArrayList<>();
        for (Ascension level : rules.ascension) {
            if (level.level <= data.ascension) {
                levels.add(level);
            }
        }
        return levels;
    }

    private boolean runRule(RunData data, String rule) {
        for (Ascension level : levels(data)) {
            if (rule.equals(level.run)) {
                return true;
            }
        }
        for (CareerRelic relic : relicsOf(data)) {
            if (rule.equals(relic.run)) {
                return true;
            }
        }
        return false;
    }

    private int woundLife(RunData data) {
        int life = rules.map == null ? 2 : rules.map.woundLife;
        return runRule(data, "deepWounds") ? life + 1 : life;
    }

    private int price(RunData data, int base) {
        int price = base;
        if (runRule(data, "discount")) {
            price = price * 3 / 4;
        }
        if (runRule(data, "markup")) {
            price = price * 5 / 4;
        }
        return price;
    }

    private Ascension ascension(int level) {
        return rules.ascension.stream().filter(candidate -> candidate.level == level).findFirst().orElse(rules.ascension.get(0));
    }

    CareerRelic relic(String id) {
        return rules.relics.stream().filter(relic -> relic.id.equals(id)).findFirst().orElse(null);
    }

    private List<CareerRelic> relicsOf(RunData data) {
        List<CareerRelic> relics = new ArrayList<>();
        if (data.relics != null) {
            for (String id : data.relics) {
                CareerRelic relic = relic(id);
                if (relic != null) {
                    relics.add(relic);
                }
            }
        }
        return relics;
    }

    // ---- starting, picking, moving, giving up

    public CareerRun start(String user) throws SQLException, CareerService.CareerException {
        return start(user, new StartOptions());
    }

    public CareerRun start(String user, StartOptions options) throws SQLException, CareerService.CareerException {
        String mode = options.mode == null ? MODE_MAP : options.mode;
        if (!MODE_MAP.equals(mode) && !MODE_BOSS_RUSH.equals(mode) && !MODE_ENDLESS.equals(mode)) {
            throw new CareerService.CareerException("A run is the map, a boss rush or endless.");
        }
        String variant = options.variant == null ? GauntletPacks.JUMPSTART : options.variant;
        if (!GauntletPacks.known(variant)) {
            throw new CareerService.CareerException("There's no " + variant + " gauntlet.");
        }
        if (GauntletPacks.CUBE.equals(variant) && (options.cubeId == null || options.cubeId.isEmpty())
                && (options.cubeCards == null || options.cubeCards.isEmpty())) {
            throw new CareerService.CareerException("Choose a cube to cut the packs from.");
        }
        if (options.cubeCards != null && options.cubeCards.size() > 1000) {
            throw new CareerService.CareerException("A cube of more than a thousand cards is too big.");
        }
        Long shared = null;
        if (options.seed != null && !options.seed.trim().isEmpty()) {
            shared = parseSeed(options.seed);
            if (shared == null) {
                throw new CareerService.CareerException("That isn't a run seed. A seed looks like K7Q2-9F3M.");
            }
        }
        String setCode = options.setCode;
        if (GauntletPacks.DRAFT.equals(variant)) {
            List<CareerService.CareerShopSet> sets;
            try {
                sets = CareerService.get().shop(user);
            } catch (SQLException e) {
                throw e;
            }
            String wanted = setCode;
            CareerService.CareerShopSet chosen = sets.stream().filter(set -> !set.locked && (wanted == null || set.setCode.equals(wanted))).findFirst().orElse(null);
            if (chosen == null) {
                throw new CareerService.CareerException("The shop doesn't sell " + (wanted == null ? "any" : wanted) + " packs to draft.");
            }
            setCode = chosen.setCode;
        }
        CareerStore store = CareerStore.get();
        long seed = shared != null ? shared : (random.nextLong() & 0xFFFFFFFFFFL);
        String draftSet = setCode;
        return refusing(() -> store.transaction(() -> {
            CareerProfile profile = store.profile(user);
            if (profile == null) {
                throw refusal("Open a Career first.");
            }
            if (!store.runs(user, KIND, true, 1).isEmpty()) {
                throw refusal("Finish or give up your gauntlet run first.");
            }
            int open = ascensionOpen(store, user);
            if (options.ascension < 0 || options.ascension > open) {
                throw refusal(open == 0 ? "Beat the last boss of the map to open ascension." : "Ascension " + open + " is the highest open to you.");
            }
            if (options.ascension > 0 && !MODE_MAP.equals(mode)) {
                throw refusal("Ascension is for the map.");
            }
            if (profile.coins < rules.entryCoins) {
                throw refusal("A gauntlet run costs " + rules.entryCoins + " coins; you have " + profile.coins + ".");
            }
            store.addCoins(user, -rules.entryCoins);
            RunData data = new RunData();
            data.mode = mode;
            data.variant = variant;
            data.seed = seed;
            data.ascension = options.ascension;
            data.cubeId = options.cubeId;
            data.cubeCards = options.cubeCards;
            data.setCode = draftSet;
            if (MODE_MAP.equals(mode)) {
                data.map = generateMap(rules.map, rules.bosses.size(), rng(data, "map"));
            }
            if (GauntletPacks.DRAFT.equals(variant)) {
                data.draft = CareerLimited.get().newDraft(draftSet);
                data.offer = draftOffer(data);
            } else {
                try {
                    data.offer = startOffer(data);
                } catch (CareerService.CareerException e) {
                    throw new SQLException(e);
                }
            }
            CareerStore.RunRow row = new CareerStore.RunRow();
            row.id = UUID.randomUUID().toString();
            row.kind = KIND;
            row.started = System.currentTimeMillis();
            row.state = "active";
            row.data = gson.toJson(data);
            store.putRun(user, row);
            store.addStat(user, "gauntlet_runs", 1);
            return view(row);
        }));
    }

    private CareerOffer startOffer(RunData data) throws CareerService.CareerException {
        CareerOffer offer = new CareerOffer();
        offer.kind = "start";
        offer.pick = rules.pickStart;
        for (CareerJumpstart.Pack pack : GauntletPacks.start(data.variant, data.cubeId, data.cubeCards, rng(data, "start"), rules.startPacks)) {
            CareerOfferOption option = new CareerOfferOption();
            option.id = pack.name;
            option.name = pack.name;
            option.color = pack.color;
            option.kind = "pack";
            for (DeckCardInfo card : pack.cards) {
                option.cards.add(new CareerRunCard(card, card.getAmount()));
            }
            offer.options.add(option);
        }
        return offer;
    }

    private static CareerOffer draftOffer(RunData data) {
        CareerOffer offer = new CareerOffer();
        offer.kind = "draft";
        offer.pick = 1;
        offer.round = data.draft.round;
        offer.draftPick = data.draft.pick;
        offer.title = "Draft";
        offer.text = "Pick a card; seven bots pick theirs and the packs pass. Three packs, then the coach builds your deck.";
        List<CareerLimited.CareerPoolCard> pack = data.draft.packs.get(0);
        for (int i = 0; i < pack.size(); i++) {
            CareerLimited.CareerPoolCard card = pack.get(i);
            CareerOfferOption option = new CareerOfferOption();
            option.id = String.valueOf(i);
            option.name = card.name;
            option.color = card.colors;
            option.kind = "card";
            CareerRunCard runCard = new CareerRunCard();
            runCard.name = card.name;
            runCard.setCode = card.setCode;
            runCard.cardNumber = card.cardNumber;
            runCard.amount = 1;
            option.cards.add(runCard);
            offer.options.add(option);
        }
        return offer;
    }

    /** after the draft: the coach's two best colour pairs, built to 40 cards, to pick one */
    private static CareerOffer builtOffer(List<CareerLimited.CareerPoolCard> pool) {
        CareerOffer offer = new CareerOffer();
        offer.kind = "start";
        offer.pick = 1;
        offer.title = "Your draft, built";
        offer.text = "The coach built the two best colour pairs of your picks. Take one to the map.";
        for (CareerLimited.PairAdvice pair : CareerLimited.coach(pool).pairs.subList(0, 2)) {
            CareerOfferOption option = new CareerOfferOption();
            option.id = pair.colors;
            option.name = pair.colors + " (" + String.join(" ", pair.reasons.subList(0, Math.min(2, pair.reasons.size()))) + ")";
            option.color = pair.colors;
            option.kind = "pack";
            option.text = String.join(" ", pair.reasons);
            for (DeckCardInfo card : pair.deck) {
                DeckCardInfo printed = card.getSetCode() == null ? CareerLimited.withPrinting(card) : card;
                option.cards.add(new CareerRunCard(printed, card.getAmount()));
            }
            offer.options.add(option);
        }
        return offer;
    }

    /** takes the waiting offer: starting packs (as many as it asks for), a bundle, a relic, a shop item or a choice */
    public CareerRun pick(String user, List<String> optionIds) throws SQLException, CareerService.CareerException {
        CareerStore store = CareerStore.get();
        return refusing(() -> store.transaction(() -> {
            CareerStore.RunRow row = active(store, user);
            RunData data = gson.fromJson(row.data, RunData.class);
            CareerOffer offer = data.offer;
            if (offer == null) {
                throw refusal("There's nothing to pick right now.");
            }
            if (optionIds == null || optionIds.size() != offer.pick
                    || optionIds.stream().distinct().count() != optionIds.size()) {
                throw refusal("Pick " + offer.pick + (offer.pick == 1 ? " option." : " options."));
            }
            List<CareerOfferOption> chosen = new ArrayList<>();
            for (String id : optionIds) {
                CareerOfferOption option = offer.options.stream().filter(candidate -> candidate.id.equals(id)).findFirst().orElse(null);
                if (option == null) {
                    throw refusal("That isn't one of the options.");
                }
                chosen.add(option);
            }
            switch (offer.kind) {
                case "draft":
                    pickDraft(data, Integer.parseInt(chosen.get(0).id));
                    break;
                case "shop":
                    buy(data, offer, chosen.get(0));
                    break;
                case "rest":
                    rest(data, offer, chosen.get(0));
                    break;
                case "event":
                    event(data, offer, chosen.get(0));
                    break;
                case "relic":
                    data.relics.add(chosen.get(0).id);
                    resolved(data, offer);
                    break;
                default:
                    // start packs and bundles go in the deck
                    for (CareerOfferOption option : chosen) {
                        addCards(data, option);
                    }
                    resolved(data, offer);
            }
            row.data = gson.toJson(data);
            store.putRun(user, row);
            return view(row);
        }));
    }

    private void pickDraft(RunData data, int index) throws SQLException {
        List<CareerLimited.CareerPoolCard> pack = data.draft.packs.get(0);
        if (index < 0 || index >= pack.size()) {
            throw refusal("That card isn't in the pack.");
        }
        if (CareerLimited.get().draftStep(data.draft, index, data.setCode)) {
            List<CareerLimited.CareerPoolCard> pool = data.draft.pools.get(0);
            data.draft = null;
            data.offer = builtOffer(pool);
        } else {
            data.offer = draftOffer(data);
        }
    }

    private static void addCards(RunData data, CareerOfferOption option) {
        Map<String, CareerRunCard> deck = new LinkedHashMap<>();
        for (CareerRunCard card : data.deck) {
            deck.put(card.setCode + "|" + card.cardNumber + "|" + card.name, card);
        }
        for (CareerRunCard card : option.cards) {
            deck.merge(card.setCode + "|" + card.cardNumber + "|" + card.name, copy(card), (a, b) -> {
                a.amount += b.amount;
                return a;
            });
        }
        if (option.color != null) {
            for (char color : option.color.toCharArray()) {
                if (data.colors.indexOf(color) < 0) {
                    data.colors += color;
                }
            }
        }
        data.deck = new ArrayList<>(deck.values());
    }

    /** an offer is done: a node that isn't a fight is cleared once its offers are */
    private static void resolved(RunData data, CareerOffer offer) {
        data.offer = null;
        Node here = node(data, data.at);
        if (here != null && !isFight(here) && !data.cleared.contains(here.id)) {
            data.cleared.add(here.id);
        }
    }

    private void buy(RunData data, CareerOffer offer, CareerOfferOption option) throws SQLException {
        if ("leave".equals(option.kind)) {
            resolved(data, offer);
            return;
        }
        int price = option.price == null ? 0 : option.price;
        if (data.runCoins < price) {
            throw refusal("That costs " + price + " run coins; you have " + data.runCoins + ".");
        }
        data.runCoins -= price;
        switch (option.kind) {
            case "relic":
                data.relics.add(option.id.substring("relic:".length()));
                break;
            case "heal":
                data.wounds = Math.max(0, data.wounds - 1);
                break;
            default:
                addCards(data, option);
        }
        offer.options.remove(option);
        if ("heal".equals(option.kind) && data.wounds > 0) {
            offer.options.add(offer.options.size() - 1, option); // another wound can be healed
        }
    }

    private void rest(RunData data, CareerOffer offer, CareerOfferOption option) {
        if ("heal".equals(option.kind)) {
            data.wounds = 0;
            resolved(data, offer);
        } else {
            data.offer = bundles(data, offer.node, "train");
        }
    }

    private void event(RunData data, CareerOffer offer, CareerOfferOption option) throws SQLException {
        Event event = rules.events.stream().filter(candidate -> candidate.id.equals(offer.event)).findFirst().orElse(null);
        EventOption choice = event == null ? null : event.options.stream().filter(candidate -> candidate.id.equals(option.id)).findFirst().orElse(null);
        if (choice == null) {
            resolved(data, offer);
            return;
        }
        if (choice.coins < 0 && data.runCoins < -choice.coins) {
            throw refusal("That takes " + (-choice.coins) + " run coins; you have " + data.runCoins + ".");
        }
        data.runCoins = Math.max(0, data.runCoins + choice.coins);
        data.wounds += choice.wound;
        if (choice.heal > 0) {
            data.wounds = Math.max(0, data.wounds - choice.heal);
        }
        if (choice.gamble) {
            if (rng(data, "gamble:" + offer.node).nextBoolean()) {
                data.runCoins += 30;
            } else {
                data.wounds++;
            }
        }
        if (choice.relic) {
            data.offer = relicOffer(data, offer.node);
        } else if (choice.bundle) {
            data.offer = bundles(data, offer.node, "event");
        } else {
            resolved(data, offer);
        }
    }

    /** moves to a node of the map: a fight waits to be played, anything else makes its offer */
    public CareerRun move(String user, String nodeId) throws SQLException, CareerService.CareerException {
        CareerStore store = CareerStore.get();
        return refusing(() -> store.transaction(() -> {
            CareerStore.RunRow row = active(store, user);
            RunData data = gson.fromJson(row.data, RunData.class);
            if (!MODE_MAP.equals(data.mode)) {
                throw refusal("This run has no map.");
            }
            if (data.offer != null) {
                throw refusal("Make your pick first.");
            }
            if (!choices(data).contains(nodeId)) {
                throw refusal("You can't go there from here.");
            }
            Node node = node(data, nodeId);
            data.at = nodeId;
            switch (node.type) {
                case SHOP:
                    data.offer = shopOffer(data, node);
                    break;
                case REST:
                    data.offer = restOffer(data, node);
                    break;
                case EVENT:
                    data.offer = eventOffer(data, node);
                    break;
                default:
                    // a fight: played with careerGauntletPlay
            }
            row.data = gson.toJson(data);
            store.putRun(user, row);
            return view(row);
        }));
    }

    private CareerOffer shopOffer(RunData data, Node node) {
        CareerOffer offer = bundles(data, node.id, "shop");
        offer.kind = "shop";
        offer.title = "Shop";
        offer.text = "Spend run coins on bundles, a relic or healing, then leave when you're done.";
        for (CareerOfferOption option : offer.options) {
            option.price = price(data, rules.map.bundlePrice);
        }
        CareerOfferOption relic = relicOption(data, rng(data, "shop-relic:" + node.id));
        if (relic != null) {
            relic.id = "relic:" + relic.id;
            relic.price = price(data, rules.map.relicPrice);
            offer.options.add(relic);
        }
        if (data.wounds > 0) {
            CareerOfferOption heal = new CareerOfferOption();
            heal.id = "heal";
            heal.kind = "heal";
            heal.name = "Heal a wound";
            heal.text = "Each wound costs " + woundLife(data) + " life at the start of a fight.";
            heal.price = price(data, rules.map.healPrice);
            offer.options.add(heal);
        }
        CareerOfferOption leave = new CareerOfferOption();
        leave.id = "leave";
        leave.kind = "leave";
        leave.name = "Leave the shop";
        offer.options.add(leave);
        return offer;
    }

    private static CareerOffer restOffer(RunData data, Node node) {
        CareerOffer offer = new CareerOffer();
        offer.kind = "rest";
        offer.pick = 1;
        offer.node = node.id;
        offer.title = "A place to rest";
        offer.text = data.wounds > 0 ? "Heal your wounds, or train with a new bundle." : "No wounds to heal: train with a new bundle, or just rest.";
        CareerOfferOption heal = new CareerOfferOption();
        heal.id = "heal";
        heal.kind = "heal";
        heal.name = data.wounds > 0 ? "Heal every wound" : "Rest";
        offer.options.add(heal);
        CareerOfferOption train = new CareerOfferOption();
        train.id = "train";
        train.kind = "train";
        train.name = "Train: pick a bundle";
        offer.options.add(train);
        return offer;
    }

    private CareerOffer eventOffer(RunData data, Node node) {
        Event event = rules.events.get(rng(data, "event:" + node.id).nextInt(rules.events.size()));
        CareerOffer offer = new CareerOffer();
        offer.kind = "event";
        offer.pick = 1;
        offer.node = node.id;
        offer.title = event.name;
        offer.event = event.id;
        offer.text = event.text;
        for (EventOption choice : event.options) {
            CareerOfferOption option = new CareerOfferOption();
            option.id = choice.id;
            option.kind = "choice";
            option.name = choice.label;
            offer.options.add(option);
        }
        return offer;
    }

    private CareerOffer relicOffer(RunData data, String nodeId) {
        CareerOffer offer = new CareerOffer();
        offer.kind = "relic";
        offer.pick = 1;
        offer.node = nodeId;
        offer.title = "Pick a relic";
        Random rng = rng(data, "relic:" + nodeId + ":" + data.relics.size());
        List<Relic> left = new ArrayList<>();
        for (Relic relic : rules.relics) {
            if (!data.relics.contains(relic.id)) {
                left.add(relic);
            }
        }
        Collections.shuffle(left, rng);
        for (Relic relic : left.subList(0, Math.min(3, left.size()))) {
            offer.options.add(relicOptionOf(relic));
        }
        if (offer.options.isEmpty()) {
            return null;
        }
        return offer;
    }

    private CareerOfferOption relicOption(RunData data, Random rng) {
        List<Relic> left = new ArrayList<>();
        for (Relic relic : rules.relics) {
            if (!data.relics.contains(relic.id)) {
                left.add(relic);
            }
        }
        return left.isEmpty() ? null : relicOptionOf(left.get(rng.nextInt(left.size())));
    }

    private static CareerOfferOption relicOptionOf(CareerRelic relic) {
        CareerOfferOption option = new CareerOfferOption();
        option.id = relic.id;
        option.kind = "relic";
        option.name = relic.name;
        option.text = relic.text;
        return option;
    }

    public CareerRun abandon(String user) throws SQLException, CareerService.CareerException {
        CareerStore store = CareerStore.get();
        return refusing(() -> store.transaction(() -> {
            CareerStore.RunRow row = active(store, user);
            row.state = "abandoned";
            row.ended = System.currentTimeMillis();
            store.putRun(user, row);
            return view(row);
        }));
    }

    private static CareerRunCard copy(CareerRunCard card) {
        CareerRunCard copy = new CareerRunCard();
        copy.name = card.name;
        copy.setCode = card.setCode;
        copy.cardNumber = card.cardNumber;
        copy.amount = card.amount;
        return copy;
    }

    private static CareerStore.RunRow active(CareerStore store, String user) throws SQLException {
        List<CareerStore.RunRow> runs = store.runs(user, KIND, true, 1);
        if (runs.isEmpty()) {
            throw refusal("You have no gauntlet run going.");
        }
        return runs.get(0);
    }

    // ---- playing

    /** What the next game needs. */
    public static class Plan {
        public String runId;
        public int bossNumber;
        public CareerCampaigns.NodeOpponent boss;
        public DeckCardLists playerDeck;
        public DeckCardLists opponentDeck;
        public CareerSetup setup;
        /** the skill the opponent plays at this time */
        public int skill;
    }

    public Plan plan(String user) throws SQLException, CareerService.CareerException, IOException {
        CareerStore store = CareerStore.get();
        CareerStore.RunRow row;
        try {
            row = active(store, user);
        } catch (SQLException e) {
            if (e.getCause() instanceof CareerService.CareerException) {
                throw (CareerService.CareerException) e.getCause();
            }
            throw e;
        }
        RunData data = gson.fromJson(row.data, RunData.class);
        if (data.offer != null) {
            throw new CareerService.CareerException("Make your pick before the next game.");
        }
        Plan plan = new Plan();
        plan.runId = row.id;
        plan.bossNumber = row.wins + 1;
        Opponent opponent;
        switch (modeOf(data)) {
            case MODE_CLASSIC:
                opponent = new Opponent();
                opponent.boss = rules.bosses.get(Math.min(row.wins, rules.bosses.size() - 1));
                opponent.twists = opponent.boss.twists;
                opponent.skill = opponent.boss.skill;
                break;
            case MODE_MAP:
                Node here = node(data, data.at);
                if (here == null || !isFight(here) || data.cleared.contains(here.id)) {
                    throw new CareerService.CareerException("Choose where to go on the map first.");
                }
                plan.bossNumber = here.row + 1;
                opponent = mapOpponent(data, here);
                break;
            default:
                if (MODE_BOSS_RUSH.equals(data.mode) && row.wins >= rosterOrder().size()) {
                    throw new CareerService.CareerException("Every boss is beaten.");
                }
                opponent = lineOpponent(data, row.wins);
        }
        plan.boss = opponent.boss;
        plan.skill = opponent.skill;
        plan.playerDeck = new DeckCardLists();
        plan.playerDeck.setName("Gauntlet run");
        for (CareerRunCard card : data.deck) {
            plan.playerDeck.getCards().add(new DeckCardInfo(card.name, card.cardNumber, card.setCode, card.amount));
        }
        if (opponent.boss.deck.startsWith("roster:")) {
            CareerContent.Opponent rosterOpponent = CareerContent.get().opponent(opponent.boss.deck.substring("roster:".length()));
            plan.opponentDeck = CareerContent.get().opponentDeck(rosterOpponent);
        } else {
            plan.opponentDeck = CareerCampaigns.get().deck(opponent.boss.deck);
        }
        plan.setup = CareerSetup.ofRun(opponent.twists, playerTwists(data));
        return plan;
    }

    void finished(CareerStore store, String user, String runId, boolean won, CareerProgress.CareerGameResult result, long now) throws SQLException {
        CareerStore.RunRow row = store.run(user, runId);
        if (row == null || !"active".equals(row.state)) {
            return;
        }
        RunData data = gson.fromJson(row.data, RunData.class);
        String mode = modeOf(data);
        if (MODE_CLASSIC.equals(mode)) {
            finishedClassic(store, user, row, data, won, result, now);
            return;
        }
        Node here = MODE_MAP.equals(mode) ? node(data, data.at) : null;
        String bossName = MODE_MAP.equals(mode) ? (here == null ? "?" : mapOpponent(data, here).boss.name) : lineOpponent(data, row.wins).boss.name;
        result.mode = new CareerProgress.CareerModeResult(KIND, "Gauntlet: " + bossName);
        result.mode.ref = runId;
        if (won) {
            row.wins++;
            boolean purse = runRule(data, "purse");
            if (MODE_MAP.equals(mode) && here != null) {
                data.cleared.add(here.id);
                int coins = (ELITE.equals(here.type) ? rules.map.eliteCoins : rules.map.fightCoins) + (purse ? 10 : 0);
                data.runCoins += coins;
                result.mode.lines.add("Beat " + bossName + ". " + coins + " run coins.");
                if (BOSS.equals(here.type)) {
                    row.state = "won";
                    row.ended = now;
                    result.mode.lines.add("You beat the whole gauntlet" + (data.ascension > 0 ? " at ascension " + data.ascension : "") + "!");
                    store.addStat(user, "gauntlets_won", 1);
                    store.maxStat(user, "gauntlet_ascension_cleared", data.ascension + 1);
                    int open = (int) Math.min(MAX_ASCENSION, data.ascension + 1);
                    if (data.ascension < MAX_ASCENSION) {
                        result.mode.lines.add("Ascension " + open + " is open: " + ascension(open).text);
                    }
                    pay(store, user, Math.max(row.wins, rules.maxWins), result);
                } else if (ELITE.equals(here.type)) {
                    if (!runRule(data, "bandages")) {
                        data.wounds++;
                        result.mode.lines.add("The elite left a wound: " + woundLife(data) + " less life each fight until you rest.");
                    }
                    data.offer = relicOffer(data, here.id);
                    if (data.offer == null) {
                        data.offer = bundles(data, here.id, "bundle");
                    }
                } else {
                    data.offer = bundles(data, here.id, "bundle");
                }
            } else {
                data.runCoins += rules.map.fightCoins + (purse ? 10 : 0);
                result.mode.lines.add("Beat " + bossName + ". " + row.wins + (row.wins == 1 ? " win." : " wins."));
                if (MODE_BOSS_RUSH.equals(mode) && row.wins >= rosterOrder().size()) {
                    row.state = "won";
                    row.ended = now;
                    result.mode.lines.add("Every boss beaten with one deck!");
                    pay(store, user, rules.maxWins, result);
                } else if (MODE_ENDLESS.equals(mode)) {
                    data.offer = bundles(data, "endless-" + row.wins, "bundle");
                }
            }
        } else {
            row.losses++;
            row.state = "lost";
            row.ended = now;
            result.mode.lines.add(bossName + " ended the run at " + row.wins + (row.wins == 1 ? " win." : " wins."));
            pay(store, user, Math.min(row.wins, rules.maxWins), result);
        }
        store.maxStat(user, "gauntlet_best", row.wins);
        if (!"active".equals(row.state)) {
            scores(user, data, row);
        }
        row.data = gson.toJson(data);
        store.putRun(user, row);
    }

    /** a finished run goes on its boards: the mode, the ascension level and the seed (S025, S028, S033) */
    private static void scores(String user, RunData data, CareerStore.RunRow row) {
        try {
            mage.server.solo.Leaderboards boards = mage.server.solo.Leaderboards.get();
            String detail = data.variant + (data.ascension > 0 ? " A" + data.ascension : "");
            switch (data.mode) {
                case MODE_BOSS_RUSH:
                    boards.submit(mage.server.solo.Leaderboards.BOSS_RUSH, "all", user, row.wins, null, detail);
                    break;
                case MODE_ENDLESS:
                    boards.submit(mage.server.solo.Leaderboards.ENDLESS, "all", user, row.wins, null, detail);
                    break;
                default:
                    boards.submit(mage.server.solo.Leaderboards.GAUNTLET, "a" + data.ascension, user, row.wins, (long) data.relics.size(), detail);
                    if ("won".equals(row.state)) {
                        boards.submit(mage.server.solo.Leaderboards.ASCENSION, "all", user, data.ascension, (long) row.wins, detail);
                    }
            }
            boards.submit(mage.server.solo.Leaderboards.GAUNTLET_SEED, seedCode(data.seed), user, row.wins, null, data.mode + " " + detail);
        } catch (SQLException | RuntimeException e) {
            org.apache.log4j.Logger.getLogger(CareerGauntlet.class).warn("Gauntlet boards: a run wasn't recorded", e);
        }
    }

    private void finishedClassic(CareerStore store, String user, CareerStore.RunRow row, RunData data, boolean won,
                                 CareerProgress.CareerGameResult result, long now) throws SQLException {
        CareerCampaigns.NodeOpponent boss = rules.bosses.get(Math.min(row.wins, rules.bosses.size() - 1));
        result.mode = new CareerProgress.CareerModeResult(KIND, "Gauntlet: " + boss.name);
        result.mode.ref = row.id;
        if (won) {
            row.wins++;
            result.mode.lines.add("Beat " + boss.name + ". " + row.wins + " of " + rules.maxWins + ".");
            if (row.wins >= rules.maxWins) {
                row.state = "won";
                row.ended = now;
                result.mode.lines.add("You beat the whole gauntlet!");
                store.addStat(user, "gauntlets_won", 1);
                pay(store, user, row.wins, result);
            } else {
                data.offer = bundles(data, "classic-" + row.wins, "bundle");
            }
        } else {
            row.losses++;
            row.state = "lost";
            row.ended = now;
            result.mode.lines.add(boss.name + " ended the run at " + row.wins + (row.wins == 1 ? " win." : " wins."));
            pay(store, user, row.wins, result);
        }
        store.maxStat(user, "gauntlet_best", row.wins);
        row.data = gson.toJson(data);
        store.putRun(user, row);
    }

    /** a bundle offer: a few cards from packs in the run's colours, one to pick */
    private CareerOffer bundles(RunData data, String nodeId, String why) {
        CareerOffer offer = new CareerOffer();
        offer.kind = "bundle";
        offer.pick = 1;
        offer.node = nodeId;
        int count = runRule(data, "fewerBundles") ? Math.max(1, rules.bundles - 1) : rules.bundles;
        Random rng = data.mode == null ? random : rng(data, why + ":" + nodeId);
        String variant = data.variant == null ? GauntletPacks.JUMPSTART : data.variant;
        for (CareerJumpstart.Pack pack : GauntletPacks.bundles(variant, data.cubeId, data.cubeCards, data.colors, rng, count)) {
            List<DeckCardInfo> spells = new ArrayList<>();
            for (DeckCardInfo card : pack.cards) {
                if (!pack.isBasic(card)) {
                    for (int i = 0; i < card.getAmount(); i++) {
                        spells.add(card);
                    }
                }
            }
            Collections.shuffle(spells, rng);
            CareerOfferOption option = new CareerOfferOption();
            option.id = pack.name + "#" + offer.options.size();
            option.name = pack.name.replaceAll("\\s*\\(\\d+\\)$", "") + " bundle";
            option.color = pack.color;
            option.kind = "bundle";
            Map<String, CareerRunCard> cards = new LinkedHashMap<>();
            for (DeckCardInfo card : spells.subList(0, Math.min(rules.bundleSize, spells.size()))) {
                cards.merge(card.getSetCode() + "|" + card.getCardNumber(), new CareerRunCard(card, 1), (a, b) -> {
                    a.amount += b.amount;
                    return a;
                });
            }
            option.cards.addAll(cards.values());
            offer.options.add(option);
        }
        return offer;
    }

    private void pay(CareerStore store, String user, int wins, CareerProgress.CareerGameResult result) throws SQLException {
        Payout payout = null;
        for (Payout candidate : rules.rewards) {
            if (candidate.wins <= wins && (payout == null || candidate.wins > payout.wins)) {
                payout = candidate;
            }
        }
        if (payout == null) {
            return;
        }
        if (payout.coins > 0) {
            store.addCoins(user, payout.coins);
            result.coins += payout.coins;
            result.mode.coins += payout.coins;
        }
        if (payout.packs > 0) {
            store.addProfileInt(user, "pack_tokens", payout.packs);
            result.packs += payout.packs;
            result.mode.lines.add(payout.packs == 1 ? "A free pack." : payout.packs + " free packs.");
        }
        if (payout.wildcard != null) {
            store.addWildcard(user, payout.wildcard, 1);
            result.mode.lines.add("A " + payout.wildcard + " wildcard.");
        }
    }

    // ---- refusals out of a transaction

    private static SQLException refusal(String message) {
        return new SQLException(new CareerService.CareerException(message));
    }

    private interface Work<T> {
        T run() throws SQLException;
    }

    private static <T> T refusing(Work<T> work) throws SQLException, CareerService.CareerException {
        try {
            return work.run();
        } catch (SQLException e) {
            if (e.getCause() instanceof CareerService.CareerException) {
                throw (CareerService.CareerException) e.getCause();
            }
            throw e;
        }
    }
}
