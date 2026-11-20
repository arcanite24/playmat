package mage.server;

import com.google.gson.Gson;
import com.google.gson.JsonParseException;
import mage.server.notify.DiscordNotifier;
import org.apache.log4j.Logger;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;

/**
 * The nightly backup's last run, as ops/web/backup.sh writes it to db/backup-status.json. A server without the backup
 * service has no such file, and nothing is shown or checked. Once an hour the server looks, and tells the staff channel
 * when the last run failed or the last good backup is more than {@value #STALE_HOURS} hours old (once a day at most).
 */
public final class BackupStatus {

    private static final Logger logger = Logger.getLogger(BackupStatus.class);
    static final Path FILE = Paths.get("db", "backup-status.json");
    static final long STALE_HOURS = 36;
    private static final long ALERT_EVERY_MS = TimeUnit.HOURS.toMillis(24);

    /** One run of backup.sh. Times are epoch seconds. */
    public static class BackupRun {
        public long at;
        public boolean ok;
        public String name;
        public String size;
        public long lastOkAt;
        /** why the last run didn't take a backup (too little disk), or empty */
        public String skipped;
        /** what is wrong, or null when nothing is: filled in by the server */
        public String problem;
    }

    private static volatile long lastAlertAt;

    private BackupStatus() {
    }

    /** The last run, or null when this server has no backup service. */
    public static BackupRun read() {
        return read(FILE, System.currentTimeMillis());
    }

    static BackupRun read(Path file, long now) {
        if (!Files.isRegularFile(file)) {
            return null;
        }
        BackupRun status;
        try {
            status = new Gson().fromJson(new String(Files.readAllBytes(file), StandardCharsets.UTF_8), BackupRun.class);
        } catch (IOException | JsonParseException e) {
            logger.warn("Can't read " + file + ": " + e.getMessage());
            status = null;
        }
        if (status == null) {
            status = new BackupRun();
            status.problem = "The backup status can't be read";
            return status;
        }
        status.problem = problem(status, now);
        return status;
    }

    static String problem(BackupRun status, long now) {
        if (!status.ok && status.skipped != null && !status.skipped.isEmpty()) {
            return "The last backup was skipped: " + status.skipped;
        }
        if (!status.ok) {
            return "The last backup failed" + (status.name == null || status.name.isEmpty() ? "" : " (" + status.name + ")");
        }
        long hours = TimeUnit.MILLISECONDS.toHours(now - TimeUnit.SECONDS.toMillis(status.lastOkAt));
        if (status.lastOkAt <= 0 || hours > STALE_HOURS) {
            return status.lastOkAt <= 0 ? "No backup has worked yet" : "The last good backup is " + hours + " hours old";
        }
        return null;
    }

    static void startChecks() {
        ScheduledExecutorService timer = Executors.newSingleThreadScheduledExecutor(runnable -> {
            Thread thread = new Thread(runnable, "Backup check");
            thread.setDaemon(true);
            return thread;
        });
        timer.scheduleAtFixedRate(BackupStatus::check, 10, 60, TimeUnit.MINUTES);
    }

    private static void check() {
        try {
            BackupRun status = read();
            long now = System.currentTimeMillis();
            if (status == null || status.problem == null || now - lastAlertAt < ALERT_EVERY_MS) {
                return;
            }
            lastAlertAt = now;
            logger.warn("Backups: " + status.problem);
            DiscordNotifier.staff().staffNews("Backups: " + status.problem + ". See the server's backup logs.");
        } catch (RuntimeException e) {
            logger.error("Backup check failed", e);
        }
    }
}
