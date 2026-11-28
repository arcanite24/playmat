package mage.server;

import com.google.gson.Gson;
import com.google.gson.JsonSyntaxException;
import com.google.gson.annotations.SerializedName;
import org.apache.log4j.Logger;

import java.io.IOException;
import java.io.RandomAccessFile;
import java.nio.charset.StandardCharsets;
import java.nio.file.AtomicMoveNotSupportedException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;
import java.time.Instant;
import java.time.format.DateTimeParseException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.function.LongSupplier;
import java.util.regex.Pattern;

/**
 * The admin console's Self-play section: the Rust AI's self-play worker on this server's host
 * (ops/web/host/rl-worker.sh, docs/rust/Ai.md "Self-play on the public server"). They share a volume
 * ({@value #DIR_ENV}, by default /opt/xmage/selfplay) and nothing else: the worker writes worker.json (its state each
 * run, every 2 minutes), history.jsonl (one line a chunk) and trainer.json (the trainer's progress, sent through the
 * worker's transfer account); the console writes settings.json, which the worker checks field by field and lays over
 * its own settings. The server never touches Docker or the worker's spool.
 */
public final class SelfPlayStatus {

    private static final Logger logger = Logger.getLogger(SelfPlayStatus.class);

    static final String DIR_ENV = "XMAGE_SELFPLAY_DIR";
    static final String DEFAULT_DIR = "/opt/xmage/selfplay";
    /** the worker writes worker.json every 2 minutes, and a chunk takes about as long: older than this, it's stopped */
    static final long STALE_SECONDS = 10 * 60;
    /** history.jsonl lines read (the worker keeps 5,000) */
    static final int HISTORY_LINES = 5000;
    static final int RECENT_CHUNKS = 20;
    static final int HOURS = 24;
    /** the worker's own patterns (rl-worker.sh apply_settings) */
    static final Pattern QUIET = Pattern.compile("^([01][0-9]|2[0-3])(:[0-5][0-9])?-([01][0-9]|2[0-3])(:[0-5][0-9])?$");
    static final int CPU_MIN = 5, CPU_MAX = 200, GAMES_MAX = 50, JOBS_MAX = 3;

    private static final Gson GSON = new Gson();

    // what the worker and the trainer write

    static class WorkerFile {
        String at;
        String state;
        String detail;
        SelfPlayJob job;
        SelfPlaySettings settings;
        Integer spoolMb;
        Integer chunksWaiting;
        SelfPlayTotals totals;
    }

    static class HistoryLine {
        String at;
        String job;
        String chunk;
        String outcome;
        int games;
        int seconds;
        String reason;
    }

    static class TrainerFile {
        String run;
        Integer round;
        Integer rounds;
        String phase;
        String at;
        @SerializedName("local_games")
        Integer localGames;
        @SerializedName("remote_games")
        Integer remoteGames;
        List<RoundLine> history;
    }

    static class RoundLine {
        Integer round;
        Boolean accepted;
        String elo;
        @SerializedName("remote_games")
        Integer remoteGames;
        @SerializedName("selfplay_s")
        Integer selfplayS;
        @SerializedName("train_s")
        Integer trainS;
        @SerializedName("sprt_s")
        Integer sprtS;
    }

    static class SettingsFile {
        Boolean paused;
        @SerializedName("quiet_utc")
        String quietUtc;
        @SerializedName("cpu_max")
        Integer cpuMax;
        @SerializedName("games_max")
        Integer gamesMax;
        Integer jobs;
        String by;
        Long at;
    }

    // what the console gets

    /** The job the worker plays. */
    public static class SelfPlayJob {
        public String id;
        public String image;
        /** the trainer's round */
        public Integer tag;
        public Integer seedBase;
    }

    /** Gate settings: the ones in effect (from the worker) or the console's (settings.json); null fields keep rl.env. */
    public static class SelfPlaySettings {
        public Boolean paused;
        /** HH-HH or HH:MM-HH:MM, UTC; empty for none */
        public String quietUtc;
        /** percent of one core the server may use before the worker stops */
        public Integer cpuMax;
        /** open games before the worker stops */
        public Integer gamesMax;
        /** cores the games run on, 1 to 3 */
        public Integer jobs;
        public Integer chunkGames;
        /** who saved them from the console, and when (epoch seconds); console settings only */
        public String by;
        public Long at;
    }

    public static class SelfPlayTotals {
        public Integer games;
        public Integer chunksDone;
        public Integer chunksStopped;
    }

    /** One chunk the worker finished, threw away or failed. */
    public static class SelfPlayChunk {
        /** epoch seconds */
        public long at;
        public String job;
        public String chunk;
        /** done, stopped or failed */
        public String outcome;
        public int games;
        public int seconds;
        public String reason;
    }

    /** One hour of the last day. */
    public static class SelfPlayHour {
        /** epoch seconds, the hour's start */
        public long start;
        public int games;
        public int chunksDone;
        public int chunksStopped;
    }

    public static class SelfPlayRound {
        public Integer round;
        public Boolean accepted;
        /** the SPRT's Elo and its interval, e.g. "elo +5.2 [-2.6, +13.1]" */
        public String elo;
        public Integer remoteGames;
        public Integer selfplaySeconds;
        public Integer trainSeconds;
        public Integer sprtSeconds;
    }

    /** The trainer's progress, as it last reported it. */
    public static class SelfPlayTrainer {
        public String run;
        public Integer round;
        public Integer rounds;
        /** selfplay, pull, train, sprt or done */
        public String phase;
        /** epoch seconds */
        public Long at;
        public Integer localGames;
        public Integer remoteGames;
        public List<SelfPlayRound> history = new ArrayList<>();
    }

    /** The Self-play section. */
    public static class SelfPlayView {
        /** the shared volume is there: the server's compose file has it */
        public boolean installed;
        /** the worker wrote worker.json in the last few minutes */
        public boolean running;
        /** epoch seconds; null when it never wrote */
        public Long updatedAt;
        /** idle, busy, playing, paused, stopped, pulling or error */
        public String state;
        public String detail;
        public SelfPlayJob job;
        /** what the worker runs with now */
        public SelfPlaySettings effective;
        /** what the console asked for; null when it never saved */
        public SelfPlaySettings console;
        public Integer spoolMb;
        public Integer chunksWaiting;
        public SelfPlayTotals totals;
        public int games24h;
        public int games7d;
        public int chunksDone24h;
        public int chunksStopped24h;
        public int seconds24h;
        /** the last 24 hours, oldest first */
        public List<SelfPlayHour> hours = new ArrayList<>();
        /** the last chunks, newest first */
        public List<SelfPlayChunk> recent = new ArrayList<>();
        public SelfPlayTrainer trainer;
    }

    private static final SelfPlayStatus INSTANCE = new SelfPlayStatus(directory(), () -> System.currentTimeMillis() / 1000);

    public static SelfPlayStatus get() {
        return INSTANCE;
    }

    private static Path directory() {
        String dir = System.getenv(DIR_ENV);
        return Paths.get(dir == null || dir.trim().isEmpty() ? DEFAULT_DIR : dir.trim());
    }

    private final Path dir;
    private final LongSupplier clock;

    SelfPlayStatus(Path dir, LongSupplier clock) {
        this.dir = dir;
        this.clock = clock;
    }

    public SelfPlayView view() {
        SelfPlayView view = new SelfPlayView();
        view.installed = Files.isDirectory(dir);
        if (!view.installed) {
            return view;
        }
        long now = clock.getAsLong();
        WorkerFile worker = read("worker.json", WorkerFile.class);
        if (worker != null) {
            view.updatedAt = epoch(worker.at);
            view.running = view.updatedAt != null && now - view.updatedAt <= STALE_SECONDS;
            view.state = worker.state;
            view.detail = worker.detail;
            view.job = worker.job;
            view.effective = worker.settings;
            view.spoolMb = worker.spoolMb;
            view.chunksWaiting = worker.chunksWaiting;
            view.totals = worker.totals;
        }
        SettingsFile saved = read("settings.json", SettingsFile.class);
        if (saved != null) {
            SelfPlaySettings console = new SelfPlaySettings();
            console.paused = saved.paused;
            console.quietUtc = saved.quietUtc;
            console.cpuMax = saved.cpuMax;
            console.gamesMax = saved.gamesMax;
            console.jobs = saved.jobs;
            console.by = saved.by;
            console.at = saved.at;
            view.console = console;
        }
        summarize(view, history(), now);
        view.trainer = trainer();
        return view;
    }

    /** the day's hours, the week's and day's sums, and the latest chunks */
    static void summarize(SelfPlayView view, List<SelfPlayChunk> chunks, long now) {
        long firstHour = (now / 3600 - (HOURS - 1)) * 3600;
        for (int i = 0; i < HOURS; i++) {
            SelfPlayHour hour = new SelfPlayHour();
            hour.start = firstHour + i * 3600L;
            view.hours.add(hour);
        }
        for (SelfPlayChunk c : chunks) {
            boolean done = "done".equals(c.outcome);
            if (c.at > now - 7 * 86400L && done) {
                view.games7d += c.games;
            }
            if (c.at > now - 86400L) {
                view.seconds24h += c.seconds;
                if (done) {
                    view.games24h += c.games;
                    view.chunksDone24h++;
                } else {
                    view.chunksStopped24h++;
                }
            }
            if (c.at >= firstHour && c.at < firstHour + HOURS * 3600L) {
                SelfPlayHour hour = view.hours.get((int) ((c.at - firstHour) / 3600));
                if (done) {
                    hour.games += c.games;
                    hour.chunksDone++;
                } else {
                    hour.chunksStopped++;
                }
            }
        }
        for (int i = chunks.size() - 1; i >= 0 && view.recent.size() < RECENT_CHUNKS; i--) {
            view.recent.add(chunks.get(i));
        }
    }

    /**
     * Saves the console's settings for the worker (it reads them on its next run, within 2 minutes; a chunk under
     * way stops within 15 seconds of a pause). A null field leaves the worker's own setting (rl.env).
     *
     * @throws IllegalArgumentException with the reason for staff when a value is out of range
     * @throws IllegalStateException when the worker isn't installed or the file can't be written
     */
    public synchronized SelfPlayView save(boolean paused, String quietUtc, Integer cpuMax, Integer gamesMax, Integer jobs, String by) {
        String quiet = quietUtc == null ? null : quietUtc.trim();
        if (quiet != null && !quiet.isEmpty() && !QUIET.matcher(quiet).matches()) {
            throw new IllegalArgumentException("Quiet hours are HH-HH or HH:MM-HH:MM in UTC, like 02-12");
        }
        if (cpuMax != null && (cpuMax < CPU_MIN || cpuMax > CPU_MAX)) {
            throw new IllegalArgumentException("The CPU limit is " + CPU_MIN + " to " + CPU_MAX + " (percent of one core)");
        }
        if (gamesMax != null && (gamesMax < 0 || gamesMax > GAMES_MAX)) {
            throw new IllegalArgumentException("The open games limit is 0 to " + GAMES_MAX);
        }
        if (jobs != null && (jobs < 1 || jobs > JOBS_MAX)) {
            throw new IllegalArgumentException("Cores are 1 to " + JOBS_MAX);
        }
        if (!Files.isDirectory(dir)) {
            throw new IllegalStateException("The self-play worker isn't set up on this server (see the README, Host tools).");
        }
        Map<String, Object> settings = new LinkedHashMap<>();
        settings.put("paused", paused);
        if (quiet != null) {
            settings.put("quiet_utc", quiet);
        }
        if (cpuMax != null) {
            settings.put("cpu_max", cpuMax);
        }
        if (gamesMax != null) {
            settings.put("games_max", gamesMax);
        }
        if (jobs != null) {
            settings.put("jobs", jobs);
        }
        settings.put("by", by);
        settings.put("at", clock.getAsLong());
        try {
            write("settings.json", GSON.toJson(settings));
        } catch (IOException e) {
            logger.warn("Can't write the self-play settings into " + dir + ": " + e.getMessage());
            throw new IllegalStateException("Can't save the settings: " + e.getMessage());
        }
        return view();
    }

    /** history.jsonl's lines as chunks, oldest first; a line that doesn't parse is skipped */
    List<SelfPlayChunk> history() {
        List<SelfPlayChunk> chunks = new ArrayList<>();
        for (String line : tail("history.jsonl", HISTORY_LINES)) {
            try {
                HistoryLine h = GSON.fromJson(line, HistoryLine.class);
                Long at = h == null ? null : epoch(h.at);
                if (at == null) {
                    continue;
                }
                SelfPlayChunk c = new SelfPlayChunk();
                c.at = at;
                c.job = h.job;
                c.chunk = h.chunk;
                c.outcome = h.outcome;
                c.games = Math.max(0, h.games);
                c.seconds = Math.max(0, h.seconds);
                c.reason = h.reason == null || h.reason.isEmpty() ? null : h.reason;
                chunks.add(c);
            } catch (JsonSyntaxException e) {
                // a half-written last line, or junk: skip it
            }
        }
        return chunks;
    }

    private SelfPlayTrainer trainer() {
        TrainerFile file = read("trainer.json", TrainerFile.class);
        if (file == null) {
            return null;
        }
        SelfPlayTrainer trainer = new SelfPlayTrainer();
        trainer.run = file.run;
        trainer.round = file.round;
        trainer.rounds = file.rounds;
        trainer.phase = file.phase;
        trainer.at = epoch(file.at);
        trainer.localGames = file.localGames;
        trainer.remoteGames = file.remoteGames;
        if (file.history != null) {
            for (RoundLine line : file.history) {
                if (line == null) {
                    continue;
                }
                SelfPlayRound round = new SelfPlayRound();
                round.round = line.round;
                round.accepted = line.accepted;
                round.elo = line.elo;
                round.remoteGames = line.remoteGames;
                round.selfplaySeconds = line.selfplayS;
                round.trainSeconds = line.trainS;
                round.sprtSeconds = line.sprtS;
                trainer.history.add(round);
            }
            Collections.reverse(trainer.history);
        }
        return trainer;
    }

    static Long epoch(String iso) {
        if (iso == null || iso.isEmpty()) {
            return null;
        }
        try {
            return Instant.parse(iso).getEpochSecond();
        } catch (DateTimeParseException e) {
            return null;
        }
    }

    private <T> T read(String name, Class<T> type) {
        Path file = dir.resolve(name);
        if (!Files.isRegularFile(file)) {
            return null;
        }
        try {
            return GSON.fromJson(new String(Files.readAllBytes(file), StandardCharsets.UTF_8), type);
        } catch (IOException | JsonSyntaxException e) {
            logger.debug("Can't read " + file + ": " + e.getMessage());
            return null;
        }
    }

    /** the file's last lines, read from its end (history.jsonl stays under a megabyte, but needn't be read whole) */
    private List<String> tail(String name, int lines) {
        Path file = dir.resolve(name);
        if (!Files.isRegularFile(file)) {
            return new ArrayList<>();
        }
        try (RandomAccessFile in = new RandomAccessFile(file.toFile(), "r")) {
            long length = in.length();
            long from = Math.max(0, length - lines * 400L);
            byte[] bytes = new byte[(int) Math.min(length - from, 8L << 20)];
            in.seek(length - bytes.length);
            in.readFully(bytes);
            List<String> all = new ArrayList<>(Arrays.asList(new String(bytes, StandardCharsets.UTF_8).split("\n")));
            if (length - bytes.length > 0 && !all.isEmpty()) {
                all.remove(0); // cut mid-line
            }
            all.removeIf(String::isEmpty);
            return new ArrayList<>(all.subList(Math.max(0, all.size() - lines), all.size()));
        } catch (IOException e) {
            return new ArrayList<>();
        }
    }

    /** written whole and then renamed, so the worker never reads half a file */
    private void write(String name, String text) throws IOException {
        Path tmp = dir.resolve("." + name + ".tmp");
        Files.write(tmp, text.getBytes(StandardCharsets.UTF_8));
        try {
            Files.move(tmp, dir.resolve(name), StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
        } catch (AtomicMoveNotSupportedException e) {
            Files.move(tmp, dir.resolve(name), StandardCopyOption.REPLACE_EXISTING);
        }
    }
}
