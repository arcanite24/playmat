package mage.server;

import mage.constants.TableState;
import mage.game.Table;
import mage.game.match.Match;
import mage.players.PlayerType;
import mage.server.managers.ManagerFactory;
import org.junit.jupiter.api.Test;

import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.UUID;
import java.util.concurrent.Callable;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.function.Function;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Regression tests for User watched-game cleanup concurrency, not-started table counting, and atomic user creation.
 */
public class UserTest {

    @Test
    void readyToStartTablesCountTowardNotStartedLimit() {
        ManagerFactory managers = deepProxy(ManagerFactory.class, Collections.emptyMap());
        User user = new User(managers, "Pat", "127.0.0.1", null);

        Table waiting = stubTable(TableState.WAITING);
        Table ready = stubTable(TableState.READY_TO_START);
        Table starting = stubTable(TableState.STARTING);
        Table dueling = stubTable(TableState.DUELING);
        Table finished = stubTable(TableState.FINISHED);

        user.addTable(UUID.randomUUID(), waiting);
        user.addTable(UUID.randomUUID(), ready);
        user.addTable(UUID.randomUUID(), starting);
        user.addTable(UUID.randomUUID(), dueling);
        user.addTable(UUID.randomUUID(), finished);

        assertEquals(3, user.getNumberOfNotStartedTables());
    }

    @Test
    void watchedGamesCleanupHandlesReEntrantModificationWithoutCme() {
        List<UUID> stopped = new ArrayList<>();
        UUID g1 = UUID.randomUUID();
        UUID g2 = UUID.randomUUID();
        UUID g3 = UUID.randomUUID();

        final User[] holder = new User[1];
        Map<String, Function<Object[], Object>> overrides = new HashMap<>();
        overrides.put("stopWatching", args -> {
            UUID gameId = (UUID) args[0];
            stopped.add(gameId);
            // Simulate a concurrent/re-entrant watch callback mutating watchedGames during disconnect cleanup
            holder[0].removeGameWatchInfo(gameId);
            if (gameId.equals(g1)) {
                holder[0].addGameWatchInfo(g3);
            }
            return null;
        });

        ManagerFactory managers = deepProxy(ManagerFactory.class, overrides);
        User user = new User(managers, "Pat", "127.0.0.1", null);
        holder[0] = user;
        user.addGameWatchInfo(g1);
        user.addGameWatchInfo(g2);

        user.onLostConnection(DisconnectReason.DisconnectedByUserButKeepTables);

        assertTrue(stopped.contains(g1));
        assertTrue(stopped.contains(g2));
    }

    @Test
    void createUserPreventsDuplicateLiveUsersUnderConcurrency() throws Exception {
        ManagerFactory managers = deepProxy(ManagerFactory.class, Collections.emptyMap());
        UserManagerImpl userManager = new UserManagerImpl(managers);

        int threads = 8;
        ExecutorService pool = Executors.newFixedThreadPool(threads);
        try {
            for (int round = 0; round < 10; round++) {
                String name = "racer-" + round;
                CountDownLatch start = new CountDownLatch(1);
                List<Callable<Optional<User>>> tasks = new ArrayList<>();
                for (int i = 0; i < threads; i++) {
                    tasks.add(() -> {
                        start.await();
                        return userManager.createUser(name, "127.0.0.1", null);
                    });
                }
                List<Future<Optional<User>>> futures = new ArrayList<>();
                for (Callable<Optional<User>> task : tasks) {
                    futures.add(pool.submit(task));
                }
                start.countDown();
                int created = 0;
                for (Future<Optional<User>> future : futures) {
                    if (future.get().isPresent()) {
                        created++;
                    }
                }
                assertEquals(1, created, "exactly one concurrent createUser may succeed for " + name);
            }
        } finally {
            pool.shutdownNow();
        }
    }

    private static Table stubTable(TableState state) {
        Match match = deepProxy(Match.class, Collections.emptyMap());
        Table table = new Table(
                UUID.randomUUID(),
                "Two Player Duel",
                "Test",
                "Pat",
                null,
                Collections.singletonList(PlayerType.HUMAN),
                t -> {},
                match,
                Collections.emptySet(),
                false
        );
        table.setState(state);
        return table;
    }

    @SuppressWarnings("unchecked")
    private static <T> T deepProxy(Class<T> type, Map<String, Function<Object[], Object>> overrides) {
        return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[]{type}, (proxy, method, args) -> {
            Function<Object[], Object> override = overrides.get(method.getName());
            if (override != null) {
                return override.apply(args == null ? new Object[0] : args);
            }
            if ("equals".equals(method.getName())) {
                return proxy == args[0];
            }
            if ("hashCode".equals(method.getName())) {
                return System.identityHashCode(proxy);
            }
            if ("toString".equals(method.getName())) {
                return type.getSimpleName() + "Proxy";
            }
            Class<?> result = method.getReturnType();
            if (result == Optional.class) {
                return Optional.empty();
            }
            if (result == boolean.class) {
                return false;
            }
            if (result == int.class) {
                return 0;
            }
            if (result == long.class) {
                return 0L;
            }
            if (result.isInterface()) {
                return deepProxy(result, overrides);
            }
            return null;
        });
    }
}
