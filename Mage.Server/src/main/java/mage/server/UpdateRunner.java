package mage.server;

import com.google.gson.Gson;
import com.google.gson.JsonSyntaxException;
import mage.server.notify.DiscordNotifier;
import org.apache.log4j.Logger;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.AtomicMoveNotSupportedException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;
import java.util.function.LongSupplier;
import java.util.regex.Pattern;

/**
 * The admin console's Update button: asks the updater (ops/web/updater/playmat-updater, the compose service with the
 * Docker socket) to update or roll back, and reads how it's going. They share a volume ({@value #DIR_ENV}, by default
 * /opt/xmage/updates) and nothing else: the server writes request.json, the updater answers in updater.json (its
 * heartbeat) and status.json (the current or last job), and keeps update.log. The server never touches Docker.
 * <p>
 * A request names a tag and nothing more; which images it means is the updater's to say. While the server restarts
 * the status file is all there is, so the console reads the job's end from it after signing in again.
 */
public final class UpdateRunner {

    private static final Logger logger = Logger.getLogger(UpdateRunner.class);

    static final String DIR_ENV = "XMAGE_UPDATES_DIR";
    static final String DEFAULT_DIR = "/opt/xmage/updates";
    /** an updater that hasn't written its heartbeat for this long isn't running */
    static final long HEARTBEAT_SECONDS = 30;
    /** a request the updater hasn't taken in this long is stale (it stopped) and may be replaced */
    static final long REQUEST_SECONDS = 60;
    static final int LOG_LINES = 60;
    /** what the updater accepts (the same pattern as playmat-updater's) */
    static final Pattern TAG = Pattern.compile("^(v[0-9]+\\.[0-9]+\\.[0-9]+(-[A-Za-z0-9.]+)?|sha-[0-9a-f]{7,40}|master|latest)$");

    public static final String UPDATE = "update";
    public static final String ROLLBACK = "rollback";

    private static final Gson GSON = new Gson();
    private static final List<String> BUSY = Arrays.asList("queued", "running");

    /** The updater's heartbeat (updater.json), as it writes it. */
    static class Heartbeat {
        String version;
        String project;
        boolean ready;
        String problem;
        String current;
        String previous;
        boolean running;
        long at;
    }

    /** One update or rollback (status.json), as the updater writes it. */
    public static class UpdateJob {
        public String id;
        /** update or rollback */
        public String action;
        public String tag;
        /** the version it replaced */
        public String from;
        public String by;
        /** queued, running, ok, failed or rolled-back */
        public String state;
        /** pull, backup, drain, restart, rollback, done */
        public String step;
        public String message;
        /** epoch seconds */
        public Long startedAt;
        public Long finishedAt;
    }

    /** The Updates section's view of the updater. */
    public static class UpdaterInfo {
        /** the updater's heartbeat is fresh: it runs */
        public boolean running;
        /** it runs and can update this server */
        public boolean ready;
        /** why it can't, in its words; null when it can or isn't running */
        public String problem;
        public String version;
        /** what the updater last put in place, and what ran before it (a rollback goes back to it); null when none */
        public String current;
        public String previous;
        /** the current or last job; null when there was none */
        public UpdateJob job;
        /** a job is queued or running */
        public boolean busy;
        /** the log's last lines, for staff */
        public List<String> log;
    }

    private static final UpdateRunner INSTANCE = new UpdateRunner(directory(), () -> System.currentTimeMillis() / 1000);

    public static UpdateRunner get() {
        return INSTANCE;
    }

    private static Path directory() {
        String dir = System.getenv(DIR_ENV);
        return Paths.get(dir == null || dir.trim().isEmpty() ? DEFAULT_DIR : dir.trim());
    }

    private final Path dir;
    private final LongSupplier clock;

    UpdateRunner(Path dir, LongSupplier clock) {
        this.dir = dir;
        this.clock = clock;
    }

    public UpdaterInfo info() {
        UpdaterInfo info = new UpdaterInfo();
        Heartbeat heartbeat = read("updater.json", Heartbeat.class);
        long now = clock.getAsLong();
        info.running = heartbeat != null && now - heartbeat.at <= HEARTBEAT_SECONDS;
        if (info.running) {
            info.ready = heartbeat.ready;
            info.problem = heartbeat.problem;
            info.version = heartbeat.version;
            info.current = heartbeat.current;
            info.previous = heartbeat.previous;
        }
        info.job = read("status.json", UpdateJob.class);
        info.busy = info.job != null && BUSY.contains(info.job.state)
                || pendingRequest(now)
                || info.running && heartbeat.running;
        if (info.job != null || info.busy) {
            info.log = tail(LOG_LINES);
        }
        return info;
    }

    /**
     * Asks for an update to {@code tag}, or (null tag, {@link #ROLLBACK}) a rollback.
     *
     * @return the request's id
     * @throws IllegalStateException with the reason for staff when the updater can't take it now
     */
    public synchronized String request(String action, String tag, String by) {
        if (UPDATE.equals(action)) {
            if (tag == null || !TAG.matcher(tag).matches()) {
                throw new IllegalArgumentException("'" + tag + "' isn't a version the updater accepts");
            }
        } else if (!ROLLBACK.equals(action)) {
            throw new IllegalArgumentException("unknown action " + action);
        }
        UpdaterInfo info = info();
        if (!info.running) {
            throw new IllegalStateException("The updater isn't running. Turn it on first (COMPOSE_PROFILES=updater, see the README).");
        }
        if (!info.ready) {
            throw new IllegalStateException(info.problem == null ? "The updater can't update this server." : info.problem);
        }
        if (info.busy) {
            throw new IllegalStateException("An update is already under way.");
        }
        if (ROLLBACK.equals(action) && info.previous == null) {
            throw new IllegalStateException("There's no earlier version to go back to.");
        }
        Map<String, Object> request = new LinkedHashMap<>();
        String id = UUID.randomUUID().toString();
        request.put("id", id);
        request.put("action", action);
        request.put("tag", UPDATE.equals(action) ? tag : null);
        request.put("by", by);
        request.put("at", clock.getAsLong());
        try {
            write("request.json", GSON.toJson(request));
        } catch (IOException e) {
            logger.warn("Can't write the update request into " + dir + ": " + e.getMessage());
            throw new IllegalStateException("Can't reach the updater: " + e.getMessage());
        }
        return id;
    }

    /**
     * Tells the staff Discord channel how each job ended, once: the updater has no network, and the server that asked
     * restarts during the job, so a timer reads the status file and remembers the last job it told about (announced).
     */
    public static void startWatching() {
        ScheduledExecutorService timer = Executors.newSingleThreadScheduledExecutor(runnable -> {
            Thread thread = new Thread(runnable, "Update watch");
            thread.setDaemon(true);
            return thread;
        });
        timer.scheduleAtFixedRate(() -> {
            try {
                String news = INSTANCE.newsOfFinishedJob();
                if (news != null) {
                    DiscordNotifier.staff().staffNews(news);
                }
            } catch (RuntimeException e) {
                logger.warn("Update watch failed", e);
            }
        }, 30, 30, TimeUnit.SECONDS);
    }

    /** the staff news for a job that ended and wasn't told about yet; null when there's none */
    String newsOfFinishedJob() {
        UpdateJob job = read("status.json", UpdateJob.class);
        if (job == null || job.id == null || job.id.isEmpty() || job.finishedAt == null || BUSY.contains(job.state)) {
            return null;
        }
        Path announced = dir.resolve("announced");
        try {
            if (Files.isRegularFile(announced)
                    && job.id.equals(new String(Files.readAllBytes(announced), StandardCharsets.UTF_8).trim())) {
                return null;
            }
            write("announced", job.id);
        } catch (IOException e) {
            // telling twice is better than not at all, but not every 30 seconds: skip until the file can be written
            return null;
        }
        String what = ROLLBACK.equals(job.action) ? "Rollback" : "Update to " + job.tag;
        return what + (job.by == null ? "" : " (asked by " + job.by + ")") + ": "
                + ("ok".equals(job.state) ? "done" : job.state) + ". " + (job.message == null ? "" : job.message);
    }

    /** a request the updater hasn't taken yet, unless it's been waiting so long the updater must have stopped */
    private boolean pendingRequest(long now) {
        Path request = dir.resolve("request.json");
        try {
            return Files.isRegularFile(request) && now - Files.getLastModifiedTime(request).toMillis() / 1000 <= REQUEST_SECONDS;
        } catch (IOException e) {
            return false;
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

    private List<String> tail(int lines) {
        Path file = dir.resolve("update.log");
        try {
            if (!Files.isRegularFile(file)) {
                return new ArrayList<>();
            }
            List<String> all = Files.readAllLines(file, StandardCharsets.UTF_8);
            return new ArrayList<>(all.subList(Math.max(0, all.size() - lines), all.size()));
        } catch (IOException e) {
            return new ArrayList<>();
        }
    }

    /** written whole and then renamed, so the updater never reads half a request */
    private void write(String name, String text) throws IOException {
        Files.createDirectories(dir);
        Path tmp = dir.resolve("." + name + ".tmp");
        Files.write(tmp, text.getBytes(StandardCharsets.UTF_8));
        try {
            Files.move(tmp, dir.resolve(name), StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
        } catch (AtomicMoveNotSupportedException e) {
            Files.move(tmp, dir.resolve(name), StandardCopyOption.REPLACE_EXISTING);
        }
    }
}
