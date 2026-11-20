package mage.server;

import mage.server.career.CareerPayouts;
import mage.server.career.CareerStore;
import mage.server.solo.SoloGames;
import mage.server.solo.SoloStore;
import mage.server.util.SqliteSpace;
import org.apache.log4j.Logger;

import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;

/**
 * Keeps solo.db and career.db from growing without end (they reached 2.6 GB and 760 MB in a month, and their
 * nightly copies filled the disk): once an hour, off the start-up path, old saves and what-if lines go, checked
 * shadow payout events lose their row copies, and the freed pages go back to the disk (the first pass converts each
 * file with one full VACUUM, which takes a few seconds per GB; see {@link SqliteSpace}).
 */
public final class StoreUpkeep {

    private static final Logger logger = Logger.getLogger(StoreUpkeep.class);
    /** after the start-up rush (restored tables, catalog export) and the deploy's health gate */
    static final long FIRST_DELAY_MINUTES = 10;
    static final long EVERY_MINUTES = 60;

    private StoreUpkeep() {
    }

    public static void start(boolean career) {
        ScheduledExecutorService executor = Executors.newSingleThreadScheduledExecutor(runnable -> {
            Thread thread = new Thread(runnable, "Store upkeep");
            thread.setDaemon(true);
            thread.setPriority(Thread.MIN_PRIORITY);
            return thread;
        });
        executor.scheduleWithFixedDelay(() -> runOnce(career), FIRST_DELAY_MINUTES, EVERY_MINUTES, TimeUnit.MINUTES);
    }

    /** one pass; every step on its own, so one failing leaves the others */
    static void runOnce(boolean career) {
        SoloGames.get().prune(true);
        reclaim("solo.db", () -> SoloStore.get().reclaimSpace());
        if (career) {
            try {
                int trimmed = CareerPayouts.upkeep();
                if (trimmed > 0) {
                    logger.info("Career: " + trimmed + " payout events dropped or trimmed");
                }
            } catch (Exception e) {
                logger.warn("Career: payout events couldn't be trimmed", e);
            }
            reclaim("career.db", () -> CareerStore.get().reclaimSpace());
        }
    }

    private interface Reclaim {
        SqliteSpace.Result run() throws Exception;
    }

    private static void reclaim(String name, Reclaim reclaim) {
        try {
            reclaim.run();
        } catch (Exception e) {
            // busy (the gateway writing) or short of disk: the next pass tries again
            logger.warn(name + ": free pages couldn't be reclaimed: " + e.getMessage());
        }
    }
}
