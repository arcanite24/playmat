package mage.server.career;

import com.google.gson.Gson;
import com.google.gson.reflect.TypeToken;
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
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * The puzzle book: fixed positions to win from this turn (resources/career/puzzles.json). Stars go by attempts:
 * three for a first-try solve, two within three tries, one after that. The first solve pays a few coins.
 */
public final class CareerPuzzles {

    public static final int FIRST_SOLVE_COINS = 15;
    public static final int FIRST_SOLVE_XP = 50;

    /** One puzzle as a player sees it. */
    public static class CareerPuzzle {
        public String id;
        public String name;
        public String text;
        public String hint;
        public int number;
        public int attempts;
        public int stars;
        public boolean solved;
        /** the puzzles before it must be solved first */
        public boolean open;
        public CareerSetup.CareerPuzzleSide you;
        public CareerSetup.CareerPuzzleSide opponent;
        /** "win", "survive" or "protect" */
        public String objective;
        /** for "protect": the permanent to keep */
        public String protect;
        /** the series it belongs to; null for the main book */
        public String series;
    }

    private static volatile CareerPuzzles instance;

    private final Map<String, CareerSetup.Puzzle> puzzles = new LinkedHashMap<>();

    private CareerPuzzles() throws IOException {
        readBuiltIn().forEach(puzzle -> puzzles.put(puzzle.id, puzzle));
        for (CareerSetup.Puzzle puzzle : mage.server.solo.ContentPacks.get().puzzles()) {
            puzzles.putIfAbsent(puzzle.id, puzzle);
        }
    }

    /** the puzzles the server ships with */
    static List<CareerSetup.Puzzle> readBuiltIn() throws IOException {
        InputStream stream = CareerPuzzles.class.getResourceAsStream("/career/puzzles.json");
        if (stream == null) {
            throw new IOException("Missing career resource puzzles.json");
        }
        try (Reader reader = new InputStreamReader(stream, StandardCharsets.UTF_8)) {
            return new Gson().fromJson(reader, new TypeToken<List<CareerSetup.Puzzle>>() {
            }.getType());
        }
    }

    static void forget() {
        instance = null;
    }

    public static CareerPuzzles get() {
        CareerPuzzles current = instance;
        if (current == null) {
            synchronized (CareerPuzzles.class) {
                current = instance;
                if (current == null) {
                    try {
                        current = new CareerPuzzles();
                    } catch (IOException | RuntimeException e) {
                        throw new IllegalStateException("Puzzles can't be read: " + e.getMessage(), e);
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    public Map<String, CareerSetup.Puzzle> puzzles() {
        return Collections.unmodifiableMap(puzzles);
    }

    /** stars for a solve on this attempt (1 is the first) */
    static int stars(int attempt) {
        return attempt <= 1 ? 3 : attempt <= 3 ? 2 : 1;
    }

    public List<CareerPuzzle> puzzles(String user) throws SQLException {
        Map<String, long[]> results = CareerStore.get().puzzleResults(user);
        List<CareerPuzzle> views = new ArrayList<>();
        // puzzles open in order: the next one unsolved, and every one before it; the first three of the book are always
        // open, and the first of each series
        Map<String, Boolean> previousSolved = new LinkedHashMap<>();
        Map<String, Integer> inSeries = new LinkedHashMap<>();
        int number = 0;
        for (CareerSetup.Puzzle puzzle : puzzles.values()) {
            number++;
            String series = puzzle.series == null ? "" : puzzle.series;
            int place = inSeries.merge(series, 1, Integer::sum);
            long[] result = results.get(puzzle.id);
            CareerPuzzle view = new CareerPuzzle();
            view.id = puzzle.id;
            view.name = puzzle.name;
            view.text = puzzle.text;
            view.hint = puzzle.hint;
            view.number = number;
            view.attempts = result == null ? 0 : (int) result[0];
            view.stars = result == null ? 0 : (int) result[1];
            view.solved = result != null && result[2] > 0;
            view.open = place <= (series.isEmpty() ? 3 : 1) || previousSolved.getOrDefault(series, true) || view.solved;
            view.you = puzzle.you;
            view.opponent = puzzle.opponent;
            view.objective = puzzle.objective == null ? "win" : puzzle.objective;
            view.protect = puzzle.protect;
            view.series = puzzle.series;
            previousSolved.put(series, view.solved);
            views.add(view);
        }
        return views;
    }

    /** checks the puzzle may be played, counts the attempt and returns it */
    public CareerSetup.Puzzle attempt(String user, String puzzleId) throws SQLException, CareerService.CareerException {
        CareerSetup.Puzzle puzzle = puzzles.get(puzzleId);
        if (puzzle == null) {
            throw new CareerService.CareerException("There is no puzzle " + puzzleId + ".");
        }
        boolean open = puzzles(user).stream().anyMatch(view -> view.id.equals(puzzleId) && view.open);
        if (!open) {
            throw new CareerService.CareerException("Solve the puzzles before " + puzzle.name + " first.");
        }
        CareerStore.get().puzzleAttempt(user, puzzleId);
        try {
            mage.server.solo.Leaderboards.get().attempt(mage.server.solo.Leaderboards.PUZZLE, puzzleId, user);
        } catch (SQLException | RuntimeException e) {
            org.apache.log4j.Logger.getLogger(CareerPuzzles.class).warn("Puzzle board: an attempt wasn't counted", e);
        }
        return puzzle;
    }

    // ---- the daily puzzle (S029), puzzles from files (S030) and mined puzzles (S031)

    public static final String DAILY_PREFIX = "daily:";
    public static final String CUSTOM_PREFIX = "custom:";
    public static final String MINED_PREFIX = "mined:";
    public static final int DAILY_COINS = 20;
    public static final int DAILY_XP = 40;
    /** the daily's hint shows after this many tries */
    public static final int DAILY_HINT_AFTER = 2;
    /** the user_state row that pins each day's puzzle, so new puzzles in the pool can't change today's */
    private static final String SERVER = "*";

    private java.util.function.Supplier<java.time.LocalDate> clock = () -> java.time.LocalDate.now(java.time.ZoneOffset.UTC);

    void useClock(java.util.function.Supplier<java.time.LocalDate> clock) {
        this.clock = clock;
    }

    public String today() {
        return clock.get().toString();
    }

    /** Today's puzzle as a player sees it: the same for everyone on the server. */
    public static class CareerDailyPuzzle {
        /** the day, 2026-10-13 (UTC) */
        public String date;
        public CareerPuzzle puzzle;
        public int attempts;
        public boolean solved;
        /** tries and seconds of the player's best solve today */
        public Long tries;
        public Long seconds;
        /** solves on the server today */
        public int solvers;
    }

    /**
     * Today's puzzle: drawn from the book, the content packs and mined puzzles of depth two or more. The pool goes in
     * a seeded order each cycle, so no puzzle comes back before every other one has had its day; the first ask of a
     * day pins it.
     */
    public synchronized CareerSetup.Puzzle daily(String date) throws SQLException {
        mage.server.solo.SoloStore solo = mage.server.solo.SoloStore.get();
        String pinned = solo.state(SERVER, "daily:" + date);
        if (pinned != null) {
            return new Gson().fromJson(pinned, CareerSetup.Puzzle.class);
        }
        List<CareerSetup.Puzzle> pool = new ArrayList<>();
        for (CareerSetup.Puzzle puzzle : puzzles.values()) {
            if (puzzle.objective == null || "win".equals(puzzle.objective)) {
                pool.add(puzzle);
            }
        }
        for (mage.server.solo.SoloStore.MinedRow row : solo.mined(mage.server.solo.PuzzleMiner.MIN_DEPTH, 365)) {
            CareerSetup.Puzzle puzzle = new Gson().fromJson(row.puzzle, CareerSetup.Puzzle.class);
            if (puzzle != null) {
                pool.add(puzzle);
            }
        }
        pool.sort(java.util.Comparator.comparing(puzzle -> puzzle.id));
        long day = java.time.LocalDate.parse(date).toEpochDay();
        long cycle = Math.floorDiv(day, pool.size());
        Collections.shuffle(pool, new java.util.Random(cycle * 7919L + 17));
        CareerSetup.Puzzle chosen = pool.get((int) Math.floorMod(day, pool.size()));
        solo.putState(SERVER, "daily:" + date, new Gson().toJson(chosen));
        return chosen;
    }

    public CareerDailyPuzzle dailyView(String user) throws SQLException {
        String date = today();
        CareerSetup.Puzzle puzzle = daily(date);
        CareerDailyPuzzle view = new CareerDailyPuzzle();
        view.date = date;
        mage.server.solo.Leaderboards boards = mage.server.solo.Leaderboards.get();
        mage.server.solo.Leaderboards.Leaderboard board = boards.board(mage.server.solo.Leaderboards.DAILY_PUZZLE, date, user,
                mage.server.solo.Leaderboards.SCOPE_SERVER, null, 1000);
        view.solvers = (int) board.entries.stream().filter(entry -> entry.v1 != null).count();
        if (board.yours != null) {
            view.attempts = board.yours.attempts;
            view.solved = board.yours.v1 != null;
            view.tries = board.yours.v1;
            view.seconds = board.yours.v2;
        }
        view.puzzle = viewOf(puzzle, 0);
        if (view.attempts < DAILY_HINT_AFTER && !view.solved) {
            view.puzzle.hint = null;
        }
        return view;
    }

    /** counts a try at today's puzzle and returns it, with the table ref to finish it by */
    public CareerSetup.Puzzle attemptDaily(String user) throws SQLException {
        String date = today();
        CareerSetup.Puzzle puzzle = daily(date);
        mage.server.solo.Leaderboards.get().attempt(mage.server.solo.Leaderboards.DAILY_PUZZLE, date, user);
        return puzzle;
    }

    private void finishedDaily(CareerStore store, String user, String date, boolean won, CareerProgress.CareerGameResult result, long now, long started) throws SQLException {
        CareerSetup.Puzzle puzzle = daily(date);
        result.mode = new CareerProgress.CareerModeResult("puzzle", "Daily puzzle: " + puzzle.name);
        result.mode.ref = DAILY_PREFIX + date;
        if (!won) {
            result.mode.lines.add("Not this time. Try again: the board counts your tries and your time.");
            return;
        }
        mage.server.solo.Leaderboards boards = mage.server.solo.Leaderboards.get();
        mage.server.solo.SoloStore.ScoreRow row = mage.server.solo.SoloStore.get().score(mage.server.solo.Leaderboards.DAILY_PUZZLE, date,
                user.toLowerCase(java.util.Locale.ENGLISH));
        boolean first = row == null || row.v1 == null;
        int tries = row == null ? 1 : Math.max(1, row.attempts);
        long seconds = Math.max(1, (now - started) / 1000);
        if (first) {
            boards.submit(mage.server.solo.Leaderboards.DAILY_PUZZLE, date, user, tries, seconds, puzzle.name);
            store.addCoins(user, DAILY_COINS);
            store.addProfileInt(user, "xp", DAILY_XP);
            store.addStat(user, "daily_puzzles", 1);
            result.coins += DAILY_COINS;
            result.xp += DAILY_XP;
            result.mode.coins += DAILY_COINS;
            result.mode.xp += DAILY_XP;
        }
        result.mode.stars = stars(tries);
        result.mode.lines.add("Solved in " + tries + (tries == 1 ? " try" : " tries") + " and " + seconds + " seconds."
                + (first ? "" : " Your first solve today is the one on the board."));
    }

    /** a puzzle from a file or the miner: no pay, no board but its own stars */
    public CareerSetup.Puzzle custom(String user, CareerSetup.Puzzle puzzle) throws CareerService.CareerException {
        String problem = mage.server.solo.PuzzleKit.problem(puzzle);
        if (problem != null) {
            throw new CareerService.CareerException(problem);
        }
        puzzle.id = mage.server.solo.PuzzleKit.positionId(puzzle);
        if (puzzle.name == null || puzzle.name.trim().isEmpty()) {
            puzzle.name = "Your puzzle";
        }
        return puzzle;
    }

    /** a mined puzzle by id, counted as a try */
    public CareerSetup.Puzzle mined(String user, String id) throws SQLException, CareerService.CareerException {
        mage.server.solo.SoloStore.MinedRow row = mage.server.solo.SoloStore.get().minedPuzzle(id);
        if (row == null) {
            throw new CareerService.CareerException("There's no mined puzzle " + id + ".");
        }
        mage.server.solo.Leaderboards.get().attempt(mage.server.solo.Leaderboards.PUZZLE, id, user);
        return new Gson().fromJson(row.puzzle, CareerSetup.Puzzle.class);
    }

    /** A mined puzzle in the list. */
    public static class CareerMinedPuzzle {
        public String id;
        public String name;
        public String text;
        public int depth;
        public String source;
        public long created;
        public int attempts;
        public int stars;
    }

    public List<CareerMinedPuzzle> minedList(String user, int limit) throws SQLException {
        List<CareerMinedPuzzle> list = new ArrayList<>();
        for (mage.server.solo.SoloStore.MinedRow row : mage.server.solo.SoloStore.get().mined(mage.server.solo.PuzzleMiner.MIN_DEPTH, limit)) {
            CareerSetup.Puzzle puzzle = new Gson().fromJson(row.puzzle, CareerSetup.Puzzle.class);
            CareerMinedPuzzle view = new CareerMinedPuzzle();
            view.id = row.id;
            view.name = puzzle.name;
            view.text = puzzle.text;
            view.depth = row.depth;
            view.source = row.source;
            view.created = row.created;
            mage.server.solo.SoloStore.ScoreRow score = mage.server.solo.SoloStore.get().score(mage.server.solo.Leaderboards.PUZZLE, row.id,
                    user.toLowerCase(java.util.Locale.ENGLISH));
            if (score != null) {
                view.attempts = score.attempts;
                view.stars = score.v1 == null ? 0 : score.v1.intValue();
            }
            list.add(view);
        }
        return list;
    }

    private void finishedCustom(String user, String ref, boolean won, CareerProgress.CareerGameResult result) throws SQLException {
        boolean mined = ref.startsWith(MINED_PREFIX);
        String id = ref.substring(ref.indexOf(':') + 1);
        result.mode = new CareerProgress.CareerModeResult("puzzle", mined ? "Mined puzzle" : "Your puzzle");
        result.mode.ref = ref;
        if (!won) {
            result.mode.lines.add("Not this time.");
            return;
        }
        if (mined) {
            mage.server.solo.SoloStore.ScoreRow row = mage.server.solo.SoloStore.get().score(mage.server.solo.Leaderboards.PUZZLE, id,
                    user.toLowerCase(java.util.Locale.ENGLISH));
            int tries = row == null ? 1 : Math.max(1, row.attempts);
            mage.server.solo.Leaderboards.get().submit(mage.server.solo.Leaderboards.PUZZLE, id, user, stars(tries), (long) tries, null);
            result.mode.stars = stars(tries);
        }
        result.mode.lines.add("Solved.");
    }

    private static CareerPuzzle viewOf(CareerSetup.Puzzle puzzle, int number) {
        CareerPuzzle view = new CareerPuzzle();
        view.id = puzzle.id;
        view.name = puzzle.name;
        view.text = puzzle.text;
        view.hint = puzzle.hint;
        view.number = number;
        view.open = true;
        view.you = puzzle.you;
        view.opponent = puzzle.opponent;
        view.objective = puzzle.objective == null ? "win" : puzzle.objective;
        view.protect = puzzle.protect;
        view.series = puzzle.series;
        return view;
    }

    /** a deck for the table seat; the puzzle replaces every card when the game starts */
    public static DeckCardLists placeholderDeckFor(String land) {
        DeckCardLists deck = new DeckCardLists();
        deck.setName("Puzzle");
        mage.cards.repository.CardInfo info = mage.cards.repository.CardRepository.instance.findPreferredCoreExpansionCard(land);
        deck.getCards().add(info == null ? new DeckCardInfo(land, null, null, 60)
                : new DeckCardInfo(info.getName(), info.getCardNumber(), info.getSetCode(), 60));
        return deck;
    }

    void finished(CareerStore store, String user, String puzzleId, boolean won, CareerProgress.CareerGameResult result, long now) throws SQLException {
        finished(store, user, puzzleId, won, result, now, now);
    }

    void finished(CareerStore store, String user, String puzzleId, boolean won, CareerProgress.CareerGameResult result, long now, long started) throws SQLException {
        if (puzzleId.startsWith(DAILY_PREFIX)) {
            finishedDaily(store, user, puzzleId.substring(DAILY_PREFIX.length()), won, result, now, started);
            return;
        }
        if (puzzleId.startsWith(CUSTOM_PREFIX) || puzzleId.startsWith(MINED_PREFIX)) {
            finishedCustom(user, puzzleId, won, result);
            return;
        }
        CareerSetup.Puzzle puzzle = puzzles.get(puzzleId);
        if (puzzle == null) {
            return;
        }
        result.mode = new CareerProgress.CareerModeResult("puzzle", puzzle.name);
        result.mode.ref = puzzleId;
        long[] before = store.puzzleResults(user).get(puzzleId);
        int attempts = before == null ? 1 : (int) before[0];
        if (!won) {
            result.mode.lines.add("Not this time. Puzzles can be tried as often as you like.");
            return;
        }
        boolean first = before == null || before[2] == 0;
        int stars = stars(attempts);
        store.puzzleSolved(user, puzzleId, stars, now);
        try {
            // the puzzle's board: most stars, then fewest tries
            mage.server.solo.Leaderboards.get().submit(mage.server.solo.Leaderboards.PUZZLE, puzzleId, user, stars, (long) attempts, null);
        } catch (SQLException | RuntimeException e) {
            org.apache.log4j.Logger.getLogger(CareerPuzzles.class).warn("Puzzle board: a solve wasn't recorded", e);
        }
        result.mode.stars = Math.max(stars, before == null ? 0 : (int) before[1]);
        result.mode.lines.add("Solved" + (attempts <= 1 ? " on the first try." : " in " + attempts + " tries."));
        if (first) {
            store.addCoins(user, FIRST_SOLVE_COINS);
            store.addProfileInt(user, "xp", FIRST_SOLVE_XP);
            store.addStat(user, "puzzles_solved", 1);
            result.coins += FIRST_SOLVE_COINS;
            result.xp += FIRST_SOLVE_XP;
            result.mode.coins += FIRST_SOLVE_COINS;
            result.mode.xp += FIRST_SOLVE_XP;
        }
        if (stars == 3) {
            store.maxStat(user, "puzzle_three_stars_" + puzzleId, 1);
        }
    }
}
