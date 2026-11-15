package mage.server.util;

import org.apache.log4j.Logger;

import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.ScheduledFuture;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.BooleanSupplier;

/**
 * A draft or event starts only once every human has opened it, and the client opens it when the start message
 * arrives (START_DRAFT, START_TOURNAMENT). That message is sent once: lost to a dropped socket, the draft or event
 * waited forever with no sign of why. This resends it to whoever hasn't joined yet, every few seconds, until the
 * draft or event starts or is given up.
 */
public final class JoinReminder {

    private static final Logger logger = Logger.getLogger(JoinReminder.class);

    public static final int EVERY_SECONDS = 10;
    // ten minutes: by then a player who never answers has been dropped by the table health check
    public static final int MAX_REMINDERS = 60;

    private JoinReminder() {
    }

    /**
     * @param done   true once nothing waits for a join any more (started, aborted)
     * @param remind resends the start message to the players who haven't joined
     */
    public static ScheduledFuture<?> start(ScheduledExecutorService executor, String what, BooleanSupplier done, Runnable remind) {
        AtomicInteger sent = new AtomicInteger();
        ScheduledFuture<?>[] self = new ScheduledFuture<?>[1];
        Runnable task = () -> {
            try {
                if (done.getAsBoolean() || sent.incrementAndGet() > MAX_REMINDERS) {
                    if (self[0] != null) {
                        self[0].cancel(false);
                    }
                    return;
                }
                remind.run();
            } catch (Exception e) {
                logger.error("Join reminder for " + what + " failed", e);
            }
        };
        self[0] = executor.scheduleAtFixedRate(task, EVERY_SECONDS, EVERY_SECONDS, TimeUnit.SECONDS);
        return self[0];
    }
}
