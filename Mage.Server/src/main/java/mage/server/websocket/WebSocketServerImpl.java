package mage.server.websocket;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import mage.MageException;
import mage.interfaces.MageServer;
import mage.server.DisconnectReason;
import mage.server.managers.ManagerFactory;
import mage.server.websocket.api.ApiContext;
import mage.server.websocket.api.WebClientApi;
import mage.server.websocket.rpc.JsonCodec;
import mage.server.websocket.rpc.RpcDispatcher;
import mage.server.websocket.rpc.RpcException;
import mage.server.websocket.rpc.RpcSessions;
import mage.util.ThreadUtils;
import org.apache.log4j.Logger;
import org.java_websocket.WebSocket;
import org.java_websocket.WebSocketImpl;
import org.java_websocket.drafts.Draft;
import org.java_websocket.drafts.Draft_6455;
import org.java_websocket.exceptions.InvalidDataException;
import org.java_websocket.extensions.permessage_deflate.PerMessageDeflateExtension;
import org.java_websocket.framing.CloseFrame;
import org.java_websocket.handshake.ClientHandshake;
import org.java_websocket.protocols.Protocol;
import org.java_websocket.handshake.ServerHandshakeBuilder;
import org.java_websocket.server.WebSocketServer;

import java.net.InetSocketAddress;
import java.nio.ByteBuffer;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashSet;
import java.util.Locale;
import java.util.Set;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.RejectedExecutionException;
import java.util.concurrent.ThreadPoolExecutor;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.stream.Collectors;

/**
 * Web client bridge: JSON-RPC 2.0 over WebSocket, next to the desktop client's JBoss remoting port.
 * <p>
 * Requests: {@code {"jsonrpc": "2.0", "id": 1, "method": "roomGetAllTables", "params": [...]}} with positional params.
 * Responses: {@code {"jsonrpc": "2.0", "id": 1, "result": ...}} or {@code {..., "error": {"code": -32602, "message": "..."}}}.
 * Server events are pushed by {@link WebSocketCallbackHandler}.
 * <p>
 * See docs/WebSocketAPI.md (generated from the method registry) for every method.
 */
public class WebSocketServerImpl extends WebSocketServer {

    private static final Logger logger = Logger.getLogger(WebSocketServerImpl.class);

    // decks, cubes and options are the biggest requests; anything larger is abuse
    private static final int MAX_MESSAGE_BYTES = 4 * 1024 * 1024;
    private static final int WORKER_THREADS = 32;
    private static final int WORKER_QUEUE = 10_000;
    private static final int MAX_RATE_LIMIT_STRIKES = 200;
    /** close code of a connection whose user logged in again from somewhere else (it must not log back in by itself) */
    static final int CLOSE_SIGNED_IN_ELSEWHERE = 4001;

    private final RpcDispatcher dispatcher;
    private final RpcSessions sessions;
    private final Set<String> allowedOrigins;
    private final WebSocketLimits limits;
    private final ConnectionLimiter connectionLimiter;
    private final ThreadPoolExecutor workers;
    private final CountDownLatch startupLatch = new CountDownLatch(1);
    private volatile Exception startupException;

    public WebSocketServerImpl(InetSocketAddress address, MageServer mageServer, ManagerFactory managerFactory, String allowedOrigins) {
        this(address, mageServer, managerFactory, allowedOrigins, "");
    }

    /**
     * @param adminPassword server admin password; admin login over WebSocket is disabled when it is empty
     */
    public WebSocketServerImpl(InetSocketAddress address, MageServer mageServer, ManagerFactory managerFactory,
                               String allowedOrigins, String adminPassword) {
        this(address, mageServer, managerFactory, allowedOrigins, adminPassword, WebSocketLimits.fromSystemProperties());
    }

    WebSocketServerImpl(InetSocketAddress address, MageServer mageServer, ManagerFactory managerFactory,
                        String allowedOrigins, String adminPassword, WebSocketLimits limits) {
        this(address, mageServer, managerFactory, new ServerRpcSessions(managerFactory), allowedOrigins, adminPassword, limits);
    }

    /**
     * @param sessions the server's sessions as the bridge sees them (tests pass their own)
     */
    WebSocketServerImpl(InetSocketAddress address, MageServer mageServer, ManagerFactory managerFactory, RpcSessions sessions,
                        String allowedOrigins, String adminPassword, WebSocketLimits limits) {
        super(address, Collections.<Draft>singletonList(
                // the empty protocol accepts clients that request no subprotocol (all browsers by default)
                new Draft_6455(Collections.singletonList(new PerMessageDeflateExtension()),
                        Collections.singletonList(new Protocol("")), MAX_MESSAGE_BYTES)));
        this.sessions = sessions;
        this.dispatcher = WebClientApi.createDispatcher(new ApiContext(mageServer, managerFactory, sessions, adminPassword));
        this.allowedOrigins = parseOrigins(allowedOrigins);
        this.limits = limits;
        this.connectionLimiter = new ConnectionLimiter(limits.maxConnectionsPerIp);
        if (!limits.addressResolver.hasTrustedProxies()) {
            logger.info("WebSocket: no trusted reverse proxies (-D" + WebSocketLimits.TRUSTED_PROXIES_PROP
                    + "), client IPs are the TCP peers");
        }

        AtomicInteger threadNumber = new AtomicInteger();
        this.workers = new ThreadPoolExecutor(WORKER_THREADS, WORKER_THREADS, 60L, TimeUnit.SECONDS,
                new LinkedBlockingQueue<>(WORKER_QUEUE),
                runnable -> {
                    Thread thread = new Thread(runnable, "WEB-RPC-" + threadNumber.incrementAndGet());
                    thread.setDaemon(true);
                    return thread;
                });
        this.workers.allowCoreThreadTimeOut(true);
        setReuseAddr(true);
    }

    public RpcDispatcher getDispatcher() {
        return dispatcher;
    }

    private static Set<String> parseOrigins(String origins) {
        if (origins == null || origins.trim().isEmpty() || origins.trim().equals("*")) {
            return Collections.emptySet(); // any origin
        }
        return Arrays.stream(origins.split(","))
                .map(origin -> origin.trim().toLowerCase(Locale.ENGLISH))
                .filter(origin -> !origin.isEmpty())
                .collect(Collectors.toCollection(HashSet::new));
    }

    @Override
    public ServerHandshakeBuilder onWebsocketHandshakeReceivedAsServer(WebSocket conn, Draft draft, ClientHandshake request) throws InvalidDataException {
        ServerHandshakeBuilder builder = super.onWebsocketHandshakeReceivedAsServer(conn, draft, request);
        if (!allowedOrigins.isEmpty()) {
            String origin = request.getFieldValue("Origin");
            // non-browser clients (tests, tools) send no origin and are not subject to cross-site attacks
            if (origin != null && !origin.isEmpty() && !allowedOrigins.contains(origin.toLowerCase(Locale.ENGLISH))) {
                logger.warn("Rejected WebSocket connection from origin " + origin);
                throw new InvalidDataException(CloseFrame.POLICY_VALIDATION, "Origin not allowed");
            }
        }
        return builder;
    }

    @Override
    public void onOpen(WebSocket conn, ClientHandshake handshake) {
        String clientIp = clientIp(conn, handshake);
        ConnectionState state = new ConnectionState(conn, workers, clientIp, limits.maxOutgoingBytes);
        conn.setAttachment(state);
        BridgeMetrics.get().opened(); // every connection with a state is closed through onClose
        if (limits.limitLoopback || !isLoopback(clientIp)) {
            if (!connectionLimiter.tryAcquire(clientIp)) {
                logger.warn("Closing WebSocket connection over the per-IP limit (" + limits.maxConnectionsPerIp + "): " + clientIp);
                conn.close(CloseFrame.POLICY_VALIDATION, "Too many connections from your address");
                return;
            }
            state.markConnectionSlotHeld();
        }
        logger.debug("WebSocket connection opened: " + clientIp + " via " + conn.getRemoteSocketAddress());
    }

    private String clientIp(WebSocket conn, ClientHandshake handshake) {
        InetSocketAddress remote = conn.getRemoteSocketAddress();
        String peer = remote == null || remote.getAddress() == null ? "" : remote.getAddress().getHostAddress();
        return limits.addressResolver.resolve(peer,
                handshake == null ? null : handshake.getFieldValue("X-Forwarded-For"),
                handshake == null ? null : handshake.getFieldValue("X-Real-IP"));
    }

    private static boolean isLoopback(String ip) {
        java.net.InetAddress address = ClientAddressResolver.parseAddress(ip);
        return address != null && address.isLoopbackAddress();
    }

    int openConnectionsFrom(String ip) {
        return connectionLimiter.openConnections(ip);
    }

    @Override
    public void onClose(WebSocket conn, int code, String reason, boolean remote) {
        ConnectionState state = conn.getAttachment();
        logger.debug("WebSocket connection closed: " + describe(conn) + " (" + code + ")");
        if (state == null) {
            return;
        }
        BridgeMetrics.get().closed();
        state.closed();
        if (state.releaseConnectionSlot()) {
            connectionLimiter.release(state.getRemoteHost());
        }
        // same as a dropped desktop connection: the user keeps tables for a while and can reconnect
        state.submit(() -> {
            String sessionId = state.getSessionId();
            if (sessionId != null && sessions.exists(sessionId)) {
                sessions.disconnect(sessionId, DisconnectReason.LostConnection);
            }
            state.bindSession(null);
        });
    }

    @Override
    public void onMessage(WebSocket conn, String message) {
        ConnectionState state = conn.getAttachment();
        if (state == null) {
            return;
        }
        BridgeMetrics.get().request();

        JsonObject request;
        try {
            request = JsonParser.parseString(message).getAsJsonObject();
        } catch (RuntimeException e) {
            sendError(conn, null, new RpcException(RpcException.PARSE_ERROR, "Request is not a JSON object"));
            return;
        }
        JsonElement id = request.get("id");

        if (!state.tryAcquire()) {
            sendError(conn, id, new RpcException(RpcException.RATE_LIMITED, "Too many requests, slow down"));
            if (state.recordRateLimitStrike() > MAX_RATE_LIMIT_STRIKES) {
                logger.warn("Closing WebSocket connection over request rate limit: " + describe(conn));
                conn.close(CloseFrame.POLICY_VALIDATION, "Request rate limit exceeded");
            }
            return;
        }

        if (!request.has("method") || !request.get("method").isJsonPrimitive()) {
            sendError(conn, id, new RpcException(RpcException.INVALID_REQUEST, "Request has no method"));
            return;
        }
        String method = request.get("method").getAsString();
        JsonElement paramsJson = request.get("params");
        if (paramsJson != null && !paramsJson.isJsonNull() && !paramsJson.isJsonArray()) {
            sendError(conn, id, new RpcException(RpcException.INVALID_REQUEST, "params must be an array"));
            return;
        }
        JsonArray params = paramsJson == null || paramsJson.isJsonNull() ? new JsonArray() : paramsJson.getAsJsonArray();

        try {
            state.submit(() -> execute(conn, state, id, method, params));
        } catch (RejectedExecutionException e) {
            sendError(conn, id, new RpcException(RpcException.SERVER_BUSY, "Server is busy, try again"));
        }
    }

    private void execute(WebSocket conn, ConnectionState state, JsonElement id, String method, JsonArray params) {
        try {
            Object result = dispatcher.dispatch(method, params, state);
            if ("connectUser".equals(method) && Boolean.TRUE.equals(result) && state.getUserName() != null) {
                closeReplaced(state);
            }
            if (id != null && !id.isJsonNull() && conn.isOpen()) {
                JsonObject response = new JsonObject();
                response.addProperty("jsonrpc", "2.0");
                response.add("id", id);
                response.add("result", JsonCodec.GSON.toJsonTree(result));
                sendText(conn, JsonCodec.GSON.toJson(response));
            }
        } catch (RpcException e) {
            sendError(conn, id, e);
        } catch (MageException e) {
            // never log request params: they can contain passwords
            logger.warn("WebSocket RPC " + method + " failed: " + e.getMessage());
            sendError(conn, id, new RpcException(RpcException.SERVER_ERROR, safeMessage(e)));
        } catch (Throwable e) {
            logger.error("WebSocket RPC " + method + " failed: " + ThreadUtils.findRootException(e), e);
            sendError(conn, id, new RpcException(RpcException.INTERNAL_ERROR, safeMessage(e)));
        }
    }

    /**
     * The login took its user over from this server's other connections that were logged in as it (another tab or
     * device): the server already ended their sessions, and now they learn why. They are closed with
     * {@link #CLOSE_SIGNED_IN_ELSEWHERE} instead of being left to find out on their next call, and clients don't log
     * back in by themselves after it, or two of them would keep taking the user from each other.
     */
    private void closeReplaced(ConnectionState signedIn) {
        String userName = signedIn.getUserName();
        for (WebSocket other : getConnections()) {
            ConnectionState state = other.getAttachment();
            if (state == null || state == signedIn || !userName.equalsIgnoreCase(state.getUserName())) {
                continue;
            }
            String sessionId = state.getSessionId();
            if (sessionId != null && !sessions.exists(sessionId) && sessions.isSignedInElsewhere(userName, sessionId)) {
                logger.info("Closing a WebSocket connection of " + userName + ": signed in somewhere else (" + describe(other) + ")");
                other.close(CLOSE_SIGNED_IN_ELSEWHERE, "Signed in somewhere else");
            }
        }
    }

    private static String safeMessage(Throwable e) {
        String message = e.getMessage();
        return message == null || message.isEmpty() ? e.getClass().getSimpleName() : message;
    }

    private void sendError(WebSocket conn, JsonElement id, RpcException error) {
        if (!conn.isOpen()) {
            return;
        }
        JsonObject errorJson = new JsonObject();
        errorJson.addProperty("code", error.getCode());
        errorJson.addProperty("message", error.getMessage());
        if (error.getReason() != null) {
            JsonObject data = new JsonObject();
            data.addProperty("reason", error.getReason());
            errorJson.add("data", data);
        }
        JsonObject response = new JsonObject();
        response.addProperty("jsonrpc", "2.0");
        response.add("id", id);
        response.add("error", errorJson);
        sendText(conn, JsonCodec.GSON.toJson(response));
    }

    /**
     * Every outgoing message goes through here. RPC responses and server callbacks are sent from different threads,
     * and the permessage-deflate encoder keeps per-connection state that is not thread safe ("Deflater has been closed"),
     * so sends on one connection are serialized.
     */
    static void sendText(WebSocket conn, String text) {
        synchronized (conn) {
            conn.send(text);
            BridgeMetrics.get().sent(text.length());
            ConnectionState state = conn.getAttachment();
            long limit = state == null ? WebSocketLimits.DEFAULT_MAX_OUTGOING_BYTES : state.getMaxOutgoingBytes();
            if (limit > 0 && conn instanceof WebSocketImpl && queuedBytesExceed(((WebSocketImpl) conn).outQueue, limit)) {
                // Java-WebSocket queues without limit: a client that stops reading would grow server memory forever
                logger.warn("Dropping WebSocket connection with more than " + limit + " bytes of unsent data: "
                        + (state == null ? conn.getRemoteSocketAddress() : state.getRemoteHost()));
                conn.closeConnection(CloseFrame.TRY_AGAIN_LATER, "Client is not reading its messages");
            }
        }
    }

    static boolean queuedBytesExceed(Iterable<ByteBuffer> queue, long limit) {
        long total = 0;
        for (ByteBuffer buffer : queue) {
            total += buffer.remaining();
            if (total > limit) {
                return true;
            }
        }
        return false;
    }

    @Override
    public void onError(WebSocket conn, Exception ex) {
        if (startupLatch.getCount() > 0) {
            startupException = ex;
            startupLatch.countDown();
        }
        logger.error("WebSocket error" + (conn == null ? "" : " on " + describe(conn)), ex);
    }

    /**
     * The client for logs: its real IP (forwarded by a trusted proxy) and the TCP peer it came through.
     */
    private static String describe(WebSocket conn) {
        ConnectionState state = conn.getAttachment();
        InetSocketAddress peer = conn.getRemoteSocketAddress();
        if (state == null || peer == null || state.getRemoteHost().equals(peer.getAddress().getHostAddress())) {
            return String.valueOf(peer);
        }
        return state.getRemoteHost() + " via " + peer;
    }

    @Override
    public void onStart() {
        logger.info("WebSocket server started on " + getAddress());
        startupLatch.countDown();
    }

    public void awaitStartup(long timeout, TimeUnit unit) throws Exception {
        if (!startupLatch.await(timeout, unit)) {
            throw new IllegalStateException("Timed out waiting for WebSocket server startup on " + getAddress());
        }
        if (startupException != null) {
            throw startupException;
        }
    }

    public void shutdown() {
        try {
            stop(1000);
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
        }
        workers.shutdown();
    }
}
