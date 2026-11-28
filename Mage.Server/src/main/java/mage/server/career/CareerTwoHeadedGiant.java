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
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Two-Headed Giant solo mode (Phase 5.4): pair up with a chosen AI partner (aggro, control, midrange, tempo,
 * or Commander) against an opposing 2-head AI team. Team tags ensure allies never attack each other and
 * victory triggers as soon as both opposing heads are eliminated.
 */
public final class CareerTwoHeadedGiant {

    public static final int FIRST_WIN_COINS = 75;
    public static final int FIRST_WIN_XP = 90;

    static class Config {
        List<PartnerSpec> partners = new ArrayList<>();
        List<PairSpec> pairs = new ArrayList<>();
    }

    static class PartnerSpec {
        String id;
        String name;
        String archetype;
        boolean commander;
        String deck;
        int skill;
        String text;
        String cover;
    }

    static class EnemySpec {
        String name;
        String deck;
        int skill;
    }

    static class PairSpec {
        String id;
        String name;
        boolean commander;
        int teamLife;
        String text;
        String cover;
        List<EnemySpec> enemies = new ArrayList<>();
    }

    public static class CareerThgPartner {
        public String id;
        public String name;
        public String archetype;
        public boolean commander;
        public String text;
        public CareerContent.CareerCover cover;
    }

    public static class CareerThgPair {
        public String id;
        public String name;
        public boolean commander;
        public int teamLife;
        public String text;
        public CareerContent.CareerCover cover;
        public List<String> enemies = new ArrayList<>();
        public boolean beaten;
    }

    public static class CareerThgState {
        public List<CareerThgPartner> partners = new ArrayList<>();
        public List<CareerThgPair> pairs = new ArrayList<>();
    }

    public static class Plan {
        public PairSpec pair;
        public PartnerSpec partner;
        public DeckCardLists playerDeck;
        public List<CareerCampaigns.DuelSeat> seats = new ArrayList<>();
        public CareerSetup setup;
        public boolean commander;
        public boolean planeChase;
    }

    private static volatile CareerTwoHeadedGiant instance;

    private final Config config;
    private final Map<String, PartnerSpec> partnersById = new LinkedHashMap<>();
    private final Map<String, PairSpec> pairsById = new LinkedHashMap<>();

    private CareerTwoHeadedGiant() throws IOException {
        InputStream stream = CareerTwoHeadedGiant.class.getResourceAsStream("/career/two-headed-giant.json");
        if (stream == null) {
            throw new IOException("Missing career resource two-headed-giant.json");
        }
        try (Reader reader = new InputStreamReader(stream, StandardCharsets.UTF_8)) {
            config = new Gson().fromJson(reader, Config.class);
            for (PartnerSpec partner : config.partners) {
                partnersById.put(partner.id, partner);
            }
            for (PairSpec pair : config.pairs) {
                pairsById.put(pair.id, pair);
            }
        }
    }

    public static CareerTwoHeadedGiant get() {
        CareerTwoHeadedGiant current = instance;
        if (current == null) {
            synchronized (CareerTwoHeadedGiant.class) {
                current = instance;
                if (current == null) {
                    try {
                        current = new CareerTwoHeadedGiant();
                    } catch (IOException | RuntimeException e) {
                        throw new IllegalStateException("Two-Headed Giant can't be read: " + e.getMessage(), e);
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    public CareerThgState state(String user) throws SQLException {
        Map<String, Integer> stats = CareerStore.get().stats(user);
        CareerThgState view = new CareerThgState();
        for (PartnerSpec p : config.partners) {
            CareerThgPartner item = new CareerThgPartner();
            item.id = p.id;
            item.name = p.name;
            item.archetype = p.archetype;
            item.commander = p.commander;
            item.text = p.text;
            item.cover = cover(p.cover);
            view.partners.add(item);
        }
        for (PairSpec pair : config.pairs) {
            CareerThgPair item = new CareerThgPair();
            item.id = pair.id;
            item.name = pair.name;
            item.commander = pair.commander;
            item.teamLife = pair.teamLife;
            item.text = pair.text;
            item.cover = cover(pair.cover);
            for (EnemySpec enemy : pair.enemies) {
                item.enemies.add(enemy.name);
            }
            item.beaten = stats.getOrDefault("thg_" + pair.id, 0) > 0;
            view.pairs.add(item);
        }
        return view;
    }

    public Plan attempt(String user, String pairId, String partnerId, String deckFile, DeckCardLists customDeck, boolean planeChase)
            throws SQLException, CareerService.CareerException {
        PairSpec pair = pairsById.get(pairId);
        if (pair == null) {
            throw new CareerService.CareerException("There's no Two-Headed Giant match " + pairId + ".");
        }
        PartnerSpec partner = partnersById.get(partnerId);
        if (partner == null || partner.commander != pair.commander) {
            throw new CareerService.CareerException("Pick a matching AI partner for this Two-Headed Giant match.");
        }
        DeckCardLists deck;
        if (customDeck != null && customDeck.getCards() != null && !customDeck.getCards().isEmpty()) {
            deck = customDeck;
        } else {
            String file = (deckFile == null || deckFile.trim().isEmpty())
                    ? (pair.commander ? "commander/act3-tokens.dck" : "red-path.dck")
                    : deckFile.trim();
            try {
                deck = CareerCampaigns.get().deck(file);
            } catch (IOException e) {
                throw new CareerService.CareerException("That deck can't be read: " + e.getMessage());
            }
        }

        Plan plan = new Plan();
        plan.pair = pair;
        plan.partner = partner;
        plan.playerDeck = deck;
        plan.commander = pair.commander || isCommanderDeck(deck);
        plan.planeChase = planeChase;

        // Seat 1: AI Partner, Seats 2-3: Opposing pair
        CareerCampaigns.DuelSeat allySeat = new CareerCampaigns.DuelSeat();
        allySeat.name = partner.name;
        allySeat.aiType = mage.players.PlayerType.COMPUTER_MAD.name();
        try {
            allySeat.deck = CareerCampaigns.get().deck(partner.deck);
        } catch (IOException e) {
            throw new CareerService.CareerException("Partner deck couldn't be read: " + partner.deck);
        }
        allySeat.skill = partner.skill > 0 ? partner.skill : 6;
        plan.seats.add(allySeat);

        for (EnemySpec enemy : pair.enemies) {
            CareerCampaigns.DuelSeat seat = new CareerCampaigns.DuelSeat();
            seat.name = enemy.name;
            seat.aiType = mage.players.PlayerType.COMPUTER_MAD.name();
            try {
                seat.deck = CareerCampaigns.get().deck(enemy.deck);
            } catch (IOException e) {
                throw new CareerService.CareerException("Enemy deck couldn't be read: " + enemy.deck);
            }
            seat.skill = enemy.skill > 0 ? enemy.skill : 6;
            plan.seats.add(seat);
        }

        CareerSetup.TwoHeadedGiantSpec spec = new CareerSetup.TwoHeadedGiantSpec();
        spec.name = pair.name;
        spec.partnerName = partner.name;
        spec.teamLife = pair.teamLife > 0 ? pair.teamLife : (plan.commander ? 40 : 30);
        spec.commander = plan.commander;
        plan.setup = CareerSetup.twoHeadedGiant(spec, null);
        return plan;
    }

    void finished(CareerStore store, String user, String pairId, boolean won, CareerProgress.CareerGameResult result) throws SQLException {
        PairSpec pair = pairsById.get(pairId);
        CareerProgress.CareerModeResult mode = new CareerProgress.CareerModeResult("thg", pair == null ? "Two-Headed Giant" : pair.name);
        mode.ref = pairId;
        if (!won) {
            mode.lines.add("The opposing pair prevailed. Try a different partner archetype or deck.");
            result.mode = mode;
            return;
        }
        String stat = "thg_" + pairId;
        boolean first = store.stats(user).getOrDefault(stat, 0) == 0;
        store.addStat(user, stat, 1);
        store.addStat(user, "thg_wins", 1);
        mode.lines.add("Victory! Your Two-Headed Giant team defeated " + (pair == null ? "the opposing pair" : pair.name) + ".");
        if (first) {
            mode.coins = FIRST_WIN_COINS;
            mode.xp = FIRST_WIN_XP;
            store.addCoins(user, mode.coins);
            store.addProfileInt(user, "xp", mode.xp);
            result.coins += mode.coins;
            result.xp += mode.xp;
            mode.lines.add("First clear bonus: +" + mode.coins + " coins and +" + mode.xp + " XP.");
        }
        result.mode = mode;
    }

    private static boolean isCommanderDeck(DeckCardLists deck) {
        if (deck == null || deck.getSideboard() == null || deck.getSideboard().isEmpty() || deck.getSideboard().size() > 2) {
            return false;
        }
        int main = 0;
        for (DeckCardInfo info : deck.getCards()) {
            main += Math.max(1, info.getAmount());
        }
        int sb = 0;
        for (DeckCardInfo info : deck.getSideboard()) {
            sb += Math.max(1, info.getAmount());
        }
        return main + sb >= 98;
    }

    private static CareerContent.CareerCover cover(String name) {
        if (name == null || name.isEmpty()) {
            return null;
        }
        CardInfo info = CardRepository.instance.findCard(name);
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
