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
