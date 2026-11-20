package mage.server;

import mage.server.notify.DiscordNotifier;
import org.apache.log4j.Logger;

import java.io.File;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;

/**
 * Free space on the disk that holds db/ (the user databases). Low means under -Dxmage.disk.minFreeMB (default
 * {@value #DEFAULT_MIN_FREE_MB}) or under {@value #MIN_FREE_PERCENT}% of the disk, whichever is more. When it's low,
 * solo games stop keeping every turn for what-ifs, /healthz and /admin say so, and the staff channel hears about it
 * (once every {@value #ALERT_EVERY_HOURS} hours at most). A full disk is what broke the server on 2026-11-20.
 */
public final class DiskStatus {

    private static final Logger logger = Logger.getLogger(DiskStatus.class);
    public static final String MIN_FREE_PROP = "xmage.disk.minFreeMB";
    static final long DEFAULT_MIN_FREE_MB = 3072;
    static final int MIN_FREE_PERCENT = 5;
    static final long ALERT_EVERY_HOURS = 6;
    private static final long CACHE_MS = TimeUnit.SECONDS.toMillis(30);

    /** The disk under db/. Sizes are bytes. */
    public static class Disk {
        public long free;
        public long total;
        /** what is wrong, or null when nothing is */
        public String problem;
    }

    private static volatile Disk cached;
    private static volatile long cachedAt;
    private static volatile long lastAlertAt;

    private DiskStatus() {
    }

    /** The disk under db/, read at most every 30 seconds; null when it can't be read. */
    public static Disk read() {
        long now = System.currentTimeMillis();
        Disk disk = cached;
        if (disk == null || now - cachedAt > CACHE_MS) {
            File dir = new File("db");
            disk = read(dir.isDirectory() ? dir : new File("."), minFreeBytes());
            cached = disk;
            cachedAt = now;
        }
        return disk;
    }

    /** True when the disk is low: writes that can wait (what-if turns) should. */
    public static boolean low() {
        Disk disk = read();
        return disk != null && disk.problem != null;
    }

    static Disk read(File dir, long minFreeBytes) {
        long total = dir.getTotalSpace();
        if (total <= 0) {
            return null;
        }
        return describe(dir.getUsableSpace(), total, minFreeBytes);
    }

    static Disk describe(long free, long total, long minFreeBytes) {
        Disk disk = new Disk();
        disk.free = free;
        disk.total = total;
        long floor = Math.max(minFreeBytes, total / 100 * MIN_FREE_PERCENT);
        if (free < floor) {
            disk.problem = "The disk is nearly full: " + mb(free) + " MB free of " + mb(total) + " MB (needs "
                    + mb(floor) + " MB)";
        }
        return disk;
    }

    static long minFreeBytes() {
        return Math.max(0, Long.getLong(MIN_FREE_PROP, DEFAULT_MIN_FREE_MB)) << 20;
    }

    private static long mb(long bytes) {
        return bytes >> 20;
    }

    static void startChecks() {
        ScheduledExecutorService timer = Executors.newSingleThreadScheduledExecutor(runnable -> {
            Thread thread = new Thread(runnable, "Disk check");
            thread.setDaemon(true);
            return thread;
        });
        timer.scheduleAtFixedRate(DiskStatus::check, 1, 10, TimeUnit.MINUTES);
    }

    private static void check() {
        try {
            Disk disk = read();
            long now = System.currentTimeMillis();
            if (disk == null || disk.problem == null || now - lastAlertAt < TimeUnit.HOURS.toMillis(ALERT_EVERY_HOURS)) {
                return;
            }
            lastAlertAt = now;
            logger.warn("Disk: " + disk.problem);
            DiscordNotifier.staff().staffNews("Disk: " + disk.problem + ". Old backups and snapshots are the usual"
                    + " suspects (ops/web/README.md, \"Backups\").");
        } catch (RuntimeException e) {
            logger.error("Disk check failed", e);
        }
    }
}
