package mage.server.career;

import com.google.gson.Gson;
import org.apache.log4j.Logger;

import java.sql.SQLException;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Where a finished game's payout is made (docs/rust/Payouts.md): {@code xmage.career.payouts}
 * ({@code XMAGE_ROUTE_CAREERPAYOUTS}, passed by the server image only with the gateway on) is {@code java} (the
 * default: Java pays and keeps the event), {@code shadow} (Java pays and keeps, with the event, the rows before and
 * after, so the gateway replays the payout on a copy and compares) or {@code rust} (Java keeps the event and the
 * gateway pays it, for the payouts Rust knows: roster duels and every game outside Career tables; Java pays the
 * modes).
 */
public final class CareerPayouts {

    private static final Logger logger = Logger.getLogger(CareerPayouts.class);

    public static final String PROP = "xmage.career.payouts";
    static final String JAVA = "java";
    static final String SHADOW = "shadow";
    static final String RUST = "rust";
    /** paid events are kept this long, for the shadow comparison and support */
    static final long KEEP_MILLIS = 30L * 24 * 60 * 60 * 1000;
    /** how long the rewards screen waits for a payout Rust hasn't made yet */
    static final long WAIT_MILLIS = 5000;

    private static final ThreadLocal<CareerPayoutEvent> PAYING = new ThreadLocal<>();
    private static volatile String routeForTests;

    private CareerPayouts() {
    }

    /** java, shadow or rust */
    static String route() {
        String route = routeForTests != null ? routeForTests : System.getProperty(PROP, JAVA);
        route = route == null ? JAVA : route.trim().toLowerCase(java.util.Locale.ROOT);
        return SHADOW.equals(route) || RUST.equals(route) ? route : JAVA;
    }

    /** for tests: the route, or null for the property's */
    static void useRoute(String route) {
        routeForTests = route;
    }

    /** whether Rust pays this event on the rust route: roster duels and every game outside Career tables */
    static boolean rustPays(CareerPayoutEvent event) {
        return CareerPayoutEvent.GAME_ENDED.equals(event.event)
                || (event.career != null && "duel".equals(event.career.kind));
    }

    /** whether the route leaves this event to Rust */
    static boolean waitsForRust(CareerPayoutEvent event) {
        return RUST.equals(route()) && rustPays(event);
    }

    /** the event being paid on this thread, whose day and settings the payout uses; null outside a payout */
    static CareerPayoutEvent paying() {
        return PAYING.get();
    }

    interface Payout<T> {
        T pay(CareerPayoutEvent event) throws SQLException;
    }

    /**
     * Pays the event as the route says: inside one career.db transaction, Java pays it and stores the event (with the
     * rows around it in shadow), or (rust) only stores it, pending, for the gateway.
     *
     * @return what Java paid, or null when the event waits for Rust (or was paid before)
     */
    static <T> T pay(CareerStore store, CareerPayoutEvent event, Payout<T> payout) throws SQLException {
        String route = route();
        Gson gson = new Gson();
        String json = gson.toJson(event);
        if (waitsForRust(event)) {
            store.transaction(() -> store.addPayoutEvent(event.key, event.at, json, "pending", null, null, null));
            return null;
        }
        return store.transaction(() -> {
            boolean shadow = SHADOW.equals(route) && rustPays(event);
            String before = shadow ? gson.toJson(rowsOf(store, event)) : null;
            T paid = paying(event, () -> payout.pay(event));
            String after = shadow ? gson.toJson(rowsOf(store, event)) : null;
            if (!store.addPayoutEvent(event.key, event.at, json, "paid", "java", before, after)) {
                // a pending event (Rust's, from before a switch back to java) paid now
                store.markPayoutPaid(event.key, "java", after);
            }
            return paid;
        });
    }

    private static Map<String, Object> rowsOf(CareerStore store, CareerPayoutEvent event) throws SQLException {
        Map<String, Object> rows = new LinkedHashMap<>(store.snapshot(event.users(), event.day));
        // the nemesis, kept in solo.db under the name as signed in
        if (event.career != null && "duel".equals(event.career.kind)) {
            Map<String, String> nemesis = new LinkedHashMap<>();
            nemesis.put(event.career.user, mage.server.solo.SoloStore.get().state(event.career.user, "career.nemesis"));
            rows.put("nemesis", nemesis);
        }
        return rows;
    }

    static <T> T paying(CareerPayoutEvent event, CareerStore.Work<T> work) throws SQLException {
        CareerPayoutEvent before = PAYING.get();
        PAYING.set(event);
        try {
            return work.run();
        } finally {
            if (before == null) {
                PAYING.remove();
            } else {
                PAYING.set(before);
            }
        }
    }

    /**
     * At start, off the rust route: pays the events Rust left pending (the route was switched back to java), and
     * drops paid events past their keep.
     */
    public static int payPending() {
        if (RUST.equals(route())) {
            return 0;
        }
        CareerStore store = CareerStore.get();
        int paid = 0;
        try {
            store.prunePayoutEvents(System.currentTimeMillis() - KEEP_MILLIS);
            for (Map.Entry<String, String> pending : store.pendingPayouts().entrySet()) {
                CareerPayoutEvent event = new Gson().fromJson(pending.getValue(), CareerPayoutEvent.class);
                if (CareerPayoutEvent.MATCH_ENDED.equals(event.event)) {
                    CareerService.get().payMatch(store, event);
                } else {
                    CareerService.get().payGame(store, event);
                }
                paid++;
            }
        } catch (SQLException | RuntimeException e) {
            logger.error("Career: pending payouts can't be paid", e);
        }
        if (paid > 0) {
            logger.info("Career: paid " + paid + " payouts the gateway left pending");
        }
        return paid;
    }

    /** waits a little while the event a game key belongs to is pending (Rust pays it in a moment) */
    static void awaitPayout(CareerStore store, String gameKey) throws SQLException {
        long until = System.currentTimeMillis() + WAIT_MILLIS;
        while (store.payoutPending(gameKey) && System.currentTimeMillis() < until) {
            try {
                Thread.sleep(100);
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
                return;
            }
        }
    }

    /** the users an event pays, for logs */
    static List<String> users(CareerPayoutEvent event) {
        return event.users();
    }
}
