package mage.server.career;

import com.google.gson.Gson;
import com.google.gson.reflect.TypeToken;
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
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Horde mode (S036): a deck that plays itself. Each of its upkeeps it flips cards and puts the permanents onto the
 * battlefield; its creatures have haste and attack every combat. The player brings a deck from the meta library and
 * lives through a set number of the horde's turns (resources/career/hordes.json). The board ranks by turns lived,
 * then life left.
 */
public final class CareerHordes {

    public static final int FIRST_WIN_COINS = 60;
    public static final int FIRST_WIN_XP = 80;

    static class Horde {
        String id;
        String name;
        String text;
        int survive;
        int flip;
        int rampEvery;
        Integer life;
        String cover;
        Map<String, Integer> deck = new LinkedHashMap<>();
    }

    /** A horde as a player sees it. */
    public static class CareerHorde {
        public String id;
        public String name;
        public String text;
        public int survive;
        public int flip;
        public int rampEvery;
        public int life;
        public int deckSize;
        public CareerContent.CareerCover cover;
        public int attempts;
        public boolean beaten;
        /** the player's best: turns lived and life left */
        public Long bestTurns;
        public Long bestLife;
    }

    /** What a horde game needs. */
    public static class Plan {
        public Horde horde;
        public DeckCardLists playerDeck;
        public DeckCardLists hordeDeck;
        public CareerSetup setup;
        public String name;
    }

    private static volatile CareerHordes instance;

    private final Map<String, Horde> hordes = new LinkedHashMap<>();

    private CareerHordes() throws IOException {
        InputStream stream = CareerHordes.class.getResourceAsStream("/career/hordes.json");
        if (stream == null) {
            throw new IOException("Missing career resource hordes.json");
        }
        try (Reader reader = new InputStreamReader(stream, StandardCharsets.UTF_8)) {
            List<Horde> list = new Gson().fromJson(reader, new TypeToken<List<Horde>>() {
            }.getType());
            for (Horde horde : list) {
                hordes.put(horde.id, horde);
            }
        }
    }

    public static CareerHordes get() {
        CareerHordes current = instance;
        if (current == null) {
            synchronized (CareerHordes.class) {
                current = instance;
                if (current == null) {
                    try {
                        current = new CareerHordes();
                    } catch (IOException | RuntimeException e) {
                        throw new IllegalStateException("Hordes can't be read: " + e.getMessage(), e);
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    /** every card name of the hordes, for the content check */
    public List<String> cardNames() {
        List<String> names = new ArrayList<>();
        for (Horde horde : hordes.values()) {
            names.addAll(horde.deck.keySet());
            if (horde.cover != null) {
                names.add(horde.cover);
            }
        }
        return names;
    }

    public List<CareerHorde> hordes(String user) throws SQLException {
        List<CareerHorde> views = new ArrayList<>();
        for (Horde horde : hordes.values()) {
            CareerHorde view = new CareerHorde();
            view.id = horde.id;
            view.name = horde.name;
            view.text = horde.text;
            view.survive = horde.survive;
            view.flip = horde.flip;
            view.rampEvery = horde.rampEvery;
            view.life = horde.life == null ? 20 : horde.life;
            view.deckSize = horde.deck.values().stream().mapToInt(Integer::intValue).sum();
            view.cover = cover(horde.cover);
            mage.server.solo.SoloStore.ScoreRow row = mage.server.solo.SoloStore.get().score(mage.server.solo.Leaderboards.HORDE, horde.id,
                    user.toLowerCase(java.util.Locale.ENGLISH));
            if (row != null) {
                view.attempts = row.attempts;
                view.bestTurns = row.v1;
                view.bestLife = row.v2;
                view.beaten = row.v1 != null && row.v1 > horde.survive;
            }
            views.add(view);
        }
        return views;
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

    /**
     * @param deckFile a deck of the meta library (a 60-card format) to bring
     */
    public Plan attempt(String user, String hordeId, String deckFile) throws SQLException, CareerService.CareerException {
        Horde horde = hordes.get(hordeId);
        if (horde == null) {
            throw new CareerService.CareerException("There's no horde " + hordeId + ".");
        }
        MetaLibrary.MetaDeck meta = MetaLibrary.get().deck(deckFile);
        if (meta == null || meta.deckType == null || !meta.deckType.startsWith("Constructed")) {
            throw new CareerService.CareerException("Bring a deck of the meta library to a horde.");
        }
        Plan plan = new Plan();
        plan.horde = horde;
        plan.name = horde.name;
        try {
            plan.playerDeck = CareerCampaigns.get().deck(deckFile);
        } catch (IOException e) {
            throw new CareerService.CareerException("That deck can't be read: " + e.getMessage());
        }
        plan.hordeDeck = new DeckCardLists();
        plan.hordeDeck.setName(horde.name);
        for (Map.Entry<String, Integer> entry : horde.deck.entrySet()) {
            CardInfo info = CardRepository.instance.findPreferredCoreExpansionCard(entry.getKey());
            plan.hordeDeck.getCards().add(info == null ? new DeckCardInfo(entry.getKey(), null, null, entry.getValue())
                    : new DeckCardInfo(info.getName(), info.getCardNumber(), info.getSetCode(), entry.getValue()));
        }
        CareerSetup.HordeRules rules = new CareerSetup.HordeRules();
        rules.name = horde.name;
        rules.survive = horde.survive;
        rules.flip = horde.flip;
        rules.rampEvery = horde.rampEvery;
        rules.life = horde.life;
        plan.setup = CareerSetup.plain().withHorde(rules);
        mage.server.solo.Leaderboards.get().attempt(mage.server.solo.Leaderboards.HORDE, horde.id, user);
        return plan;
    }

    void finished(CareerStore store, String user, String hordeId, boolean won, CareerTally tally, CareerProgress.CareerGameResult result, long now) throws SQLException {
        Horde horde = hordes.get(hordeId);
        if (horde == null) {
            return;
        }
        result.mode = new CareerProgress.CareerModeResult("horde", horde.name);
        result.mode.ref = hordeId;
        // turns lived, counted by the player's own turns (the horde's come between them); a win is one past the last
        int turns = won ? horde.survive + 1 : Math.max(0, Math.min(horde.survive, tally.get("my_turns")));
        int life = Math.max(0, tally.get("life_at_end"));
        mage.server.solo.Leaderboards boards = mage.server.solo.Leaderboards.get();
        boolean firstWin = won && !beaten(user, horde);
        boards.submit(mage.server.solo.Leaderboards.HORDE, horde.id, user, turns, (long) life, null);
        if (!won) {
            result.mode.lines.add("The horde got you after " + turns + (turns == 1 ? " turn." : " turns."));
            return;
        }
        result.mode.lines.add("You lived through the horde with " + life + " life left.");
        if (firstWin) {
            store.addCoins(user, FIRST_WIN_COINS);
            store.addProfileInt(user, "xp", FIRST_WIN_XP);
            store.addStat(user, "hordes_beaten", 1);
            result.coins += FIRST_WIN_COINS;
            result.xp += FIRST_WIN_XP;
            result.mode.coins += FIRST_WIN_COINS;
            result.mode.xp += FIRST_WIN_XP;
        }
    }

    private static boolean beaten(String user, Horde horde) throws SQLException {
        mage.server.solo.SoloStore.ScoreRow row = mage.server.solo.SoloStore.get().score(mage.server.solo.Leaderboards.HORDE, horde.id,
                user.toLowerCase(java.util.Locale.ENGLISH));
        return row != null && row.v1 != null && row.v1 > horde.survive;
    }
}
