package mage.server.career;

import com.google.gson.Gson;
import mage.cards.decks.DeckCardInfo;
import mage.cards.decks.DeckCardLists;
import mage.cards.repository.CardInfo;
import mage.cards.repository.CardRepository;

import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Random;

/**
 * Archenemy PvE mode (Phase 5.2): play "Hunt the Archenemy" against a 60-life Scheme Boss or
 * "Become the Archenemy" at 50-60 life with a randomized 20-card Scheme Deck against 3 AI challengers.
 */
public final class CareerArchenemy {

    public static final int FIRST_WIN_COINS = 80;
    public static final int FIRST_WIN_XP = 100;

    static class Config {
        List<CareerSetup.SchemeSpec> schemes = new ArrayList<>();
        List<Encounter> encounters = new ArrayList<>();
    }

    static class SeatSpec {
        String name;
        String deck;
        int skill;
    }

    static class Encounter {
        String id;
        String name;
        /** "hunt" or "become" */
        String mode;
        boolean commander;
        String text;
        int archenemyLife;
        int challengerLife;
        String cover;
        List<SeatSpec> seats = new ArrayList<>();
    }

    public static class CareerArchenemyEncounter {
        public String id;
        public String name;
        public String mode;
        public boolean commander;
        public String text;
        public int archenemyLife;
        public int challengerLife;
        public CareerContent.CareerCover cover;
        public List<String> opponents = new ArrayList<>();
        public boolean beaten;
    }

    public static class CareerScheme {
        public String id;
        public String name;
        public String text;
    }

    public static class CareerArchenemyState {
        public List<CareerArchenemyEncounter> encounters = new ArrayList<>();
        public List<CareerScheme> schemes = new ArrayList<>();
    }

    public static class Plan {
        public Encounter encounter;
        public DeckCardLists playerDeck;
        public List<CareerCampaigns.DuelSeat> seats = new ArrayList<>();
        public CareerSetup setup;
        public boolean commander;
        public boolean planeChase;
    }

    private static volatile CareerArchenemy instance;

    private final Config config;
    private final Map<String, Encounter> byId = new LinkedHashMap<>();

    private CareerArchenemy() throws IOException {
        InputStream stream = CareerArchenemy.class.getResourceAsStream("/career/schemes.json");
        if (stream == null) {
            throw new IOException("Missing career resource schemes.json");
        }
        try (Reader reader = new InputStreamReader(stream, StandardCharsets.UTF_8)) {
            config = new Gson().fromJson(reader, Config.class);
            for (Encounter encounter : config.encounters) {
                byId.put(encounter.id, encounter);
            }
        }
    }

    public static CareerArchenemy get() {
        CareerArchenemy current = instance;
        if (current == null) {
            synchronized (CareerArchenemy.class) {
                current = instance;
                if (current == null) {
                    try {
                        current = new CareerArchenemy();
                    } catch (IOException | RuntimeException e) {
                        throw new IllegalStateException("Archenemy can't be read: " + e.getMessage(), e);
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    public List<String> cardNames() {
        List<String> names = new ArrayList<>();
        for (CareerSetup.SchemeSpec scheme : config.schemes) {
            if (scheme.permanents != null) {
                names.addAll(scheme.permanents);
            }
            if (scheme.emblem != null && !scheme.emblem.isEmpty()) {
                names.add(scheme.emblem);
            }
        }
        for (Encounter encounter : config.encounters) {
            if (encounter.cover != null) {
                names.add(encounter.cover);
            }
        }
        return names;
    }

    public List<CareerSetup.SchemeSpec> schemes() {
        return Collections.unmodifiableList(config.schemes);
    }

    public CareerArchenemyState state(String user) throws SQLException {
        Map<String, Integer> stats = CareerStore.get().stats(user);
        CareerArchenemyState view = new CareerArchenemyState();
        for (Encounter encounter : config.encounters) {
            CareerArchenemyEncounter item = new CareerArchenemyEncounter();
            item.id = encounter.id;
            item.name = encounter.name;
            item.mode = encounter.mode;
            item.commander = encounter.commander;
            item.text = encounter.text;
            item.archenemyLife = encounter.archenemyLife;
            item.challengerLife = encounter.challengerLife;
            item.cover = cover(encounter.cover);
            for (SeatSpec seat : encounter.seats) {
                item.opponents.add(seat.name);
            }
            item.beaten = stats.getOrDefault("archenemy_" + encounter.id, 0) > 0;
            view.encounters.add(item);
        }
        for (CareerSetup.SchemeSpec scheme : config.schemes) {
            CareerScheme s = new CareerScheme();
            s.id = scheme.id;
            s.name = scheme.name;
            s.text = scheme.text;
            view.schemes.add(s);
        }
        return view;
    }

    public Plan attempt(String user, String encounterId, String deckFile, DeckCardLists customDeck, boolean planeChase)
            throws SQLException, CareerService.CareerException {
        Encounter encounter = byId.get(encounterId);
        if (encounter == null) {
            throw new CareerService.CareerException("There's no Archenemy encounter " + encounterId + ".");
        }
        Plan plan = new Plan();
        plan.encounter = encounter;
        plan.commander = encounter.commander;
        plan.planeChase = planeChase;
        if (customDeck != null) {
            int main = customDeck.getCards().stream().mapToInt(DeckCardInfo::getAmount).sum();
            int sb = customDeck.getSideboard().stream().mapToInt(DeckCardInfo::getAmount).sum();
            if (encounter.commander && (main < 98 || sb < 1)) {
                throw new CareerService.CareerException("Commander Archenemy requires a 100-card Commander deck.");
            }
            if (!encounter.commander && main + sb < 40) {
                throw new CareerService.CareerException("Bring a deck with at least 40 cards to Archenemy.");
            }
            plan.playerDeck = customDeck;
        } else {
            String file = (deckFile == null || deckFile.isEmpty())
                    ? (encounter.commander ? "commander/starter.dck" : "red-path.dck")
                    : deckFile;
            try {
                plan.playerDeck = CareerCampaigns.get().deck(file);
            } catch (IOException e) {
                throw new CareerService.CareerException("That deck can't be read: " + e.getMessage());
            }
        }
        for (SeatSpec seatSpec : encounter.seats) {
            CareerCampaigns.DuelSeat seat = new CareerCampaigns.DuelSeat();
            seat.name = seatSpec.name;
            seat.skill = seatSpec.skill;
            seat.aiType = mage.players.PlayerType.COMPUTER_MAD.name();
            try {
                seat.deck = CareerCampaigns.get().deck(seatSpec.deck);
            } catch (IOException e) {
                throw new CareerService.CareerException("Archenemy seat deck couldn't be read: " + seatSpec.deck);
            }
            plan.seats.add(seat);
        }
        List<CareerSetup.SchemeSpec> shuffled = new ArrayList<>(config.schemes);
        Collections.shuffle(shuffled, new Random());
        CareerSetup.ArchenemyRules rules = new CareerSetup.ArchenemyRules();
        rules.name = encounter.name;
        rules.playerIsArchenemy = "become".equals(encounter.mode);
        rules.archenemyLife = encounter.archenemyLife;
        rules.challengerLife = encounter.challengerLife;
        rules.schemes = shuffled.subList(0, Math.min(20, shuffled.size()));
        plan.setup = CareerSetup.plain().withArchenemy(rules);
        return plan;
    }

    void finished(CareerStore store, String user, String encounterId, boolean won, CareerProgress.CareerGameResult result, long now) throws SQLException {
        Encounter encounter = byId.get(encounterId);
        if (encounter == null) {
            return;
        }
        result.mode = new CareerProgress.CareerModeResult("archenemy", encounter.name);
        result.mode.ref = encounterId;
        if (!won) {
            result.mode.lines.add("The Schemes overwhelmed the table.");
            return;
        }
        result.mode.lines.add("Victory in " + encounter.name + "!");
        String key = "archenemy_" + encounter.id;
        boolean firstWin = store.stats(user).getOrDefault(key, 0) == 0;
        store.addStat(user, key, 1);
        store.addStat(user, "archenemy_wins", 1);
        if (firstWin) {
            store.addCoins(user, FIRST_WIN_COINS);
            store.addProfileInt(user, "xp", FIRST_WIN_XP);
            result.coins += FIRST_WIN_COINS;
            result.xp += FIRST_WIN_XP;
            result.mode.coins += FIRST_WIN_COINS;
            result.mode.xp += FIRST_WIN_XP;
        }
    }

    private static CareerContent.CareerCover cover(String name) {
        if (name == null) {
            return null;
        }
        CardInfo info = CardRepository.instance.findPreferredCoreExpansionCard(name);
        if (info == null) {
            return null;
        }
        CareerContent.CareerCover cover = new CareerContent.CareerCover();
        cover.name = info.getName();
        cover.setCode = info.getSetCode();
        cover.cardNumber = info.getCardNumber();
        return cover;
    }
}
