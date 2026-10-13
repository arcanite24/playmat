package mage.server.solo;

import mage.game.Game;
import mage.game.advice.Advice;
import mage.game.advice.GameAdvisor;
import mage.server.career.CareerService;
import mage.server.game.GameController;
import mage.server.util.PluginUtil;
import org.apache.log4j.Logger;

import java.util.Set;
import java.util.UUID;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;

/**
 * "What would the AI do?" (S003): the mad AI takes the player's seat in a copy of their game and says what it would
 * do, without doing it. One request per player at a time, a few at once on the server.
 */
public final class AdviceService {

    private static final Logger logger = Logger.getLogger(AdviceService.class);
    private static final AdviceService INSTANCE = new AdviceService();
    private static final String ADVISOR_CLASS = "mage.player.ai.MadAdvisor";

    /** how long the game may take to hand over a copy (the player must be the one it waits for) */
    static final long COPY_TIMEOUT_MS = 10_000;
    /** how long the advisor may think */
    static final long THINK_TIMEOUT_MS = 60_000;
    public static final int DEFAULT_SKILL = 6;

    private final ExecutorService pool = Executors.newFixedThreadPool(2, r -> {
        Thread thread = new Thread(r, "solo-advisor");
        thread.setDaemon(true);
        return thread;
    });
    private final Set<UUID> busy = ConcurrentHashMap.newKeySet();
    private volatile GameAdvisor advisor;

    public static AdviceService get() {
        return INSTANCE;
    }

    /** for tests */
    public void use(GameAdvisor advisor) {
        this.advisor = advisor;
    }

    /**
     * @throws IllegalArgumentException when there's no advice to give, with why
     */
    public Advice advise(GameController controller, UUID userId, int skill) {
        if (!busy.add(userId)) {
            throw new IllegalArgumentException("The advisor is still thinking about your last question.");
        }
        try {
            StringBuilder problem = new StringBuilder();
            UUID[] playerId = new UUID[1];
            CompletableFuture<Game> copy = controller.copyForAdvice(userId, playerId, problem);
            if (copy == null) {
                throw new IllegalArgumentException(problem.toString());
            }
            Game game;
            try {
                game = copy.get(COPY_TIMEOUT_MS, TimeUnit.MILLISECONDS);
            } catch (TimeoutException e) {
                throw new IllegalArgumentException("Ask when the game is waiting for you to act.");
            }
            GameAdvisor current = advisor();
            int level = Math.max(1, Math.min(10, skill));
            CompletableFuture<Advice> thinking = CompletableFuture.supplyAsync(() -> current.advise(game, playerId[0], level), pool);
            try {
                Advice advice = thinking.get(THINK_TIMEOUT_MS, TimeUnit.MILLISECONDS);
                // a Career game counts its hints, and a hint marks the game (S010)
                CareerService.CareerTable career = CareerService.get().tableInfo(controller.getTableId());
                if (career != null) {
                    CareerService.get().assisted(controller.getTableId(), controller.getGameId(), "hint");
                    advice.hintsLeft = Math.max(0, CareerService.HINTS_PER_GAME - career.hintsUsed);
                }
                return advice;
            } catch (TimeoutException e) {
                thinking.cancel(true);
                throw new IllegalArgumentException("The advisor ran out of time on this one.");
            }
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new IllegalArgumentException("The advisor was interrupted.");
        } catch (java.util.concurrent.ExecutionException e) {
            logger.warn("Advisor failed", e.getCause());
            throw new IllegalArgumentException("The advisor couldn't work this position out.");
        } finally {
            busy.remove(userId);
        }
    }

    private GameAdvisor advisor() {
        GameAdvisor current = advisor;
        if (current == null) {
            try {
                Class<?> type = Class.forName(ADVISOR_CLASS, true, PluginUtil.classLoader);
                current = (GameAdvisor) type.getDeclaredConstructor().newInstance();
                advisor = current;
            } catch (ReflectiveOperationException | LinkageError e) {
                logger.warn("The mad AI plugin isn't loaded, so there's no advisor", e);
                throw new IllegalArgumentException("Advice needs the mad AI, which this server doesn't run.");
            }
        }
        return current;
    }
}
