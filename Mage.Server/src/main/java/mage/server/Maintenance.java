package mage.server;

import mage.MageException;
import mage.server.managers.ManagerFactory;
import mage.server.notify.DiscordNotifier;
import mage.util.ThreadUtils;
import org.apache.log4j.Logger;

import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.ScheduledFuture;
import java.util.concurrent.TimeUnit;

/**
 * Planned downtime, announced from /admin. From the announcement on, nobody can start a new table, players are warned
 * as the time comes closer, and the sign-in screen and Home show a banner. With a restart asked for, the server exits
 * once the time has come and no game is running; Docker's restart policy brings it back (on the new image, if one
 * was built). Kept in memory: a restart ends it.
 */
public final class Maintenance {

    private static final Logger logger = Logger.getLogger(Maintenance.class);
    private static final Maintenance INSTANCE = new Maintenance();
    /** minutes before the time when players get another warning */
    private static final long[] WARNINGS = {30, 10, 5, 1};
    private static final long IDLE_CHECK_SECONDS = 20;

    /** What is planned; null fields when nothing is. */
    public static final class MaintenancePlan {
        public long startsAt;
        public String message;
        public boolean restartWhenIdle;
        public String by;
        public long announcedAt;
    }

    private final ScheduledExecutorService timer = Executors.newSingleThreadScheduledExecutor(runnable -> {
        Thread thread = new Thread(runnable, "Maintenance");
        thread.setDaemon(true);
        return thread;
    });
    private volatile MaintenancePlan plan;
    private ScheduledFuture<?> tick;
    private volatile Runnable exit = () -> System.exit(0);

    private Maintenance() {
    }

    public static Maintenance get() {
        return INSTANCE;
    }

    /** For tests: what happens instead of exiting the process. */
    void onExit(Runnable exit) {
        this.exit = exit;
    }

    public MaintenancePlan plan() {
        return plan;
    }

    public boolean isPlanned() {
        return plan != null;
    }

    /**
     * Announces maintenance in {@code minutes} (0: now). A new announcement replaces the old one.
     */
    public synchronized MaintenancePlan announce(ManagerFactory managers, long minutes, String message, boolean restartWhenIdle, String by) {
        cancelTimer();
        MaintenancePlan next = new MaintenancePlan();
        next.announcedAt = System.currentTimeMillis();
        next.startsAt = next.announcedAt + TimeUnit.MINUTES.toMillis(Math.max(0, minutes));
        next.message = message == null ? "" : message.trim();
        next.restartWhenIdle = restartWhenIdle;
        next.by = by;
        plan = next;
        Moderation moderation = new Moderation(managers);
        moderation.broadcast(warning(next, minutes));
        long[] warned = {minutes};
        tick = timer.scheduleAtFixedRate(() -> {
            try {
                tick(managers, moderation, next, warned);
            } catch (RuntimeException e) {
                logger.error("Maintenance check failed", e);
            }
        }, IDLE_CHECK_SECONDS, IDLE_CHECK_SECONDS, TimeUnit.SECONDS);
        logger.info("Maintenance announced by " + by + " for " + minutes + " min" + (restartWhenIdle ? ", restart when idle" : ""));
        DiscordNotifier.staff().staffNews("Maintenance in " + minutes + " min, announced by " + by
                + (restartWhenIdle ? "; the server restarts once no game is running" : "")
                + (next.message.isEmpty() ? "" : ": " + next.message));
        return next;
    }

    public synchronized boolean cancel(ManagerFactory managers) {
        if (plan == null) {
            return false;
        }
        cancelTimer();
        plan = null;
        new Moderation(managers).broadcast("The planned maintenance is off. New tables are open again.");
        return true;
    }

    /** Throws when tables can't be started now. */
    public void refuseNewTables() throws MageException {
        MaintenancePlan current = plan;
        if (current != null) {
            throw new MageException("The server is about to go down for maintenance, so no new tables can start."
                    + (current.message.isEmpty() ? "" : ' ' + current.message));
        }
    }

    private void tick(ManagerFactory managers, Moderation moderation, MaintenancePlan current, long[] warned) {
        if (plan != current) {
            return;
        }
        long left = current.startsAt - System.currentTimeMillis();
        long minutesLeft = (left + TimeUnit.MINUTES.toMillis(1) - 1) / TimeUnit.MINUTES.toMillis(1);
        for (long at : WARNINGS) {
            if (minutesLeft <= at && warned[0] > at && left > 0) {
                warned[0] = at;
                moderation.broadcast(warning(current, at));
                break;
            }
        }
        if (left > 0 || !current.restartWhenIdle) {
            return;
        }
        // Rust-hosted games too: the restart stops the gateway that plays them
        int games = managers.gameManager().getNumberActiveGames() + RustHost.running();
        if (games > 0) {
            return;
        }
        logger.info("Maintenance: no game is running; restarting");
        moderation.broadcast("WARNING: the server restarts for maintenance now. Sign in again in a few minutes.");
        synchronized (this) {
            cancelTimer();
        }
        timer.execute(() -> {
            ThreadUtils.sleep(3000); // time for the warning to arrive
            exit.run();
        });
    }

    private static String warning(MaintenancePlan plan, long minutes) {
        String when = minutes <= 0 ? "now" : "in " + minutes + (minutes == 1 ? " minute" : " minutes");
        String text = "WARNING: server maintenance " + when + ". No new tables can start; games in progress can finish"
                + (plan.restartWhenIdle ? ", then the server restarts." : ".");
        return plan.message.isEmpty() ? text : text + ' ' + plan.message;
    }

    private void cancelTimer() {
        if (tick != null) {
            tick.cancel(false);
            tick = null;
        }
    }
}
