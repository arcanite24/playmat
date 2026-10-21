package mage.server;

import com.google.gson.Gson;
import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;
import mage.server.managers.ManagerFactory;
import org.apache.log4j.Logger;

import java.io.IOException;
import java.io.OutputStream;
import java.io.UnsupportedEncodingException;
import java.lang.management.ManagementFactory;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.net.URLDecoder;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;

/**
 * What build is running, and a /healthz answer for the container's health check. The endpoint listens on the
 * loopback address only (IPv4, which the container health check dials) (port {@value #DEFAULT_PORT}, or -Dxmage.healthPort; 0 turns it off).
 * <p>
 * The same listener lets the host's deploy script (ops/web/deploy/playmat-deploy, through {@code docker compose exec})
 * drain the server before a new version replaces it, without the admin password: {@code POST /maintenance?minutes=N&message=...}
 * announces maintenance (no restart when idle: the deploy restarts the container itself) and {@code POST /maintenance/cancel}
 * calls it off. Only processes inside the container can reach it.
 */
public final class Health {

    private static final Logger logger = Logger.getLogger(Health.class);

    public static final String PORT_PROP = "xmage.healthPort";
    public static final String COMMIT_PROP = "xmage.commit";
    static final int DEFAULT_PORT = 17173;
    /** the same bounds as the owner's adminMaintenance call */
    static final long MAX_MAINTENANCE_MINUTES = 24 * 60;
    static final int MAX_MAINTENANCE_MESSAGE = 300;
    /** who the announcement is from, in the log, the staff channel and the plan */
    static final String DEPLOY_ACTOR = "deploy";

    private static volatile String commit;

    private Health() {
    }

    /** What the endpoint reads and does: the running server, or a stand-in in tests. */
    interface Operations {
        Map<String, Object> snapshot();

        /** Announces maintenance in {@code minutes}; returns when it starts (epoch millis). */
        long startMaintenance(long minutes, String message);

        /** False when nothing was planned. */
        boolean cancelMaintenance();
    }

    public static String version() {
        return Main.getVersion().toString(false);
    }

    /** The git commit the server was built from: -Dxmage.commit, or the .commit file the Docker build writes; null when unknown. */
    public static String commit() {
        if (commit == null) {
            String fromProperty = System.getProperty(COMMIT_PROP);
            commit = fromProperty != null && !fromProperty.trim().isEmpty() ? fromProperty.trim() : readCommitFile(Paths.get(".commit"));
        }
        return commit.isEmpty() ? null : commit;
    }

    static String readCommitFile(Path file) {
        try {
            if (Files.isRegularFile(file)) {
                String text = new String(Files.readAllBytes(file), StandardCharsets.UTF_8).trim();
                return text.matches("[0-9a-f]{7,64}") ? text : "";
            }
        } catch (IOException e) {
            logger.warn("Can't read " + file + ": " + e.getMessage());
        }
        return "";
    }

    static Map<String, Object> snapshot(ManagerFactory managers) {
        Map<String, Object> health = new LinkedHashMap<>();
        health.put("status", "ok");
        health.put("version", version());
        health.put("commit", commit());
        health.put("uptimeSeconds", ManagementFactory.getRuntimeMXBean().getUptime() / 1000);
        health.put("usersOnline", managers.userManager().getUsers().size());
        int games = managers.gameManager().getNumberActiveGames();
        health.put("activeGames", games);
        // what maintenance waits for before a restart (games waiting for a slot included); the deploy drains on it
        health.put("gamesRunning", games);
        Maintenance.MaintenancePlan maintenance = Maintenance.get().plan();
        health.put("maintenanceAt", maintenance == null ? null : maintenance.startsAt);
        BackupStatus.BackupRun backup = BackupStatus.read();
        if (backup != null) {
            health.put("backupProblem", backup.problem);
        }
        return health;
    }

    /** Starts the endpoint unless it is turned off; a port in use is logged, not fatal. */
    static void start(ManagerFactory managers) {
        int port = Integer.getInteger(PORT_PROP, DEFAULT_PORT);
        if (port <= 0) {
            return;
        }
        Operations operations = new Operations() {
            @Override
            public Map<String, Object> snapshot() {
                return Health.snapshot(managers);
            }

            @Override
            public long startMaintenance(long minutes, String message) {
                return Maintenance.get().announce(managers, minutes, message, false, DEPLOY_ACTOR).startsAt;
            }

            @Override
            public boolean cancelMaintenance() {
                return Maintenance.get().cancel(managers);
            }
        };
        try {
            HttpServer server = listen(new InetSocketAddress(InetAddress.getByAddress(new byte[]{127, 0, 0, 1}), port), operations);
            Runtime.getRuntime().addShutdownHook(new Thread(() -> server.stop(0), "Health shutdown"));
            logger.info("Health check on http://127.0.0.1:" + port + "/healthz");
        } catch (IOException e) {
            logger.warn("Health check endpoint not started on port " + port + ": " + e.getMessage());
        }
    }

    /** The endpoint on {@code address} (port 0: any free one), started. */
    static HttpServer listen(InetSocketAddress address, Operations operations) throws IOException {
        HttpServer server = HttpServer.create(address, 0);
        Gson gson = new Gson();
        server.createContext("/healthz", exchange -> {
            try {
                Map<String, Object> health;
                try {
                    health = operations.snapshot();
                } catch (RuntimeException e) {
                    respond(exchange, 500, "{\"status\":\"error\"}");
                    return;
                }
                respond(exchange, 200, gson.toJson(health));
            } finally {
                exchange.close();
            }
        });
        // the deploy's drain: start maintenance, call it off. POST only, so nothing that merely looks (a browser, a probe) changes it.
        server.createContext("/maintenance", exchange -> {
            try {
                String path = exchange.getRequestURI().getPath();
                if (!path.equals("/maintenance") && !path.equals("/maintenance/cancel")) {
                    respond(exchange, 404, error("not found"));
                } else if (!"POST".equals(exchange.getRequestMethod())) {
                    exchange.getResponseHeaders().set("Allow", "POST");
                    respond(exchange, 405, error("use POST"));
                } else if (path.equals("/maintenance/cancel")) {
                    Map<String, Object> answer = new LinkedHashMap<>();
                    answer.put("cancelled", operations.cancelMaintenance());
                    respond(exchange, 200, gson.toJson(answer));
                } else {
                    Map<String, String> query = query(exchange.getRequestURI().getRawQuery());
                    long minutes;
                    try {
                        minutes = Long.parseLong(query.getOrDefault("minutes", ""));
                    } catch (NumberFormatException e) {
                        minutes = -1;
                    }
                    String message = query.getOrDefault("message", "");
                    if (minutes < 0 || minutes > MAX_MAINTENANCE_MINUTES) {
                        respond(exchange, 400, error("minutes is a whole number from 0 to " + MAX_MAINTENANCE_MINUTES));
                    } else if (message.length() > MAX_MAINTENANCE_MESSAGE) {
                        respond(exchange, 400, error("the message is " + MAX_MAINTENANCE_MESSAGE + " characters at most"));
                    } else {
                        Map<String, Object> answer = new LinkedHashMap<>();
                        answer.put("maintenanceAt", operations.startMaintenance(minutes, message));
                        respond(exchange, 200, gson.toJson(answer));
                    }
                }
            } catch (RuntimeException e) {
                logger.error("Health endpoint: " + exchange.getRequestURI().getPath() + " failed", e);
                respond(exchange, 500, error("failed: " + e.getMessage()));
            } finally {
                exchange.close();
            }
        });
        server.setExecutor(null);
        server.start();
        return server;
    }

    /** name=value pairs of a raw query string, decoded; the first of a repeated name wins. */
    static Map<String, String> query(String raw) {
        Map<String, String> values = new LinkedHashMap<>();
        if (raw == null || raw.isEmpty()) {
            return values;
        }
        for (String pair : raw.split("&")) {
            int eq = pair.indexOf('=');
            String name = decode(eq < 0 ? pair : pair.substring(0, eq));
            String value = eq < 0 ? "" : decode(pair.substring(eq + 1));
            values.putIfAbsent(name, value);
        }
        return values;
    }

    private static String decode(String text) {
        try {
            return URLDecoder.decode(text, "UTF-8");
        } catch (UnsupportedEncodingException | IllegalArgumentException e) {
            return text;
        }
    }

    private static String error(String message) {
        return new Gson().toJson(Collections.singletonMap("error", message));
    }

    private static void respond(HttpExchange exchange, int status, String json) throws IOException {
        byte[] body = json.getBytes(StandardCharsets.UTF_8);
        exchange.getResponseHeaders().set("Content-Type", "application/json");
        exchange.sendResponseHeaders(status, body.length);
        try (OutputStream out = exchange.getResponseBody()) {
            out.write(body);
        }
    }
}
