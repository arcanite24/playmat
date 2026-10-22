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
        /** how a win scores (S032): "turns" (fewest, the default), "life" (most left) or "no-creatures" (fewest turns, no creature cast) */
        public String rule;
        /** the format, for a generated challenge */
        public String format;
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
        /** "turns", "life" or "no-creatures" */
        public String rule;
        /** the leaderboard mode this week's scores go to */
        public String boardMode;
        public String format;
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

    /**
     * This week's challenge: a content pack's for the week, else the hand-made list and the generated year take turns
     * (even weeks since 2026-01-05 the list in order, odd weeks the year's generated challenge for the week).
     */
    Challenge current() {
        Challenge fromPack = scheduled.get(week());
        if (fromPack != null) {
            return fromPack;
        }
        LocalDate date = today.get();
        long weeks = java.time.temporal.ChronoUnit.WEEKS.between(LocalDate.of(2026, 1, 5), date);
        if (Math.floorMod(weeks, 2) == 1) {
            List<Challenge> year = year(date.get(IsoFields.WEEK_BASED_YEAR));
            if (!year.isEmpty()) {
                return year.get(Math.min(year.size(), date.get(IsoFields.WEEK_OF_WEEK_BASED_YEAR)) - 1);
            }
        }
        return challenges.get((int) Math.floorMod(weeks / 2, challenges.size()));
    }

    // ---- the generated year (S032)

    static final String[] RULES = {"turns", "life", "no-creatures"};

    /** twists the opponent may start with; every card here is in the core sets */
    private static final CareerSetup.CareerTwists[] TWISTS = {
            twist(5, 0, null, null),
            twist(0, 1, null, null),
            twist(0, 0, "Mind Stone", null),
            twist(0, 0, null, "Glorious Anthem"),
            twist(3, 1, null, null),
            twist(0, 0, "Wall of Wood", null),
    };

    private static CareerSetup.CareerTwists twist(int life, int cards, String permanent, String emblem) {
        CareerSetup.CareerTwists twists = new CareerSetup.CareerTwists();
        twists.lifeBonus = life;
        twists.extraCards = cards;
        if (permanent != null) {
            twists.permanents = new java.util.ArrayList<>(java.util.Collections.singletonList(permanent));
        }
        twists.emblem = emblem;
        return twists;
    }

    private final java.util.Map<Integer, List<Challenge>> years = new java.util.concurrent.ConcurrentHashMap<>();

    /** a year of challenges (one per ISO week) from the meta library, the same on every server for the same year */
    public List<Challenge> year(int year) {
        return years.computeIfAbsent(year, CareerChallenges::generate);
    }

    static List<Challenge> generate(int year) {
        List<Challenge> list = new java.util.ArrayList<>();
        List<MetaLibrary.MetaFormat> formats = new java.util.ArrayList<>();
        for (MetaLibrary.MetaFormat format : MetaLibrary.get().formats()) {
            boolean constructed = format.decks.stream().anyMatch(deck -> deck.deckType != null && deck.deckType.startsWith("Constructed"));
            if (constructed && format.decks.size() >= 2) {
                formats.add(format);
            }
        }
        if (formats.isEmpty()) {
            return list;
        }
        java.util.Random random = new java.util.Random(year * 1_000_003L);
        int weeks = LocalDate.of(year, 12, 28).get(IsoFields.WEEK_OF_WEEK_BASED_YEAR);
        for (int week = 1; week <= weeks; week++) {
            MetaLibrary.MetaFormat format = formats.get((week - 1) % formats.size());
            List<MetaLibrary.MetaDeck> decks = new java.util.ArrayList<>(format.decks);
            java.util.Collections.shuffle(decks, random);
            MetaLibrary.MetaDeck yours = decks.get(0);
            MetaLibrary.MetaDeck theirs = decks.stream().skip(1).max(java.util.Comparator.comparingInt(deck -> deck.tier + random.nextInt(2))).get();
            String rule = RULES[(week - 1) % RULES.length];
            Challenge challenge = new Challenge();
            challenge.id = "y" + year + "-w" + String.format("%02d", week);
            challenge.format = format.id;
            challenge.rule = rule;
            challenge.playerDeck = yours.file;
            challenge.opponent = new CareerCampaigns.NodeOpponent();
            challenge.opponent.name = theirs.pilot == null ? theirs.name : theirs.pilot;
            challenge.opponent.deck = theirs.file;
            challenge.opponent.skill = Math.min(8, 3 + theirs.tier);
            challenge.opponent.twists = TWISTS[random.nextInt(TWISTS.length)];
            challenge.name = format.name + ": " + yours.name + " against " + theirs.name;
            challenge.text = "Play " + yours.name + " against " + challenge.opponent.name + "'s " + theirs.name + ". "
                    + ("life".equals(rule) ? "The board ranks wins by the most life left."
                    : "no-creatures".equals(rule) ? "Win without casting a creature spell; the board ranks by fewest turns."
                    : "The board ranks wins by the fewest turns.");
            list.add(challenge);
        }
        return list;
    }

    /** the board a challenge's scores go to, by its rule */
    static String boardMode(Challenge challenge) {
        if ("life".equals(challenge.rule)) {
            return mage.server.solo.Leaderboards.WEEKLY_CHALLENGE_LIFE;
        }
        if ("no-creatures".equals(challenge.rule)) {
            return mage.server.solo.Leaderboards.WEEKLY_CHALLENGE_NO_CREATURES;
        }
        return mage.server.solo.Leaderboards.WEEKLY_CHALLENGE;
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
        view.rule = challenge.rule == null ? "turns" : challenge.rule;
        view.boardMode = boardMode(challenge);
        view.format = challenge.format;
        boolean lifeFirst = "life".equals(view.rule);
        mage.server.solo.Leaderboards.Leaderboard board = mage.server.solo.Leaderboards.get()
                .board(view.boardMode, view.week, user, mage.server.solo.Leaderboards.SCOPE_SERVER, null, BOARD_SIZE);
        if (board.yours != null) {
            view.attempts = board.yours.attempts;
            view.won = board.yours.v1 != null;
            Long turns = lifeFirst ? board.yours.v2 : board.yours.v1;
            Long life = lifeFirst ? board.yours.v1 : board.yours.v2;
            view.bestTurns = turns == null ? null : turns.intValue();
            view.bestLife = life == null ? null : life.intValue();
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
            Long turns = lifeFirst ? line.v2 : line.v1;
            Long life = lifeFirst ? line.v1 : line.v2;
            entry.turns = turns == null ? null : turns.intValue();
            entry.life = life == null ? null : life.intValue();
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
        mage.server.solo.Leaderboards.get().attempt(boardMode(plan.challenge), plan.week, user);
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
        if ("no-creatures".equals(challenge.rule) && tally.get("creature_spells") > 0) {
            result.mode.lines.add("A win, but you cast a creature spell: this week's board only counts wins without one.");
            return;
        }
        mage.server.solo.Leaderboards boards = mage.server.solo.Leaderboards.get();
        String mode = boardMode(challenge);
        boolean first = !boards.hasScore(mode, week, user);
        if ("life".equals(challenge.rule)) {
            boards.submit(mode, week, user, life, (long) turns, null);
        } else {
            boards.submit(mode, week, user, turns, (long) life, null);
        }
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
