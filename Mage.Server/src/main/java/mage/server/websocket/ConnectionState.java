package mage.server.websocket;

import mage.server.websocket.rpc.RpcConnection;
import org.java_websocket.WebSocket;
import org.jboss.remoting.callback.InvokerCallbackHandler;

import java.util.ArrayDeque;
import java.util.Collections;
import java.util.List;
import java.util.Queue;
import java.util.UUID;
import java.util.concurrent.Executor;
import java.util.concurrent.atomic.AtomicBoolean;

/**
 * Web client bridge: per-connection state (bound session, request queue, rate limit).
 * <p>
 * Requests of one connection run one at a time and in order on the shared worker pool,
 * so slow requests never block the socket threads or other players.
 */
final class ConnectionState implements RpcConnection {

    // token bucket: sustained requests per second and burst size
    private static final double RATE_PER_SECOND = 40.0;
    private static final double BURST = 120.0;

    private final WebSocket conn;
    private final String remoteHost;
    private final Executor workers;
    private final long maxOutgoingBytes;
    private final AtomicBoolean holdsConnectionSlot = new AtomicBoolean();
    private final Queue<Runnable> queue = new ArrayDeque<>();
    private boolean running;

    private volatile String sessionId;
    private volatile String userName;
    /** set once, from the handshake, before the connection's first request */
    private volatile String signedOnUser;
    private volatile WebSocketCallbackHandler callbackHandler;
    private final GameStates gameStates = new GameStates();
    /** one per connection, shared by its logins' handlers, so its messages keep their order */
    private final Executor sender = SerialSender.create();

    private int rateLimitStrikes;
    private double tokens = BURST;
    private long lastRefillNanos = System.nanoTime();

    /**
     * @param remoteHost       client IP (behind a trusted proxy: the forwarded one, see {@link ClientAddressResolver})
     * @param maxOutgoingBytes bytes queued for this client before the connection is dropped
     */
    ConnectionState(WebSocket conn, Executor workers, String remoteHost, long maxOutgoingBytes) {
        this.conn = conn;
        this.workers = workers;
        this.remoteHost = remoteHost == null ? "" : remoteHost;
        this.maxOutgoingBytes = maxOutgoingBytes;
    }

    long getMaxOutgoingBytes() {
        return maxOutgoingBytes;
    }

    void markConnectionSlotHeld() {
        holdsConnectionSlot.set(true);
    }

    /**
     * @return true exactly once if this connection held a per-IP connection slot
     */
    boolean releaseConnectionSlot() {
        return holdsConnectionSlot.compareAndSet(true, false);
    }

    @Override
    public String getSessionId() {
        return sessionId;
    }

    @Override
    public void bindSession(String sessionId) {
        this.sessionId = sessionId;
    }

    @Override
    public String getUserName() {
        return userName;
    }

    @Override
    public void setUserName(String userName) {
        this.userName = userName;
    }

    @Override
    public String getSignedOnUser() {
        return signedOnUser;
    }

    void setSignedOnUser(String signedOnUser) {
        this.signedOnUser = signedOnUser;
    }

    @Override
    public String getRemoteHost() {
        return remoteHost;
    }

    @Override
    public InvokerCallbackHandler createCallbackHandler() {
        synchronized (gameStates) {
            gameStates.newSession();
        }
        WebSocketCallbackHandler handler = new WebSocketCallbackHandler(conn, gameStates, sender);
        callbackHandler = handler;
        return handler;
    }

    @Override
    public List<String> setCapabilities(List<String> requested) {
        boolean stateDiffs = requested.contains(GameStates.CAPABILITY);
        synchronized (gameStates) {
            gameStates.setEnabled(stateDiffs);
        }
        return stateDiffs ? Collections.singletonList(GameStates.CAPABILITY) : Collections.emptyList();
    }

    @Override
    public boolean resendGameState(UUID gameId) {
        WebSocketCallbackHandler handler = callbackHandler;
        return handler != null && handler.resendState(gameId);
    }

    @Override
    public void forgetGameState(UUID gameId) {
        synchronized (gameStates) {
            gameStates.forget(gameId);
        }
    }

    /**
     * The socket closed: game states are no longer needed.
     */
    void closed() {
        synchronized (gameStates) {
            gameStates.newSession();
        }
    }

    @Override
    public void promptAnswered(UUID gameId) {
        WebSocketCallbackHandler handler = callbackHandler;
        if (handler != null) {
            handler.promptAnswered(gameId);
        }
    }

    @Override
    public boolean resendPrompt(UUID gameId) {
        WebSocketCallbackHandler handler = callbackHandler;
        return handler != null && handler.resendPrompt(gameId);
    }

    synchronized boolean tryAcquire() {
        long now = System.nanoTime();
        tokens = Math.min(BURST, tokens + (now - lastRefillNanos) / 1_000_000_000.0 * RATE_PER_SECOND);
        lastRefillNanos = now;
        if (tokens < 1.0) {
            return false;
        }
        tokens -= 1.0;
        return true;
    }

    synchronized int recordRateLimitStrike() {
        return ++rateLimitStrikes;
    }

    /**
     * Runs the task after every earlier task of this connection has finished.
     *
     * @throws java.util.concurrent.RejectedExecutionException when the worker pool is saturated
     */
    void submit(Runnable task) {
        synchronized (queue) {
            queue.add(task);
            if (running) {
                return;
            }
            running = true;
        }
        try {
            workers.execute(this::drain);
        } catch (RuntimeException e) {
            synchronized (queue) {
                queue.remove(task);
                running = false;
            }
            throw e;
        }
    }

    private void drain() {
        while (true) {
            Runnable next;
            synchronized (queue) {
                next = queue.poll();
                if (next == null) {
                    running = false;
                    return;
                }
            }
            next.run();
        }
    }
}
