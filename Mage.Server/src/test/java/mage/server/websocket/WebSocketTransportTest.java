package mage.server.websocket;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import mage.interfaces.MageServer;
import mage.interfaces.ServerState;
import mage.players.PlayerType;
import mage.server.DisconnectReason;
import mage.server.websocket.rpc.RpcSessions;
import org.java_websocket.WebSocket;
import org.java_websocket.client.WebSocketClient;
import org.java_websocket.drafts.Draft;
import org.java_websocket.drafts.Draft_6455;
import org.java_websocket.extensions.permessage_deflate.PerMessageDeflateExtension;
import org.java_websocket.handshake.ServerHandshake;
import org.jboss.remoting.callback.InvokerCallbackHandler;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.lang.reflect.Proxy;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.net.ServerSocket;
import java.net.URI;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.TimeUnit;

import static org.assertj.core.api.Assertions.assertThat;

/**
 * Web client bridge over a real socket: handshake, framing, error objects and origin checks.
 */
public class WebSocketTransportTest {

    private WebSocketServerImpl server;
    private int port;

    @BeforeEach
    void startServer() throws Exception {
        start("https://play.example.com");
    }

    @AfterEach
    void stopServer() {
        if (server != null) {
            server.shutdown();
        }
    }

    private void start(String allowedOrigins) throws Exception {
        start(allowedOrigins, WebSocketLimits.fromSystemProperties());
    }

    private void restart(WebSocketLimits limits) throws Exception {
        server.shutdown();
        start("https://play.example.com", limits);
    }

    private void start(String allowedOrigins, WebSocketLimits limits) throws Exception {
        try (ServerSocket socket = new ServerSocket(0)) {
            port = socket.getLocalPort();
        }
        MageServer mageServer = (MageServer) Proxy.newProxyInstance(MageServer.class.getClassLoader(),
                new Class<?>[]{MageServer.class}, (proxy, method, args) -> {
                    if (method.getReturnType() == ServerState.class) {
                        return new ServerState(Collections.emptyList(), Collections.emptyList(),
                                new PlayerType[0], new String[]{"Constructed - Standard"}, new String[0], false, null, 0, 0);
                    }
                    return null;
                });
        server = new WebSocketServerImpl(new InetSocketAddress("127.0.0.1", port), mageServer, null, allowedOrigins, "", limits);
        server.start();
        server.awaitStartup(10, TimeUnit.SECONDS);
    }

    /** A server whose logins work like the real one's with accounts: logging in as a connected user takes it over. */
    private OneSessionPerUser startWithLogins() throws Exception {
        server.shutdown();
        try (ServerSocket socket = new ServerSocket(0)) {
            port = socket.getLocalPort();
        }
        OneSessionPerUser sessions = new OneSessionPerUser();
        MageServer mageServer = (MageServer) Proxy.newProxyInstance(MageServer.class.getClassLoader(),
                new Class<?>[]{MageServer.class}, (proxy, method, args) -> {
                    if (method.getName().equals("connectUser")) {
                        sessions.login((String) args[0], (String) args[2]);
                        return true;
                    }
                    return method.getReturnType() == boolean.class ? false : null;
                });
        server = new WebSocketServerImpl(new InetSocketAddress("127.0.0.1", port), mageServer, null, sessions, "", "",
                WebSocketLimits.fromSystemProperties());
        server.start();
        server.awaitStartup(10, TimeUnit.SECONDS);
        return sessions;
    }

    @Test
    void aLoginAsTheSameUserClosesTheConnectionItTookTheUserFrom() throws Exception {
        OneSessionPerUser sessions = startWithLogins();
        TestClient first = TestClient.connect(port, null);
        TestClient bob = TestClient.connect(port, null);
        TestClient second = TestClient.connect(port, null);
        try {
            assertThat(login(first, "alice").get("result").getAsBoolean()).isTrue();
            assertThat(login(bob, "bob").get("result").getAsBoolean()).isTrue();
            assertThat(login(second, "alice").get("result").getAsBoolean()).isTrue();

            assertThat(first.awaitClose(5000)).as("the connection alice was taken from is closed").isTrue();
            assertThat(first.closeCode).isEqualTo(WebSocketServerImpl.CLOSE_SIGNED_IN_ELSEWHERE);
            assertThat(bob.awaitClose(300)).as("other users stay").isFalse();
            assertThat(second.awaitClose(300)).as("the new login stays").isFalse();
            assertThat(sessions.live).hasSize(2);
        } finally {
            first.closeBlocking();
            bob.closeBlocking();
            second.closeBlocking();
        }
    }

    @Test
    void aConnectionWhoseSessionEndedOtherwiseIsNotClosed() throws Exception {
        OneSessionPerUser sessions = startWithLogins();
        TestClient first = TestClient.connect(port, null);
        TestClient second = TestClient.connect(port, null);
        try {
            login(first, "alice");
            // the session expired or the server dropped it: alice is nowhere else, and a login as bob is someone else
            sessions.live.clear();
            sessions.users.clear();
            login(second, "bob");
            assertThat(first.awaitClose(300)).isFalse();

            JsonObject call = first.request("{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"roomGetAllTables\",\"params\":[\""
                    + java.util.UUID.randomUUID() + "\"]}");
            assertThat(errorCode(call)).isEqualTo(-32001);
            assertThat(call.get("error").getAsJsonObject().has("data")).as("no reason: it just has no session").isFalse();
        } finally {
            first.closeBlocking();
            second.closeBlocking();
        }
    }

    private static JsonObject login(TestClient client, String userName) throws InterruptedException {
        return client.request("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"connectUser\",\"params\":[\"" + userName
                + "\",\"\",\"x\"]}");
    }

    /** The server's sessions as the bridge sees them, with one session per user like a server with accounts. */
    private static final class OneSessionPerUser implements RpcSessions {
        final Set<String> live = ConcurrentHashMap.newKeySet();
        final Map<String, String> users = new ConcurrentHashMap<>(); // user name -> its session
        final Map<String, String> names = new ConcurrentHashMap<>(); // session -> user name

        void login(String userName, String sessionId) {
            String previous = users.put(userName, sessionId);
            if (previous != null && !previous.equals(sessionId)) {
                live.remove(previous); // the server ends the other instance's session
            }
            names.put(sessionId, userName);
        }

        @Override
        public void create(String sessionId, InvokerCallbackHandler callbackHandler, String host) {
            live.add(sessionId);
        }

        @Override
        public boolean exists(String sessionId) {
            return live.contains(sessionId);
        }

        @Override
        public void disconnect(String sessionId, DisconnectReason reason) {
            live.remove(sessionId);
        }

        @Override
        public boolean isSignedInElsewhere(String userName, String sessionId) {
            String current = userName == null ? null : users.get(userName);
            return current != null && !current.equals(sessionId) && live.contains(current);
        }

        @Override
        public Optional<String> userName(String sessionId) {
            return Optional.ofNullable(names.get(sessionId));
        }

        @Override
        public Optional<String> restoreToken(String sessionId) {
            return Optional.empty();
        }

        @Override
        public Optional<String> rememberLogin(String sessionId) {
            return Optional.empty();
        }

        @Override
        public void trustRememberedLogin(String sessionId, String userName, String token) {
        }

        @Override
        public void forgetLogin(String userName, String token) {
        }

        @Override
        public boolean isChatMember(String sessionId, UUID chatId) {
            return false;
        }
    }

    @Test
    void publicRequestWorksOverABrowserLikeHandshake() throws Exception {
        TestClient client = TestClient.connect(port, null);
        try {
            JsonObject response = client.request("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getDeckTypes\",\"params\":[]}");
            assertThat(response.get("id").getAsInt()).isEqualTo(1);
            assertThat(response.get("result").getAsJsonArray().get(0).getAsString()).isEqualTo("Constructed - Standard");
        } finally {
            client.closeBlocking();
        }
    }

    @Test
    void errorsAreJsonRpcErrorObjects() throws Exception {
        TestClient client = TestClient.connect(port, null);
        try {
            assertThat(errorCode(client.request("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"nope\"}"))).isEqualTo(-32601);
            assertThat(errorCode(client.request("{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"roomGetAllTables\",\"params\":[\"" + java.util.UUID.randomUUID() + "\"]}")))
                    .isEqualTo(-32001);
            assertThat(errorCode(client.request("not json"))).isEqualTo(-32700);
            assertThat(errorCode(client.request("{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"getDeckTypes\",\"params\":{}}"))).isEqualTo(-32600);
        } finally {
            client.closeBlocking();
        }
    }

    @Test
    void responsesKeepRequestOrderPerConnection() throws Exception {
        TestClient client = TestClient.connect(port, null);
        try {
            for (int i = 1; i <= 20; i++) {
                client.send("{\"jsonrpc\":\"2.0\",\"id\":" + i + ",\"method\":\"getDeckTypes\"}");
            }
            for (int i = 1; i <= 20; i++) {
                assertThat(client.next().get("id").getAsInt()).isEqualTo(i);
            }
        } finally {
            client.closeBlocking();
        }
    }

    @Test
    void allowedOriginsAreEnforcedForBrowsers() throws Exception {
        TestClient allowed = TestClient.connect(port, "https://play.example.com");
        try {
            assertThat(allowed.isOpen()).isTrue();
        } finally {
            allowed.closeBlocking();
        }
        TestClient blocked = TestClient.connect(port, "https://evil.example.com");
        try {
            assertThat(blocked.isOpen()).isFalse();
        } finally {
            blocked.closeBlocking();
        }
    }

    @Test
    void concurrentSendsOnACompressedConnectionAllArrive() throws Exception {
        TestClient client = TestClient.connect(port, null, true);
        try {
            WebSocket conn = server.getConnections().iterator().next();
            String payload = String.join("", Collections.nCopies(200, "compressible text "));
            int threads = 8;
            int perThread = 100;
            ExecutorService pool = Executors.newFixedThreadPool(threads);
            List<Future<?>> sends = new ArrayList<>();
            for (int t = 0; t < threads; t++) {
                // RPC responses and server callbacks share a connection but come from different threads
                sends.add(pool.submit(() -> {
                    for (int i = 0; i < perThread; i++) {
                        WebSocketServerImpl.sendText(conn, "{\"n\":" + i + ",\"text\":\"" + payload + "\"}");
                    }
                }));
            }
            for (Future<?> send : sends) {
                send.get(20, TimeUnit.SECONDS);
            }
            pool.shutdown();
            for (int i = 0; i < threads * perThread; i++) {
                assertThat(client.next().get("text").getAsString()).isEqualTo(payload);
            }
        } finally {
            client.closeBlocking();
        }
    }

    @Test
    void connectionsPerClientAddressAreLimited() throws Exception {
        restart(new WebSocketLimits(new ClientAddressResolver(""), 2, true, WebSocketLimits.DEFAULT_MAX_OUTGOING_BYTES));
        TestClient first = TestClient.connect(port, null);
        TestClient second = TestClient.connect(port, null);
        TestClient third = TestClient.connect(port, null);
        try {
            assertThat(first.awaitClose(300)).isFalse();
            assertThat(second.awaitClose(300)).isFalse();
            assertThat(third.awaitClose(5000)).as("third connection from the same address is closed").isTrue();
            assertThat(third.closeCode).isEqualTo(1008);
        } finally {
            first.closeBlocking();
            second.closeBlocking();
            third.closeBlocking();
        }
        // closed connections free their slot
        waitUntil(() -> server.openConnectionsFrom("127.0.0.1") == 0);
        TestClient again = TestClient.connect(port, null);
        try {
            assertThat(again.awaitClose(300)).isFalse();
        } finally {
            again.closeBlocking();
        }
    }

    @Test
    void forwardedAddressIsUsedOnlyFromATrustedProxy() throws Exception {
        restart(new WebSocketLimits(new ClientAddressResolver("127.0.0.1"), 16, true, WebSocketLimits.DEFAULT_MAX_OUTGOING_BYTES));
        Map<String, String> headers = new HashMap<>();
        headers.put("X-Forwarded-For", "6.6.6.6, 203.0.113.5");
        TestClient client = TestClient.connectWithHeaders(port, headers);
        try {
            waitUntil(() -> server.openConnectionsFrom("203.0.113.5") == 1);
            assertThat(server.openConnectionsFrom("127.0.0.1")).isZero();
            assertThat(server.openConnectionsFrom("6.6.6.6")).isZero();
        } finally {
            client.closeBlocking();
        }
    }

    @Test
    void clientThatStopsReadingIsDropped() throws Exception {
        long limit = 256 * 1024;
        restart(new WebSocketLimits(new ClientAddressResolver(""), 16, false, limit));
        try (Socket socket = new Socket("127.0.0.1", port)) {
            socket.setReceiveBufferSize(16 * 1024);
            OutputStream out = socket.getOutputStream();
            out.write(("GET / HTTP/1.1\r\nHost: 127.0.0.1:" + port + "\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n"
                    + "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n")
                    .getBytes(StandardCharsets.US_ASCII));
            out.flush();
            InputStream in = socket.getInputStream();
            StringBuilder response = new StringBuilder();
            while (!response.toString().endsWith("\r\n\r\n")) {
                int b = in.read();
                assertThat(b).as("handshake response").isNotEqualTo(-1);
                response.append((char) b);
            }
            assertThat(response.toString()).startsWith("HTTP/1.1 101");

            waitUntil(() -> !server.getConnections().isEmpty());
            WebSocket conn = server.getConnections().iterator().next();
            String chunk = String.join("", Collections.nCopies(64 * 1024, "x"));
            // the socket never reads: kernel buffers fill up, then the server's queue
            for (int i = 0; i < 2000 && conn.isOpen(); i++) {
                WebSocketServerImpl.sendText(conn, chunk);
            }
            assertThat(conn.isOpen()).as("connection dropped once more than " + limit + " bytes were queued").isFalse();
        }
    }

    private static void waitUntil(java.util.function.BooleanSupplier condition) throws InterruptedException {
        long deadline = System.currentTimeMillis() + 5000;
        while (!condition.getAsBoolean()) {
            assertThat(System.currentTimeMillis()).as("condition within 5 s").isLessThan(deadline);
            Thread.sleep(20);
        }
    }

    private static int errorCode(JsonObject response) {
        return response.get("error").getAsJsonObject().get("code").getAsInt();
    }

    private static final class TestClient extends WebSocketClient {
        private final BlockingQueue<String> messages = new LinkedBlockingQueue<>();
        private final java.util.concurrent.CountDownLatch closed = new java.util.concurrent.CountDownLatch(1);
        volatile int closeCode;

        private TestClient(URI uri, Draft draft, Map<String, String> headers) {
            super(uri, draft, headers);
        }

        static TestClient connect(int port, String origin) throws InterruptedException {
            return connect(port, origin, false);
        }

        static TestClient connectWithHeaders(int port, Map<String, String> headers) throws InterruptedException {
            TestClient client = new TestClient(URI.create("ws://127.0.0.1:" + port), new Draft_6455(), headers);
            client.connectBlocking(5, TimeUnit.SECONDS);
            return client;
        }

        boolean awaitClose(long millis) throws InterruptedException {
            return closed.await(millis, TimeUnit.MILLISECONDS);
        }

        static TestClient connect(int port, String origin, boolean compressed) throws InterruptedException {
            Map<String, String> headers = new HashMap<>();
            if (origin != null) {
                headers.put("Origin", origin);
            }
            Draft draft = compressed
                    ? new Draft_6455(Collections.singletonList(new PerMessageDeflateExtension()))
                    : new Draft_6455();
            TestClient client = new TestClient(URI.create("ws://127.0.0.1:" + port), draft, headers);
            client.connectBlocking(5, TimeUnit.SECONDS);
            return client;
        }

        JsonObject request(String json) throws InterruptedException {
            send(json);
            return next();
        }

        JsonObject next() throws InterruptedException {
            String message = messages.poll(5, TimeUnit.SECONDS);
            assertThat(message).as("response from server").isNotNull();
            return JsonParser.parseString(message).getAsJsonObject();
        }

        @Override
        public void onOpen(ServerHandshake handshake) {
        }

        @Override
        public void onMessage(String message) {
            messages.add(message);
        }

        @Override
        public void onClose(int code, String reason, boolean remote) {
            closeCode = code;
            closed.countDown();
        }

        @Override
        public void onError(Exception ex) {
        }
    }
}
