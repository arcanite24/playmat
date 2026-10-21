package mage.server.websocket.api;

import java.util.ArrayDeque;
import java.util.Deque;
import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.TimeUnit;
import java.util.function.LongSupplier;

/**
 * Web client bridge: how many accounts (or access requests) one client address may make, {@link #PER_HOUR} an hour
 * and {@link #PER_DAY} a day, so a script can't fill the server with accounts. Kept in memory like
 * {@link LoginThrottle}; the address is the one the bridge resolved (X-Forwarded-For from a trusted proxy). Players
 * behind one NAT share it.
 */
final class RegistrationLimiter {

    static final int PER_HOUR = 3;
    static final int PER_DAY = 10;
    private static final long HOUR = TimeUnit.HOURS.toMillis(1);
    private static final long DAY = TimeUnit.DAYS.toMillis(1);

    private final Map<String, Deque<Long>> recent = new HashMap<>();
    private final LongSupplier clock;

    RegistrationLimiter() {
        this(System::currentTimeMillis);
    }

    RegistrationLimiter(LongSupplier clock) {
        this.clock = clock;
    }

    /**
     * Counts one more for this address, when it may have one.
     *
     * @return null when counted; otherwise why not, for the player
     */
    synchronized String take(String address) {
        long now = clock.getAsLong();
        recent.values().removeIf(times -> times.isEmpty() || now - times.peekLast() >= DAY);
        Deque<Long> times = recent.computeIfAbsent(key(address), a -> new ArrayDeque<>());
        while (!times.isEmpty() && now - times.peekFirst() >= DAY) {
            times.pollFirst();
        }
        long lastHour = times.stream().filter(at -> now - at < HOUR).count();
        if (lastHour >= PER_HOUR) {
            return "Too many new accounts from your network in the last hour. Try again later.";
        }
        if (times.size() >= PER_DAY) {
            return "Too many new accounts from your network today. Try again tomorrow.";
        }
        times.addLast(now);
        return null;
    }

    /** Gives back the last one counted, when what it was for failed (a taken name, a wrong field). */
    synchronized void refund(String address) {
        Deque<Long> times = recent.get(key(address));
        if (times != null) {
            times.pollLast();
        }
    }

    private static String key(String address) {
        return address == null ? "" : address;
    }
}
