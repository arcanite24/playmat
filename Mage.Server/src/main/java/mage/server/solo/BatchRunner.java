package mage.server.solo;

import com.google.gson.Gson;
import mage.cards.decks.Deck;
import mage.cards.decks.DeckCardLists;
import mage.constants.PhaseStep;
import mage.constants.RangeOfInfluence;
import mage.game.Game;
import mage.game.GameException;
import mage.game.GameOptions;
import mage.game.match.Match;
import mage.game.match.MatchOptions;
import mage.players.Player;
import mage.players.PlayerType;
import mage.server.admin.RuntimeSettings;
import mage.server.game.GameFactory;
import mage.server.game.PlayerFactory;
import mage.server.replay.ReplayRecorder;
import mage.server.replay.ReplayStore;
import mage.util.RandomUtil;
import mage.util.ThreadUtils;
import org.apache.log4j.Logger;

import java.sql.SQLException;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;

/**
 * AI-against-AI games in the background (S004): a player (or a mode, such as a deck simulator) asks for so many
 * games between two decks, and the runner plays them, a few at a time, and keeps score. Every game is seeded, so a
 * surprising one can be played again. Jobs live in {@link SoloStore}: they survive a restart and carry on.
 * <p>
 * Staff set how many games run at once (0 stops the runner) and how many games a player may ask for in a day.
 */
public final class BatchRunner {

    private static final Logger logger = Logger.getLogger(BatchRunner.class);
    private static final BatchRunner INSTANCE = new BatchRunner();
    private static final Gson GSON = new Gson();

    public static final String QUEUED = "queued";
    public static final String RUNNING = "running";
    public static final String DONE = "done";
    public static final String CANCELLED = "cancelled";
    public static final String FAILED = "failed";

    public static final int MAX_GAMES_PER_JOB = 500;
    public static final int MAX_TURNS = 60;
    /** a player's jobs waiting or running at once */
    public static final int MAX_OPEN_JOBS = 3;
    static final long GAME_TIMEOUT_MS = TimeUnit.MINUTES.toMillis(15);

    /** What to play. */
    public static class JobSpec {
        public String nameA = "Deck A";
        public String nameB = "Deck B";
        public DeckCardLists deckA;
        public DeckCardLists deckB;
        public int skillA = 4;
        public int skillB = 4;
        /** each side's AI (S044): COMPUTER_MAD (the default) or COMPUTER_MONTE_CARLO */
        public String aiA;
        public String aiB;
        /** keep the positions where a player found lethal as puzzles (S031) */
        public boolean minePuzzles;
        public int games = 10;
        /** the first game's seed; game n plays seed + n. Null: one is picked */
        public Long seed;
        /** a game still going at this turn is a draw */
        public int maxTurns = 30;
        public String gameType = "Two Player Duel";
        public String deckType = "Constructed - Freeform";
        /** keep the replays of this many of the closest games (the matchup lab); 0 records none */
        public int keepReplays;
    }

    /** replays a job may keep */
    public static final int MAX_KEPT_REPLAYS = 5;
    /** the AIs a batch can seat */
    public static final List<String> AI_TYPES = java.util.Arrays.asList(PlayerType.COMPUTER_MAD.name(), PlayerType.COMPUTER_MONTE_CARLO.name());

    /** One game's outcome. */
    public static class GameRecord {
        public int n;
        public long seed;
        /** "A", "B", "draw" or "error" */
        public String winner;
        public int turns;
        /** who played first: "A" or "B" */
        public String first;
        /** life totals when it ended */
        public int lifeA;
        public int lifeB;
        /** the recorded game, while it's one of the job's closest */
        public String replayId;
        /** puzzles the miner kept from this game */
        public int mined;

        /** how close it was: the smaller, the closer (a draw at the turn limit is close; a blowout isn't) */
        public int margin() {
            return "draw".equals(winner) ? 0 : Math.abs(lifeA - lifeB);
        }
    }

    /** The score so far. */
    public static class JobResult {
        public int winsA;
        public int winsB;
        public int draws;
        public int errors;
        public long turns;
        /** games A played first, and won of those */
        public int aFirst;
        public int aFirstWins;
        /** puzzles the miner kept (S031) */
        public int mined;
        public final List<GameRecord> games = new ArrayList<>();
        /** the closest games, whose replays are kept */
        public final List<GameRecord> closest = new ArrayList<>();

        int played() {
            return games.size();
        }
    }

    /** a job's live state while it runs */
    private static final class Running {
        final SoloStore.JobRow row;
        final JobSpec spec;
        final JobResult result;
        final AtomicInteger next;
        final AtomicInteger inFlight = new AtomicInteger();
        final AtomicBoolean cancelled = new AtomicBoolean();

        Running(SoloStore.JobRow row, JobSpec spec, JobResult result) {
            this.row = row;
            this.spec = spec;
            this.result = result;
            this.next = new AtomicInteger(result.played());
        }
    }

    private final Map<String, Running> running = new ConcurrentHashMap<>();
    private final AtomicInteger gamesInFlight = new AtomicInteger();
    private final ExecutorService games = Executors.newCachedThreadPool(r -> {
        Thread thread = new Thread(r, ThreadUtils.THREAD_PREFIX_GAME + " batch");
        thread.setDaemon(true);
        return thread;
    });
    private volatile ScheduledExecutorService ticker;
    /** for tests: plays one game instead of the engine */
    private volatile GamePlayer player = BatchRunner::playGame;

    interface GamePlayer {
        GameRecord play(JobSpec spec, int n, long seed, String owner) throws Exception;
    }

    public static BatchRunner get() {
        return INSTANCE;
    }

    void use(GamePlayer player) {
        this.player = player;
    }

    /** starts the runner: jobs that were running when the server stopped carry on */
    public synchronized void start() {
        if (ticker != null) {
            return;
        }
        try {
            for (SoloStore.JobRow row : SoloStore.get().jobsWithStatus(RUNNING)) {
                SoloStore.get().updateJob(row.id, QUEUED, null, null, row.done, row.result, null);
            }
        } catch (SQLException | RuntimeException e) {
            logger.warn("Batch: unfinished jobs couldn't be queued again", e);
        }
        ticker = Executors.newSingleThreadScheduledExecutor(r -> {
            Thread thread = new Thread(r, "batch-runner");
            thread.setDaemon(true);
            return thread;
        });
        ticker.scheduleWithFixedDelay(this::tick, 2, 2, TimeUnit.SECONDS);
    }

    /**
     * Queues a job.
     *
     * @throws IllegalArgumentException when the spec or the player's quota says no
     */
    public SoloStore.JobRow submit(String user, String kind, JobSpec spec) throws SQLException {
        check(spec);
        int perDay = RuntimeSettings.get().batchGamesPerDay();
        int today = SoloStore.get().gamesSince(user, System.currentTimeMillis() - TimeUnit.DAYS.toMillis(1));
        if (today + spec.games > perDay) {
            throw new IllegalArgumentException("That's more batch games than one player may ask for in a day (" + perDay
                    + "; " + Math.max(0, perDay - today) + " left).");
        }
        long open = SoloStore.get().jobs(user, 50).stream()
                .filter(job -> QUEUED.equals(job.status) || RUNNING.equals(job.status)).count();
        if (open >= MAX_OPEN_JOBS) {
            throw new IllegalArgumentException("You have " + open + " batches waiting already; let one finish first.");
        }
        if (spec.seed == null) {
            spec.seed = RandomUtil.getRandom().nextLong() & 0xFFFFFFFFFFFFL;
        }
        SoloStore.JobRow row = new SoloStore.JobRow();
        row.id = UUID.randomUUID().toString();
        row.user = user;
        row.kind = kind == null || kind.isEmpty() ? "versus" : kind;
        row.created = System.currentTimeMillis();
        row.status = QUEUED;
        row.spec = GSON.toJson(spec);
        row.games = spec.games;
        SoloStore.get().insertJob(row);
        return row;
    }

    /** @return false when there's no such open job (for that user; null user: staff) */
    public boolean cancel(String id, String user) throws SQLException {
        SoloStore.JobRow row = SoloStore.get().job(id);
        if (row == null || (user != null && !user.equals(row.user))) {
            return false;
        }
        if (!QUEUED.equals(row.status) && !RUNNING.equals(row.status)) {
            return false;
        }
        Running job = running.get(id);
        if (job != null) {
            job.cancelled.set(true);
        }
        SoloStore.get().updateJob(id, CANCELLED, null, System.currentTimeMillis(), row.done, row.result, null);
        return true;
    }

    public int gamesInFlight() {
        return gamesInFlight.get();
    }

    public static JobResult result(SoloStore.JobRow row) {
        if (row.result == null || row.result.isEmpty()) {
            return new JobResult();
        }
        return GSON.fromJson(row.result, JobResult.class);
    }

    public static JobSpec spec(SoloStore.JobRow row) {
        return GSON.fromJson(row.spec, JobSpec.class);
    }

    static void check(JobSpec spec) {
        if (spec == null || spec.deckA == null || spec.deckB == null) {
            throw new IllegalArgumentException("A batch needs two decks.");
        }
        if (spec.games < 1 || spec.games > MAX_GAMES_PER_JOB) {
            throw new IllegalArgumentException("A batch plays 1 to " + MAX_GAMES_PER_JOB + " games.");
        }
        if (spec.maxTurns < 1 || spec.maxTurns > MAX_TURNS) {
            throw new IllegalArgumentException("Games stop at turn 1 to " + MAX_TURNS + ".");
        }
        spec.keepReplays = Math.max(0, Math.min(MAX_KEPT_REPLAYS, spec.keepReplays));
        spec.skillA = Math.max(1, Math.min(10, spec.skillA));
        spec.skillB = Math.max(1, Math.min(10, spec.skillB));
        for (String ai : new String[]{spec.aiA, spec.aiB}) {
            if (ai != null && !AI_TYPES.contains(ai)) {
                throw new IllegalArgumentException("A batch's AI is " + String.join(" or ", AI_TYPES) + ".");
            }
        }
        if (!"Two Player Duel".equals(spec.gameType)) {
            throw new IllegalArgumentException("Batches play two-player duels for now.");
        }
        for (DeckCardLists deck : new DeckCardLists[]{spec.deckA, spec.deckB}) {
            int cards = deck.getCards().stream().mapToInt(card -> card.getAmount()).sum();
            if (cards < 1 || cards > 250) {
                throw new IllegalArgumentException("Each deck needs 1 to 250 cards.");
            }
        }
    }

    // ---- running

    /** one turn of the runner: finish what ended, start what fits */
    void tick() {
        try {
            int limit = RuntimeSettings.get().batchConcurrency();
            while (gamesInFlight.get() < limit) {
                Running job = nextJob();
                if (job == null) {
                    return;
                }
                int n = job.next.getAndIncrement();
                job.inFlight.incrementAndGet();
                gamesInFlight.incrementAndGet();
                games.submit(() -> playOne(job, n));
            }
        } catch (Throwable e) {
            logger.warn("Batch: the runner stumbled", e);
        }
    }

    private Running nextJob() throws SQLException {
        for (Running job : running.values()) {
            if (!job.cancelled.get() && job.next.get() < job.spec.games) {
                return job;
            }
        }
        for (SoloStore.JobRow row : SoloStore.get().jobsWithStatus(QUEUED)) {
            if (running.containsKey(row.id)) {
                continue;
            }
            JobSpec spec;
            try {
                spec = spec(row);
                check(spec);
            } catch (RuntimeException e) {
                SoloStore.get().updateJob(row.id, FAILED, null, System.currentTimeMillis(), row.done, row.result, e.getMessage());
                continue;
            }
            Running job = new Running(row, spec, result(row));
            if (job.next.get() >= spec.games) {
                finish(job);
                continue;
            }
            running.put(row.id, job);
            SoloStore.get().updateJob(row.id, RUNNING, System.currentTimeMillis(), null, job.result.played(), GSON.toJson(job.result), null);
            return job;
        }
        return null;
    }

    private void playOne(Running job, int n) {
        GameRecord record;
        try {
            if (job.cancelled.get()) {
                return;
            }
            record = player.play(job.spec, n, job.spec.seed + n, job.row.user);
        } catch (Throwable e) {
            logger.warn("Batch: game " + n + " of job " + job.row.id + " failed", e);
            record = new GameRecord();
            record.n = n;
            record.seed = job.spec.seed + n;
            record.winner = "error";
        } finally {
            gamesInFlight.decrementAndGet();
        }
        boolean last;
        synchronized (job) {
            if (record != null && !job.cancelled.get()) {
                job.result.games.add(record);
                switch (record.winner) {
                    case "A":
                        job.result.winsA++;
                        break;
                    case "B":
                        job.result.winsB++;
                        break;
                    case "draw":
                        job.result.draws++;
                        break;
                    default:
                        job.result.errors++;
                }
                job.result.turns += record.turns;
                job.result.mined += record.mined;
                if ("A".equals(record.first)) {
                    job.result.aFirst++;
                    if ("A".equals(record.winner)) {
                        job.result.aFirstWins++;
                    }
                }
                keepIfClose(job, record);
            } else if (record != null && record.replayId != null) {
                ReplayStore.get().remove(record.replayId);
            }
            last = job.inFlight.decrementAndGet() == 0 && (job.cancelled.get() || job.next.get() >= job.spec.games);
            try {
                if (!job.cancelled.get() && !last) {
                    SoloStore.get().updateJob(job.row.id, RUNNING, null, null, job.result.played(), GSON.toJson(job.result), null);
                }
            } catch (SQLException e) {
                logger.warn("Batch: progress couldn't be saved", e);
            }
        }
        if (last) {
            finish(job);
        }
    }

    /** a recorded game stays while it's one of the job's closest; the one it pushes out goes */
    private static void keepIfClose(Running job, GameRecord record) {
        if (record.replayId == null) {
            return;
        }
        if ("error".equals(record.winner) || job.spec.keepReplays <= 0) {
            ReplayStore.get().remove(record.replayId);
            record.replayId = null;
            return;
        }
        List<GameRecord> closest = job.result.closest;
        closest.add(record);
        closest.sort((a, b) -> a.margin() != b.margin() ? Integer.compare(a.margin(), b.margin()) : Integer.compare(b.turns, a.turns));
        while (closest.size() > job.spec.keepReplays) {
            GameRecord out = closest.remove(closest.size() - 1);
            ReplayStore.get().remove(out.replayId);
            out.replayId = null;
        }
    }

    private void finish(Running job) {
        running.remove(job.row.id);
        if (job.cancelled.get()) {
            return;
        }
        job.result.games.sort((a, b) -> Integer.compare(a.n, b.n));
        try {
            SoloStore.get().updateJob(job.row.id, DONE, null, System.currentTimeMillis(), job.result.played(), GSON.toJson(job.result), null);
        } catch (SQLException e) {
            logger.warn("Batch: a finished job couldn't be saved", e);
        }
    }

    /** waits until nothing runs (tests) */
    void drain(long timeoutMs) throws InterruptedException {
        long until = System.currentTimeMillis() + timeoutMs;
        while (System.currentTimeMillis() < until) {
            tick();
            if (running.isEmpty() && gamesInFlight.get() == 0) {
                return;
            }
            Thread.sleep(50);
        }
    }

    /** one seeded game between the two decks; even games A plays first, odd ones B */
    static GameRecord playGame(JobSpec spec, int n, long seed, String owner) throws Exception {
        MatchOptions options = new MatchOptions("batch", spec.gameType, false);
        options.setDeckType(spec.deckType);
        Match match = GameFactory.instance.createMatch(spec.gameType, options);
        if (match == null) {
            throw new GameException("No game type " + spec.gameType);
        }
        PlayerType typeA = spec.aiA == null ? PlayerType.COMPUTER_MAD : PlayerType.valueOf(spec.aiA);
        PlayerType typeB = spec.aiB == null ? PlayerType.COMPUTER_MAD : PlayerType.valueOf(spec.aiB);
        Player a = PlayerFactory.instance.createPlayer(typeA, spec.nameA, RangeOfInfluence.ALL, spec.skillA)
                .orElseThrow(() -> new GameException("The " + typeA + " AI isn't loaded"));
        Player b = PlayerFactory.instance.createPlayer(typeB, spec.nameB.equals(spec.nameA) ? spec.nameB + " 2" : spec.nameB,
                RangeOfInfluence.ALL, spec.skillB).orElseThrow(() -> new GameException("The " + typeB + " AI isn't loaded"));
        match.addPlayer(a, Deck.load(spec.deckA, false, false));
        match.addPlayer(b, Deck.load(spec.deckB, false, false));
        match.startMatch();
        // the match shuffles its seats from the thread's generator: seed that too
        RandomUtil.withRandom(new java.util.Random(seed), () -> {
            match.startGame();
            return null;
        });
        Game game = match.getGame();
        GameOptions gameOptions = new GameOptions();
        gameOptions.testMode = false;
        gameOptions.rollbackTurnsAllowed = false;
        gameOptions.seed = seed;
        gameOptions.stopOnTurn = spec.maxTurns + 1;
        gameOptions.stopAtStep = PhaseStep.UNTAP;
        game.setGameOptions(gameOptions);

        GameRecord record = new GameRecord();
        record.n = n;
        record.seed = seed;
        ReplayRecorder recorder = spec.keepReplays > 0
                ? ReplayRecorder.startBatch(game, spec.nameA + " vs " + spec.nameB + " #" + (n + 1), owner) : null;
        if (recorder != null) {
            record.replayId = recorder.gameId();
        }
        PuzzleMiner miner = spec.minePuzzles ? PuzzleMiner.attach(game) : null;
        boolean aFirst = n % 2 == 0;
        record.first = aFirst ? "A" : "B";
        AtomicReference<Throwable> failure = new AtomicReference<>();
        Thread thread = new Thread(() -> {
            try {
                game.start(aFirst ? a.getId() : b.getId());
            } catch (Throwable e) {
                failure.set(e);
            }
        }, ThreadUtils.THREAD_PREFIX_GAME + " batch " + n);
        thread.setDaemon(true);
        thread.start();
        thread.join(GAME_TIMEOUT_MS);
        record.turns = game.getTurnNum();
        record.lifeA = game.getPlayer(a.getId()).getLife();
        record.lifeB = game.getPlayer(b.getId()).getLife();
        if (recorder != null) {
            recorder.finish(game.getWinner());
        }
        if (thread.isAlive()) {
            game.setGameStopped(true);
            thread.interrupt();
            record.winner = "error";
            return record;
        }
        if (miner != null && failure.get() == null) {
            record.mined = miner.keep(owner, spec.nameA + " vs " + spec.nameB);
        }
        if (failure.get() != null) {
            logger.warn("Batch: game " + n + " failed", failure.get());
            record.winner = "error";
        } else if (game.getPlayer(a.getId()).hasWon()) {
            record.winner = "A";
        } else if (game.getPlayer(b.getId()).hasWon()) {
            record.winner = "B";
        } else {
            record.winner = "draw";
        }
        return record;
    }
}
