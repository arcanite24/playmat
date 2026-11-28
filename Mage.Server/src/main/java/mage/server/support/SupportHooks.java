package mage.server.support;

import com.google.gson.Gson;
import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;
import mage.server.admin.GithubSignIn;
import mage.server.websocket.ClientAddressResolver;
import org.apache.log4j.Logger;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.sql.SQLException;
import java.util.ArrayDeque;
import java.util.Collections;
import java.util.Deque;
import java.util.HashMap;
import java.util.Iterator;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.ThreadPoolExecutor;
import java.util.concurrent.TimeUnit;
import java.util.function.LongSupplier;

/**
 * The payment providers' webhooks and the public goal, on a second embedded HTTP listener (the JDK's, like
 * {@code Health}) on 0.0.0.0:{@value #DEFAULT_PORT}. In the Docker deployment nothing publishes the port: Caddy
 * forwards {@code /hooks/*} (POST) and {@code /public/*} (GET) to it over the compose network.
 * <ul>
 * <li>{@code POST /hooks/kofi}: Ko-fi's form post; its verification token must match {@code XMAGE_KOFI_TOKEN}
 * (compared in constant time). 200 on success and on a repeat (Ko-fi retries), 401 on a wrong token, 400 when it
 * isn't a Ko-fi payment.</li>
 * <li>{@code POST /hooks/github-sponsors}: GitHub's sponsorship events, signed with {@code XMAGE_GITHUB_SPONSORS_SECRET}
 * (X-Hub-Signature-256); the ping event answers too.</li>
 * <li>{@code GET /public/support.json}: the month's goal, what it pays for and the public supporters wall, cached five
 * minutes and open to any origin (the project's site shows it).</li>
 * </ul>
 * Bodies are 64 KB at most and must arrive within 10 seconds; each client address may make
 * {@value #RATE_LIMIT} requests a minute. Port: {@code xmage.hooksPort} or {@code XMAGE_HOOKS_PORT} (0: off); by
 * default on only when a provider is configured.
 */
public final class SupportHooks {

    private static final Logger logger = Logger.getLogger(SupportHooks.class);

    public static final String PORT_PROP = "xmage.hooksPort";
    public static final String PORT_ENV = "XMAGE_HOOKS_PORT";
    public static final String KOFI_TOKEN_PROP = "xmage.kofi.token";
    public static final String KOFI_TOKEN_ENV = "XMAGE_KOFI_TOKEN";
    public static final String SPONSORS_SECRET_PROP = "xmage.githubSponsors.secret";
    public static final String SPONSORS_SECRET_ENV = "XMAGE_GITHUB_SPONSORS_SECRET";
    static final int DEFAULT_PORT = 17174;
    static final int MAX_BODY = 64 * 1024;
    static final int READ_TIMEOUT_SECONDS = 10;
    static final int RATE_LIMIT = 30;
    static final long RATE_WINDOW_MS = 60_000;
    static final int THREADS = 2;

    /** What the listener records and reads: the server's support, or a stand-in in tests. */
    interface Receiver {
        Support.Recorded kofi(KofiPayload payment, long now) throws SQLException;

        void sponsorship(GithubSponsorsEvent event, long now) throws SQLException;

        Support.SupportStatus status(long now) throws SQLException;
    }

    /** Where a request comes from: the client a trusted proxy names, or the peer. */
    interface Addresses {
        String of(HttpExchange exchange);
    }

    private static volatile boolean listening;

    private SupportHooks() {
    }

    /** whether the server takes payments (the listener runs) */
    public static boolean listening() {
        return listening;
    }

    static String setting(String prop, String env) {
        String value = System.getProperty(prop);
        if (value == null || value.trim().isEmpty()) {
            value = System.getenv(env);
        }
        return value == null ? "" : value.trim();
    }

    /** Starts the listener unless it is off; a port in use is logged, not fatal. Call before any other JDK HTTP server. */
    public static void start() {
        // the JDK server reads its limits once, when the first server starts: requests and answers get 10 seconds
        if (System.getProperty("sun.net.httpserver.maxReqTime") == null) {
            System.setProperty("sun.net.httpserver.maxReqTime", String.valueOf(READ_TIMEOUT_SECONDS));
        }
        if (System.getProperty("sun.net.httpserver.maxRspTime") == null) {
            System.setProperty("sun.net.httpserver.maxRspTime", String.valueOf(READ_TIMEOUT_SECONDS));
        }
        String kofiToken = setting(KOFI_TOKEN_PROP, KOFI_TOKEN_ENV);
        String sponsorsSecret = setting(SPONSORS_SECRET_PROP, SPONSORS_SECRET_ENV);
        String portText = setting(PORT_PROP, PORT_ENV);
        int port;
        try {
            port = portText.isEmpty() ? (kofiToken.isEmpty() && sponsorsSecret.isEmpty() ? 0 : DEFAULT_PORT) : Integer.parseInt(portText);
        } catch (NumberFormatException e) {
            logger.warn("Support webhooks off: " + PORT_ENV + " is not a port number");
            return;
        }
        if (!sponsorsSecret.isEmpty()) {
            // a sponsor who signs in with GitHub later gets their sponsorship linked then
            GithubSignIn.onLinked((githubId, account, now) -> {
                try {
                    Support.get().githubLinked(githubId, account, now);
                } catch (SQLException e) {
                    logger.error("Support: sponsorship not linked at sign-in", e);
                }
            });
        }
        if (port <= 0) {
            return;
        }
        Receiver receiver = new Receiver() {
            @Override
            public Support.Recorded kofi(KofiPayload payment, long now) throws SQLException {
                return Support.get().recordKofi(payment, now);
            }

            @Override
            public void sponsorship(GithubSponsorsEvent event, long now) throws SQLException {
                Support.get().recordSponsorship(event, now);
            }

            @Override
            public Support.SupportStatus status(long now) throws SQLException {
                return Support.get().status(null, now);
            }
        };
        ClientAddressResolver resolver = ClientAddressResolver.fromSystemProperties();
        Addresses addresses = exchange -> resolver.resolve(exchange.getRemoteAddress().getAddress().getHostAddress(),
                exchange.getRequestHeaders().getFirst("X-Forwarded-For"), exchange.getRequestHeaders().getFirst("X-Real-IP"));
        try {
            HttpServer server = listen(new InetSocketAddress(port), kofiToken, sponsorsSecret, receiver, addresses, System::currentTimeMillis);
            Runtime.getRuntime().addShutdownHook(new Thread(() -> server.stop(0), "Support webhooks shutdown"));
            listening = true;
            Support.startDaily();
            logger.info("Support webhooks on port " + port + ": Ko-fi " + (kofiToken.isEmpty() ? "off" : "on")
                    + ", GitHub Sponsors " + (sponsorsSecret.isEmpty() ? "off" : "on"));
        } catch (IOException e) {
            logger.warn("Support webhooks not started on port " + port + ": " + e.getMessage());
        }
    }

    /** The listener on {@code address} (port 0: any free one), started. */
    static HttpServer listen(InetSocketAddress address, String kofiToken, String sponsorsSecret, Receiver receiver,
                             Addresses addresses, LongSupplier clock) throws IOException {
        HttpServer server = HttpServer.create(address, 0);
        Gson gson = new Gson();
        RateLimit limit = new RateLimit(clock);
        server.createContext("/hooks/", exchange -> {
            try {
                if (!limit.take(addresses.of(exchange))) {
                    exchange.getResponseHeaders().set("Retry-After", "60");
                    respond(exchange, 429, error("too many requests"));
                    return;
                }
                String path = exchange.getRequestURI().getPath();
                boolean kofi = path.equals("/hooks/kofi");
                boolean github = path.equals("/hooks/github-sponsors");
                if (!kofi && !github) {
                    respond(exchange, 404, error("not found"));
                } else if (!"POST".equals(exchange.getRequestMethod())) {
                    exchange.getResponseHeaders().set("Allow", "POST");
                    respond(exchange, 405, error("use POST"));
                } else if (kofi && kofiToken.isEmpty() || github && sponsorsSecret.isEmpty()) {
                    respond(exchange, 404, error("this provider is not set up here"));
                } else {
                    byte[] body = body(exchange);
                    if (body == null) {
                        respond(exchange, 413, error("the body is " + MAX_BODY / 1024 + " KB at most"));
                    } else if (kofi) {
                        kofi(exchange, body, kofiToken, receiver, clock, gson);
                    } else {
                        github(exchange, body, sponsorsSecret, receiver, clock, gson);
                    }
                }
            } catch (SQLException | RuntimeException e) {
                logger.error("Support webhook " + exchange.getRequestURI().getPath() + " failed", e);
                // the provider retries a failed delivery
                respond(exchange, 500, error("failed"));
            } finally {
                exchange.close();
            }
        });
        server.createContext("/public/", exchange -> {
            try {
                cors(exchange);
                if (!limit.take(addresses.of(exchange))) {
                    exchange.getResponseHeaders().set("Retry-After", "60");
                    respond(exchange, 429, error("too many requests"));
                } else if (!exchange.getRequestURI().getPath().equals("/public/support.json")) {
                    respond(exchange, 404, error("not found"));
                } else if ("OPTIONS".equals(exchange.getRequestMethod())) {
                    exchange.sendResponseHeaders(204, -1);
                } else if (!"GET".equals(exchange.getRequestMethod()) && !"HEAD".equals(exchange.getRequestMethod())) {
                    exchange.getResponseHeaders().set("Allow", "GET, HEAD, OPTIONS");
                    respond(exchange, 405, error("use GET"));
                } else {
                    Support.SupportStatus status = receiver.status(clock.getAsLong());
                    exchange.getResponseHeaders().set("Cache-Control", "public, max-age=300");
                    respond(exchange, 200, gson.toJson(status));
                }
            } catch (SQLException | RuntimeException e) {
                logger.error("Support status failed", e);
                respond(exchange, 500, error("failed"));
            } finally {
                exchange.close();
            }
        });
        ThreadPoolExecutor executor = new ThreadPoolExecutor(THREADS, THREADS, 0, TimeUnit.MILLISECONDS, new LinkedBlockingQueue<>(), runnable -> {
            Thread thread = new Thread(runnable, "XMAGE support hooks");
            thread.setDaemon(true);
            return thread;
        });
        server.setExecutor(executor);
        server.start();
        return server;
    }

    private static void kofi(HttpExchange exchange, byte[] body, String token, Receiver receiver, LongSupplier clock, Gson gson)
            throws IOException, SQLException {
        KofiPayload payment;
        try {
            payment = KofiPayload.parse(new String(body, StandardCharsets.UTF_8));
        } catch (IllegalArgumentException e) {
            respond(exchange, 400, error("not a Ko-fi payment: " + e.getMessage()));
            return;
        }
        if (!sameSecret(token, payment.verificationToken)) {
            logger.warn("Support: Ko-fi delivery with a wrong verification token");
            respond(exchange, 401, error("wrong verification token"));
            return;
        }
        Support.Recorded recorded = receiver.kofi(payment, clock.getAsLong());
        Map<String, Object> answer = new LinkedHashMap<>();
        answer.put("ok", true);
        answer.put("duplicate", recorded.duplicate);
        respond(exchange, 200, gson.toJson(answer));
    }

    private static void github(HttpExchange exchange, byte[] body, String secret, Receiver receiver, LongSupplier clock, Gson gson)
            throws IOException, SQLException {
        if (!GithubSponsorsEvent.verify(secret, body, exchange.getRequestHeaders().getFirst("X-Hub-Signature-256"))) {
            logger.warn("Support: GitHub Sponsors delivery with a wrong signature");
            respond(exchange, 401, error("wrong signature"));
            return;
        }
        String event = exchange.getRequestHeaders().getFirst("X-GitHub-Event");
        Map<String, Object> answer = new LinkedHashMap<>();
        answer.put("ok", true);
        if ("ping".equals(event)) {
            answer.put("pong", true);
            respond(exchange, 200, gson.toJson(answer));
            return;
        }
        if (!"sponsorship".equals(event)) {
            answer.put("ignored", event);
            respond(exchange, 202, gson.toJson(answer));
            return;
        }
        GithubSponsorsEvent sponsorship;
        try {
            sponsorship = GithubSponsorsEvent.parse(new String(body, StandardCharsets.UTF_8));
        } catch (IllegalArgumentException e) {
            respond(exchange, 400, error("not a sponsorship event: " + e.getMessage()));
            return;
        }
        receiver.sponsorship(sponsorship, clock.getAsLong());
        respond(exchange, 200, gson.toJson(answer));
    }

    /** Constant time, whatever the guess. */
    static boolean sameSecret(String expected, String given) {
        if (expected == null || expected.isEmpty() || given == null) {
            return false;
        }
        try {
            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            byte[] a = digest.digest(expected.getBytes(StandardCharsets.UTF_8));
            byte[] b = MessageDigest.getInstance("SHA-256").digest(given.getBytes(StandardCharsets.UTF_8));
            return MessageDigest.isEqual(a, b);
        } catch (java.security.NoSuchAlgorithmException e) {
            throw new IllegalStateException(e);
        }
    }

    /** The request body, or null when it is over the limit. */
    private static byte[] body(HttpExchange exchange) throws IOException {
        String length = exchange.getRequestHeaders().getFirst("Content-Length");
        if (length != null) {
            try {
                if (Long.parseLong(length.trim()) > MAX_BODY) {
                    drain(exchange);
                    return null;
                }
            } catch (NumberFormatException ignored) {
                // read it and see
            }
        }
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        byte[] chunk = new byte[8192];
        try (InputStream in = exchange.getRequestBody()) {
            int read;
            while ((read = in.read(chunk)) > 0) {
                out.write(chunk, 0, read);
                if (out.size() > MAX_BODY) {
                    drain(exchange);
                    return null;
                }
            }
        }
        return out.toByteArray();
    }

    /** Reads off (and drops) some of what is left, so closing on an unread body does not reset the client before it sees the 413. */
    private static void drain(HttpExchange exchange) {
        byte[] chunk = new byte[8192];
        long left = 1024 * 1024;
        try {
            InputStream in = exchange.getRequestBody();
            int read;
            while (left > 0 && (read = in.read(chunk)) > 0) {
                left -= read;
            }
        } catch (IOException ignored) {
            // the client gave up; nothing to answer
        }
    }

    private static void cors(HttpExchange exchange) {
        exchange.getResponseHeaders().set("Access-Control-Allow-Origin", "*");
        exchange.getResponseHeaders().set("Access-Control-Allow-Methods", "GET, HEAD, OPTIONS");
        exchange.getResponseHeaders().set("Access-Control-Max-Age", "86400");
    }

    private static String error(String message) {
        return new Gson().toJson(Collections.singletonMap("error", message));
    }

    private static void respond(HttpExchange exchange, int status, String json) throws IOException {
        byte[] body = json.getBytes(StandardCharsets.UTF_8);
        exchange.getResponseHeaders().set("Content-Type", "application/json");
        boolean head = "HEAD".equals(exchange.getRequestMethod());
        exchange.sendResponseHeaders(status, head ? -1 : body.length);
        if (!head) {
            try (OutputStream out = exchange.getResponseBody()) {
                out.write(body);
            }
        }
    }

    /** {@value #RATE_LIMIT} requests a minute per client address. */
    static final class RateLimit {
        private final LongSupplier clock;
        private final Map<String, Deque<Long>> recent = new HashMap<>();

        RateLimit(LongSupplier clock) {
            this.clock = clock;
        }

        synchronized boolean take(String address) {
            long now = clock.getAsLong();
            if (recent.size() > 10_000) {
                for (Iterator<Deque<Long>> it = recent.values().iterator(); it.hasNext(); ) {
                    Deque<Long> times = it.next();
                    if (times.isEmpty() || now - times.peekLast() >= RATE_WINDOW_MS) {
                        it.remove();
                    }
                }
            }
            Deque<Long> times = recent.computeIfAbsent(address == null ? "" : address, a -> new ArrayDeque<>());
            while (!times.isEmpty() && now - times.peekFirst() >= RATE_WINDOW_MS) {
                times.pollFirst();
            }
            if (times.size() >= RATE_LIMIT) {
                return false;
            }
            times.addLast(now);
            return true;
        }
    }
}
