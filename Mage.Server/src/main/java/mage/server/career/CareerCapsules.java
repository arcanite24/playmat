package mage.server.career;

import com.google.gson.Gson;
import com.google.gson.reflect.TypeToken;

import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.security.SecureRandom;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.Set;

/**
 * Capsule machines (M29): Career's gachapon. A machine holds ten themed sets of five cosmetics; a token buys a pull,
 * and a pull gives one item the player doesn't have yet, picked at random, so every pull is new and a machine is
 * finished in exactly as many pulls as it holds. Finishing a set gives its sleeve, finishing a machine its own reward
 * and title. Machines come in waves, as content (resources/career/machines/index.json and one file per machine;
 * site/docs/capsules.html says how to add one). Machine 1 holds full-art card styles (kind "cardstyle"): worn on a
 * card, a style replaces the printed card's look with a painting, everywhere that card is drawn.
 */
public final class CareerCapsules {

    /** the cosmetic kind of a card style */
    public static final String CARD_STYLE = "cardstyle";
    /** pulls one request may ask for */
    public static final int MAX_PULLS = 10;
    /** the token bundles the shop sells */
    public static final List<Integer> BUNDLES = Collections.unmodifiableList(java.util.Arrays.asList(1, 5, 10));

    /** A machine's colours: its cabinet's body and trim. */
    public static class CareerCabinet {
        public String body;
        public String trim;
    }

    /** One capsule: a card style of one card. */
    public static class CareerMachineItem {
        /** "<set>-<card slug>"; its picture is cardstyle/<set>/<card slug>.webp */
        public String id;
        /** the card it styles, by its English name */
        public String card;
        /** what the picture shows (the art prompt) */
        public String subject;
    }

    /** Five capsules in one style and theme. */
    public static class CareerMachineSet {
        public String id;
        public String name;
        public String es;
        /** the capsule's colour (#rrggbb) */
        public String capsule;
        /** the set's art style (the art prompt) */
        public String style;
        public List<CareerMachineItem> items = new ArrayList<>();
        /** what finishing the set gives: its sleeve */
        public CareerProgress.CareerCosmetic reward;
    }

    /** A machine, as content and as the shop shows it. */
    public static class CareerMachine {
        public String id;
        /** the wave it came in; the shop shows the newest first */
        public int wave;
        public String name;
        public String es;
        public String blurb;
        public String blurbEs;
        /** the kind of cosmetic it holds */
        public String kind;
        public CareerCabinet cabinet;
        /** what finishing it gives, and its title */
        public CareerProgress.CareerCosmetic reward;
        public CareerProgress.CareerCosmetic title;
        public List<CareerMachineSet> sets = new ArrayList<>();
    }

    /** The shop: every machine, the player's tokens and what they've pulled. */
    public static class CareerMachines {
        public int tokens;
        public int coins;
        /** coins a token costs, and the bundles sold */
        public int tokenPrice;
        public List<Integer> bundles;
        /** newest wave first */
        public List<CareerMachine> machines = new ArrayList<>();
        /** every item pulled, any machine */
        public List<String> owned = new ArrayList<>();
        /** card styles worn: card name to item */
        public Map<String, String> styles = new LinkedHashMap<>();
    }

    /** One capsule a pull gave. */
    public static class CareerPull {
        public String item;
        public String set;
        public String card;
        /** this capsule finished its set, and the set's reward */
        public boolean setDone;
        public CareerProgress.CareerCosmetic setReward;
        /** this capsule finished the machine, and the machine's rewards */
        public boolean machineDone;
        public List<CareerProgress.CareerCosmetic> machineRewards = new ArrayList<>();
    }

    /** What a pull request gave. */
    public static class CareerPullResult {
        public String machine;
        /** in the order they came out */
        public List<CareerPull> pulls = new ArrayList<>();
        /** tokens left */
        public int tokens;
        /** capsules left in the machine */
        public int left;
    }

    private static volatile CareerCapsules instance;

    private final List<CareerMachine> machines;
    private final Map<String, CareerMachine> byId = new LinkedHashMap<>();
    private final Map<String, CareerMachineItem> items = new LinkedHashMap<>();
    private final Map<String, CareerMachineSet> setOf = new LinkedHashMap<>();
    private final Map<String, CareerMachine> machineOf = new LinkedHashMap<>();
    private Random random = new SecureRandom();

    CareerCapsules(List<CareerMachine> machines) {
        this.machines = machines;
        for (CareerMachine machine : machines) {
            byId.put(machine.id, machine);
            for (CareerMachineSet set : machine.sets) {
                set.reward = new CareerProgress.CareerCosmetic("sleeve", "set-" + set.id);
                for (CareerMachineItem item : set.items) {
                    items.put(item.id, item);
                    setOf.put(item.id, set);
                    machineOf.put(item.id, machine);
                }
            }
        }
    }

    public static CareerCapsules get() {
        CareerCapsules current = instance;
        if (current == null) {
            synchronized (CareerCapsules.class) {
                current = instance;
                if (current == null) {
                    try {
                        current = load();
                    } catch (IOException e) {
                        throw new IllegalStateException("Capsule machines can't be read", e);
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    private static class IndexEntry {
        String id;
        int wave;
    }

    private static CareerCapsules load() throws IOException {
        Gson gson = new Gson();
        List<IndexEntry> index = gson.fromJson(text("machines/index.json"), new TypeToken<List<IndexEntry>>() {
        }.getType());
        List<CareerMachine> machines = new ArrayList<>();
        for (IndexEntry entry : index) {
            CareerMachine machine = gson.fromJson(text("machines/" + entry.id + ".json"), CareerMachine.class);
            if (!entry.id.equals(machine.id)) {
                throw new IOException("machines/" + entry.id + ".json holds machine " + machine.id);
            }
            machine.wave = entry.wave;
            machines.add(machine);
        }
        machines.sort((a, b) -> Integer.compare(b.wave, a.wave));
        return new CareerCapsules(machines);
    }

    private static String text(String name) throws IOException {
        InputStream stream = CareerCapsules.class.getResourceAsStream("/career/" + name);
        if (stream == null) {
            throw new IOException("Missing career resource " + name);
        }
        try (Reader reader = new InputStreamReader(stream, StandardCharsets.UTF_8)) {
            StringBuilder text = new StringBuilder();
            char[] buffer = new char[8192];
            for (int read; (read = reader.read(buffer)) > 0; ) {
                text.append(buffer, 0, read);
            }
            return text.toString();
        }
    }

    /** for tests: a fixed draw */
    void random(Random random) {
        this.random = random;
    }

    public List<CareerMachine> machines() {
        return Collections.unmodifiableList(machines);
    }

    public CareerMachine machine(String id) {
        return id == null ? null : byId.get(id);
    }

    public CareerMachineItem item(String id) {
        return id == null ? null : items.get(id);
    }

    /**
     * The catalog entries machines add (CareerCosmetics): every card style, and every set's sleeve. A machine's own
     * reward and title are ordinary catalog items (cosmetics.json); the catalog gives them their source.
     */
    List<CareerCosmetics.CareerCatalogItem> catalogItems() {
        List<CareerCosmetics.CareerCatalogItem> catalog = new ArrayList<>();
        for (CareerMachine machine : machines) {
            for (CareerMachineSet set : machine.sets) {
                for (CareerMachineItem item : set.items) {
                    CareerCosmetics.CareerCatalogItem style = new CareerCosmetics.CareerCatalogItem();
                    style.kind = machine.kind;
                    style.id = item.id;
                    style.name = item.card;
                    style.es = item.card;
                    style.rarity = "rare";
                    style.color = set.capsule;
                    style.card = item.card;
                    style.series = set.id;
                    style.sources.add(new CareerCosmetics.CareerCosmeticSource("capsule", machine.id, 0));
                    catalog.add(style);
                }
                CareerCosmetics.CareerCatalogItem sleeve = new CareerCosmetics.CareerCatalogItem();
                sleeve.kind = set.reward.kind;
                sleeve.id = set.reward.id;
                sleeve.name = set.name;
                sleeve.es = set.es;
                sleeve.rarity = "mythic";
                sleeve.color = set.capsule;
                sleeve.series = set.id;
                sleeve.sources.add(new CareerCosmetics.CareerCosmeticSource("capsuleSet", machine.id, 0));
                catalog.add(sleeve);
            }
        }
        return catalog;
    }

    // ---- tokens

    /** gives (or, negative, takes) tokens, logged; inside the caller's transaction */
    static void addTokens(CareerStore store, String user, int amount, String reason, long now) throws SQLException {
        if (amount == 0) {
            return;
        }
        store.addProfileInt(user, "capsule_tokens", amount);
        store.logTokens(user, amount, reason, now);
    }

    // ---- the shop

    public CareerMachines shop(String user) throws SQLException {
        CareerStore store = CareerStore.get();
        CareerService.ensureProfile(store, user, System.currentTimeMillis());
        CareerProfile profile = store.profile(user);
        CareerMachines shop = new CareerMachines();
        shop.tokens = profile.capsuleTokens;
        shop.coins = profile.coins;
        shop.tokenPrice = CareerRewards.get().capsules().tokenPrice;
        shop.bundles = BUNDLES;
        shop.machines.addAll(machines);
        shop.owned.addAll(owned(store, user));
        shop.styles.putAll(store.cardStyles(user));
        return shop;
    }

    /** every machine item the account has */
    private List<String> owned(CareerStore store, String user) throws SQLException {
        List<String> owned = new ArrayList<>();
        for (String[] unlock : store.unlocks(user)) {
            if (items.containsKey(unlock[1]) && kindOf(unlock[1]).equals(unlock[0])) {
                owned.add(unlock[1]);
            }
        }
        return owned;
    }

    private String kindOf(String item) {
        CareerMachine machine = machineOf.get(item);
        return machine == null ? "" : machine.kind;
    }

    /** Buys tokens with coins, in one of the shop's bundles. */
    public CareerProfile buyTokens(String user, int count) throws SQLException, CareerService.CareerException {
        if (!BUNDLES.contains(count)) {
            throw new CareerService.CareerException("Tokens come in bundles of " + BUNDLES + ".");
        }
        CareerStore store = CareerStore.get();
        int price = CareerRewards.get().capsules().tokenPrice * count;
        CareerService.refusing(() -> store.transaction(() -> {
            long now = System.currentTimeMillis();
            CareerService.ensureProfile(store, user, now);
            CareerProfile profile = store.profile(user);
            if (profile.coins < price) {
                throw new SQLException(new CareerService.CareerException(
                        count + (count == 1 ? " token costs " : " tokens cost ") + price + " coins; you have " + profile.coins + "."));
            }
            store.addCoins(user, -price);
            addTokens(store, user, count, "bought", now);
            return null;
        }));
        return store.profile(user);
    }

    /**
     * Pulls up to {@code count} capsules from a machine: each one a different item the player doesn't have, picked
     * at random among those left. A request for more than the machine has left gives (and charges) what's left.
     */
    public CareerPullResult pull(String user, String machineId, int count) throws SQLException, CareerService.CareerException {
        CareerMachine machine = machine(machineId);
        if (machine == null) {
            throw new CareerService.CareerException("There's no machine called " + machineId + ".");
        }
        if (count < 1 || count > MAX_PULLS) {
            throw new CareerService.CareerException("Pull 1 to " + MAX_PULLS + " capsules at a time.");
        }
        CareerStore store = CareerStore.get();
        CareerPullResult result = new CareerPullResult();
        result.machine = machine.id;
        CareerService.refusing(() -> store.transaction(() -> {
            long now = System.currentTimeMillis();
            CareerService.ensureProfile(store, user, now);
            Set<String> owned = new HashSet<>(owned(store, user));
            List<CareerMachineItem> left = new ArrayList<>();
            for (CareerMachineSet set : machine.sets) {
                for (CareerMachineItem item : set.items) {
                    if (!owned.contains(item.id)) {
                        left.add(item);
                    }
                }
            }
            if (left.isEmpty()) {
                throw new SQLException(new CareerService.CareerException("You have every capsule in " + machine.name + "."));
            }
            int pulls = Math.min(count, left.size());
            int tokens = store.profile(user).capsuleTokens;
            if (tokens < pulls) {
                throw new SQLException(new CareerService.CareerException(
                        (pulls == 1 ? "A pull takes a token" : pulls + " pulls take " + pulls + " tokens") + "; you have " + tokens + "."));
            }
            addTokens(store, user, -pulls, "pull " + machine.id, now);
            for (int index = 0; index < pulls; index++) {
                CareerMachineItem item = left.remove(random.nextInt(left.size()));
                store.addUnlock(user, machine.kind, item.id, now);
                store.addPull(user, machine.id, item.id, now);
                owned.add(item.id);
                CareerMachineSet set = setOf.get(item.id);
                CareerPull pull = new CareerPull();
                pull.item = item.id;
                pull.set = set.id;
                pull.card = item.card;
                if (set.items.stream().allMatch(one -> owned.contains(one.id))) {
                    pull.setDone = true;
                    pull.setReward = set.reward;
                    store.addUnlock(user, set.reward.kind, set.reward.id, now);
                    store.addStat(user, "capsule_sets", 1);
                }
                if (left.isEmpty()) {
                    pull.machineDone = true;
                    for (CareerProgress.CareerCosmetic reward : new CareerProgress.CareerCosmetic[]{machine.reward, machine.title}) {
                        if (reward != null) {
                            store.addUnlock(user, reward.kind, reward.id, now);
                            pull.machineRewards.add(reward);
                        }
                    }
                    store.addStat(user, "capsule_machines", 1);
                }
                result.pulls.add(pull);
            }
            store.addStat(user, "capsules_pulled", pulls);
            CareerProgress.get().achievementsIn(store, user, null, null, now);
            result.tokens = store.profile(user).capsuleTokens;
            result.left = left.size();
            return null;
        }));
        return result;
    }

    /** Wears a card style on its card, or takes the card's style off (a null item). */
    public Map<String, String> equipStyle(String user, String card, String itemId) throws SQLException, CareerService.CareerException {
        if (card == null || card.isEmpty()) {
            throw new CareerService.CareerException("Which card?");
        }
        CareerStore store = CareerStore.get();
        if (itemId != null) {
            CareerMachineItem item = item(itemId);
            if (item == null || !item.card.equals(card)) {
                throw new CareerService.CareerException("That style isn't for " + card + ".");
            }
            if (!owned(store, user).contains(itemId)) {
                throw new CareerService.CareerException("Pull that style from its machine first.");
            }
        }
        store.setCardStyle(user, card, itemId);
        return store.cardStyles(user);
    }

    /** the card styles an account wears that it still owns: card name to item, for its look */
    public Map<String, String> worn(String user) throws SQLException {
        CareerStore store = CareerStore.get();
        Set<String> owned = new HashSet<>(owned(store, user));
        Map<String, String> worn = new LinkedHashMap<>();
        for (Map.Entry<String, String> style : store.cardStyles(user).entrySet()) {
            CareerMachineItem item = item(style.getValue());
            if (item != null && item.card.equals(style.getKey()) && owned.contains(item.id)) {
                worn.put(style.getKey(), item.id);
            }
        }
        return worn;
    }
}
