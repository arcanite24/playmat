package mage.server.career;

import com.google.gson.Gson;
import com.google.gson.reflect.TypeToken;
import mage.cards.decks.DeckCardLists;

import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.sql.SQLException;
import java.time.LocalDate;
import java.time.ZoneOffset;
import java.time.temporal.IsoFields;
import java.util.ArrayList;
import java.util.List;
import java.util.function.Supplier;

/**
 * The weekly challenge: the same fixed deck against the same opponent for everyone on the server, a new one each
 * week (resources/career/challenges.json, in turn). The board ranks wins by fewest turns, then most life left.
 */
public final class CareerChallenges {

    public static final int FIRST_WIN_COINS = 50;
    public static final int FIRST_WIN_XP = 100;
    public static final int BOARD_SIZE = 20;

    public static class Challenge {
        public String id;
        public String name;
        public String text;
        public String playerDeck;
        public CareerCampaigns.NodeOpponent opponent;
        /** a content pack's challenge runs only in this ISO week ("2026-W44"); the built-in ones take turns */
        public String week;
    }

    /** This week's challenge, the player's result and the board. */
    public static class CareerChallenge {
        public String week;
        public String id;
        public String name;
        public String text;
        public String opponentName;
        /** the card whose art is the opponent's portrait */
        public CareerContent.CareerCover cover;
        public int skill;
        public CareerSetup.CareerTwists twists;
        public int attempts;
        public boolean won;
        public Integer bestTurns;
        public Integer bestLife;
        public List<CareerChallengeEntry> board = new ArrayList<>();
    }

    public static class CareerChallengeEntry {
        public int rank;
        public String user;
        public boolean won;
        public Integer turns;
        public Integer life;
        public int attempts;
        public boolean you;
    }

    private static volatile CareerChallenges instance;

    private final List<Challenge> challenges;
    private Supplier<LocalDate> today = () -> LocalDate.now(ZoneOffset.UTC);

    /** pack challenges by the week they run */
    private final java.util.Map<String, Challenge> scheduled = new java.util.HashMap<>();

    private CareerChallenges() throws IOException {
        challenges = readBuiltIn();
        for (Challenge challenge : mage.server.solo.ContentPacks.get().challenges()) {
            if (challenge.week != null) {
                scheduled.putIfAbsent(challenge.week, challenge);
            }
        }
    }

    /** the challenges the server ships with, in their weekly turn */
    static List<Challenge> readBuiltIn() throws IOException {
        InputStream stream = CareerChallenges.class.getResourceAsStream("/career/challenges.json");
        if (stream == null) {
            throw new IOException("Missing career resource challenges.json");
        }
        try (Reader reader = new InputStreamReader(stream, StandardCharsets.UTF_8)) {
            return new Gson().fromJson(reader, new TypeToken<List<Challenge>>() {
            }.getType());
        }
    }

    static void forget() {
        instance = null;
    }

    public static CareerChallenges get() {
        CareerChallenges current = instance;
        if (current == null) {
            synchronized (CareerChallenges.class) {
                current = instance;
                if (current == null) {
                    try {
                        current = new CareerChallenges();
                    } catch (IOException | RuntimeException e) {
                        throw new IllegalStateException("Challenges can't be read: " + e.getMessage(), e);
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    void useClock(Supplier<LocalDate> today) {
        this.today = today;
    }

    public List<Challenge> challenges() {
        return challenges;
    }

    /** the content packs' challenges, by week */
    public java.util.Map<String, Challenge> scheduled() {
        return java.util.Collections.unmodifiableMap(scheduled);
    }

    String week() {
        LocalDate date = today.get();
        return date.get(IsoFields.WEEK_BASED_YEAR) + "-W" + String.format("%02d", date.get(IsoFields.WEEK_OF_WEEK_BASED_YEAR));
    }

    /** this week's challenge: the list in turn, by weeks since 2026-01-05 */
    Challenge current() {
        Challenge fromPack = scheduled.get(week());
        if (fromPack != null) {
            return fromPack;
        }
        long weeks = java.time.temporal.ChronoUnit.WEEKS.between(LocalDate.of(2026, 1, 5), today.get());
        return challenges.get((int) Math.floorMod(weeks, challenges.size()));
    }

    public CareerChallenge view(String user) throws SQLException {
        Challenge challenge = current();
        CareerChallenge view = new CareerChallenge();
        view.week = week();
        view.id = challenge.id;
        view.name = challenge.name;
        view.text = challenge.text;
        view.opponentName = challenge.opponent.name;
        view.cover = CareerContent.coverOf("campaigns/decks/" + challenge.opponent.deck);
        view.skill = challenge.opponent.skill;
        view.twists = challenge.opponent.twists;
        mage.server.solo.Leaderboards.Leaderboard board = mage.server.solo.Leaderboards.get()
                .board(mage.server.solo.Leaderboards.WEEKLY_CHALLENGE, view.week, user, mage.server.solo.Leaderboards.SCOPE_SERVER, null, BOARD_SIZE);
        if (board.yours != null) {
            view.attempts = board.yours.attempts;
            view.won = board.yours.v1 != null;
            view.bestTurns = board.yours.v1 == null ? null : board.yours.v1.intValue();
            view.bestLife = board.yours.v2 == null ? null : board.yours.v2.intValue();
        }
        List<mage.server.solo.Leaderboards.LeaderboardEntry> lines = new java.util.ArrayList<>(board.entries);
        if (board.yours != null && board.yours.rank > BOARD_SIZE) {
            lines.add(board.yours);
        }
        for (mage.server.solo.Leaderboards.LeaderboardEntry line : lines) {
            CareerChallengeEntry entry = new CareerChallengeEntry();
            entry.rank = line.rank;
            entry.user = line.user;
            entry.attempts = line.attempts;
            entry.won = line.v1 != null;
            entry.turns = line.v1 == null ? null : line.v1.intValue();
            entry.life = line.v2 == null ? null : line.v2.intValue();
            entry.you = line.you;
            view.board.add(entry);
        }
        return view;
    }

    /** What a played challenge needs. */
    public static class Plan {
        public String week;
        public Challenge challenge;
        public DeckCardLists playerDeck;
        public DeckCardLists opponentDeck;
        public CareerSetup setup;
    }

    public Plan attempt(String user) throws SQLException, IOException {
        Plan plan = new Plan();
        plan.week = week();
        plan.challenge = current();
        plan.playerDeck = CareerCampaigns.get().deck(plan.challenge.playerDeck);
        plan.opponentDeck = CareerCampaigns.get().deck(plan.challenge.opponent.deck);
        plan.setup = CareerSetup.ofTwists(plan.challenge.opponent.twists);
        mage.server.solo.Leaderboards.get().attempt(mage.server.solo.Leaderboards.WEEKLY_CHALLENGE, plan.week, user);
        return plan;
    }

    void finished(CareerStore store, String user, String week, boolean won, CareerTally tally, CareerProgress.CareerGameResult result, long now) throws SQLException {
        Challenge challenge = current();
        result.mode = new CareerProgress.CareerModeResult("challenge", challenge.name);
        result.mode.ref = week;
        if (!won) {
            result.mode.lines.add("No win this time. Try again as often as you like this week.");
            return;
        }
        int turns = Math.max(1, tally.get("my_turns"));
        int life = tally.get("life_at_end");
        mage.server.solo.Leaderboards boards = mage.server.solo.Leaderboards.get();
        boolean first = !boards.hasScore(mage.server.solo.Leaderboards.WEEKLY_CHALLENGE, week, user);
        boards.submit(mage.server.solo.Leaderboards.WEEKLY_CHALLENGE, week, user, turns, (long) life, null);
        result.mode.lines.add("Won in " + turns + (turns == 1 ? " turn" : " turns") + " with " + life + " life left.");
        if (first) {
            store.addCoins(user, FIRST_WIN_COINS);
            store.addProfileInt(user, "xp", FIRST_WIN_XP);
            result.coins += FIRST_WIN_COINS;
            result.xp += FIRST_WIN_XP;
            result.mode.coins += FIRST_WIN_COINS;
            result.mode.xp += FIRST_WIN_XP;
            result.mode.lines.add("Your first win this week.");
        }
    }
}
