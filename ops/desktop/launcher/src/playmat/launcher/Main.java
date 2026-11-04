package playmat.launcher;

import java.awt.Desktop;
import java.awt.GraphicsEnvironment;
import java.io.File;
import java.io.IOException;
import java.io.RandomAccessFile;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.channels.FileLock;
import java.nio.file.Path;
import java.time.Duration;
import java.util.Arrays;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Playmat for one computer: starts the XMage server, serves the web client on http://localhost:17170 and opens the
 * browser when the game is ready. Closing the launcher stops both.
 *
 * <p>Arguments: --headless (no window, status on the console), --no-browser (don't open the browser).
 */
public final class Main {

    static final String RELEASES = "https://github.com/arcanite24/playmat/releases/latest";
    private static final String LATEST_API = "https://api.github.com/repos/arcanite24/playmat/releases/latest";

    private static FileLock instanceLock;

    private final Layout layout;
    private final Settings settings;
    private final GameServer gameServer;
    private final WebServer webServer;
    private final boolean openBrowser;
    private Ui ui;
    private volatile boolean stopping;
    private volatile boolean browserOpened;
    private Thread lifecycle;

    private Main(Layout layout, boolean openBrowser) {
        this.layout = layout;
        this.settings = new Settings(layout.data.resolve("settings.properties"));
        this.gameServer = new GameServer(layout, settings);
        this.webServer = new WebServer(layout.web(), gameServer::isReady);
        this.openBrowser = openBrowser;
    }

    public static void main(String[] args) throws Exception {
        List<String> options = Arrays.asList(args);
        boolean headless = options.contains("--headless") || GraphicsEnvironment.isHeadless();
        try {
            launch(options, headless);
        } catch (Throwable e) {
            // javaw has no console: without this a failed start would just do nothing
            Log.error("Playmat couldn't start", e);
            if (!headless) {
                try {
                    javax.swing.JOptionPane.showMessageDialog(null, "Playmat couldn't start:\n" + e,
                            "Playmat", javax.swing.JOptionPane.ERROR_MESSAGE);
                } catch (Throwable ignored) {
                    // no display after all
                }
            }
            System.exit(1);
        }
    }

    private static void launch(List<String> options, boolean headless) throws Exception {
        Layout layout = Layout.detect();

        // one launcher at a time: a second double-click just opens the game
        RandomAccessFile lockFile = new RandomAccessFile(layout.data.resolve("launcher.lock").toFile(), "rw");
        FileLock lock = lockFile.getChannel().tryLock();
        // held for the life of the process (a collected channel would release it)
        instanceLock = lock;
        if (lock == null) {
            System.out.println("Playmat is already running: http://localhost:" + WebServer.PORT);
            if (!headless) {
                browse("http://localhost:" + WebServer.PORT);
            }
            System.exit(0);
            return;
        }

        Log.open(layout.logs().resolve("launcher.log"));
        Log.info("Playmat " + layout.version() + " on " + System.getProperty("os.name") + " " + System.getProperty("os.arch")
                + ", Java " + System.getProperty("java.version"));
        Log.info("App: " + layout.app + "  Data: " + layout.data);

        Main main = new Main(layout, !options.contains("--no-browser") && !headless);
        Ui ui = null;
        if (!headless) {
            try {
                ui = Window.open(main, layout.version(), main.settings.accounts());
            } catch (Exception | Error e) {
                Log.warn("No window (" + e + "), status goes to the console");
            }
        }
        main.ui = ui != null ? ui : new ConsoleUi();
        headless = ui == null;
        Runtime.getRuntime().addShutdownHook(new Thread(main::shutdown, "playmat-shutdown"));
        main.startAll();
        main.checkForUpdate();
        if (headless) {
            Thread.currentThread().join();
        }
    }

    Path logFile() {
        return layout.logs().resolve("server.log");
    }

    Path dataFolder() {
        return layout.data;
    }

    Path webFolder() {
        return layout.web();
    }

    /** Starts (or restarts) the web client and the game server in the background. */
    synchronized void startAll() {
        if (lifecycle != null && lifecycle.isAlive()) {
            return;
        }
        stopping = false;
        lifecycle = new Thread(this::run, "playmat-lifecycle");
        lifecycle.setDaemon(true);
        lifecycle.start();
    }

    /** Stops the game server and starts it again with the current settings (the browser tab reconnects). */
    void restart() {
        new Thread(() -> {
            stopping = true;
            gameServer.stop();
            Thread previous = lifecycle;
            try {
                if (previous != null) {
                    previous.join(20000);
                }
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
            }
            startAll();
        }, "playmat-restart").start();
    }

    void setAccounts(boolean on) {
        settings.setAccounts(on);
    }

    void quit() {
        new Thread(() -> {
            shutdown();
            System.exit(0);
        }, "playmat-quit").start();
    }

    private synchronized void shutdown() {
        stopping = true;
        gameServer.stop();
        webServer.stop();
    }

    private boolean webStarted;

    private void run() {
        try {
            ui.starting("Starting Playmat…", "");
            gameServer.stopLeftover();

            if (!webStarted) {
                if (!Network.isFree(WebServer.PORT)) {
                    ui.failed("Port " + WebServer.PORT + " is in use",
                            "Another program is using it, maybe another copy of Playmat. Close it, then try again.", true);
                    return;
                }
                webServer.start();
                webStarted = true;
            }
            for (int port : new int[]{GameServer.BRIDGE_PORT}) {
                if (!Network.isFree(port)) {
                    ui.failed("Port " + port + " is in use",
                            "Another game server (Playmat or XMage) seems to be running on this computer. "
                                    + "Close it, then try again.", true);
                    return;
                }
            }

            gameServer.prepare(Network.origins(WebServer.PORT));
            // after prepare: a new version drops the card database there
            boolean first = gameServer.firstStart();
            gameServer.start();
            String waitNote = first
                    ? "First start: preparing the card database. This takes a few minutes, only this once."
                    : "Loading cards. This takes a moment.";
            ui.starting(first ? "Setting up Playmat…" : "Starting Playmat…", waitNote);

            long started = System.currentTimeMillis();
            Thread ticker = new Thread(() -> {
                while (!gameServer.isReady() && gameServer.isRunning()) {
                    long s = (System.currentTimeMillis() - started) / 1000;
                    String line = gameServer.lastLogLine();
                    ui.progress(String.format("%d:%02d", s / 60, s % 60) + (line == null ? "" : "  ·  " + shorten(line)));
                    try {
                        Thread.sleep(1000);
                    } catch (InterruptedException e) {
                        return;
                    }
                }
            }, "playmat-progress");
            ticker.setDaemon(true);
            ticker.start();

            if (!gameServer.awaitReady()) {
                if (!stopping) {
                    Log.warn("The game server stopped while starting; see " + logFile());
                    ui.failed("The game server stopped while starting",
                            "The log has the details. If it keeps happening, ask on the Playmat Discord.", true);
                }
                return;
            }
            String url = "http://localhost:" + WebServer.PORT;
            List<String> lan = Network.lanAddresses();
            String lanUrl = lan.isEmpty() ? null : "http://" + lan.get(0) + ":" + WebServer.PORT;
            Log.info("Ready after " + (System.currentTimeMillis() - started) / 1000 + "s");
            ui.ready(url, lanUrl);
            if (openBrowser && !browserOpened) {
                browserOpened = true;
                browse(url);
            }

            int code = gameServer.awaitExit();
            if (!stopping) {
                Log.warn("The game server exited with code " + code);
                ui.failed("The game server stopped", "The log has the details. Try again to start it.", true);
            }
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
        } catch (Exception e) {
            Log.error("Start failed", e);
            ui.failed("Playmat couldn't start", e.getMessage() == null ? e.toString() : e.getMessage(), true);
        }
    }

    private static String shorten(String line) {
        return line.length() > 90 ? line.substring(0, 89) + "…" : line;
    }

    /** Tells the player when a newer release is out; quietly does nothing offline. */
    private void checkForUpdate() {
        String current = layout.version();
        if (!current.matches("\\d+(\\.\\d+)*")) {
            return;
        }
        Thread check = new Thread(() -> {
            try {
                HttpClient client = HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(10))
                        .followRedirects(HttpClient.Redirect.NORMAL).build();
                HttpRequest request = HttpRequest.newBuilder(URI.create(LATEST_API)).timeout(Duration.ofSeconds(15))
                        .header("Accept", "application/vnd.github+json").header("User-Agent", "playmat-launcher").build();
                HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString());
                if (response.statusCode() != 200) {
                    return;
                }
                Matcher tag = Pattern.compile("\"tag_name\"\\s*:\\s*\"v?([0-9.]+)\"").matcher(response.body());
                if (tag.find() && newer(tag.group(1), current)) {
                    Log.info("Playmat " + tag.group(1) + " is available");
                    ui.updateAvailable(tag.group(1), RELEASES);
                }
            } catch (Exception e) {
                Log.info("Update check skipped: " + e);
            }
        }, "playmat-update-check");
        check.setDaemon(true);
        check.start();
    }

    static boolean newer(String candidate, String current) {
        String[] a = candidate.split("\\.");
        String[] b = current.split("\\.");
        for (int i = 0; i < Math.max(a.length, b.length); i++) {
            int x = i < a.length && !a[i].isEmpty() ? Integer.parseInt(a[i]) : 0;
            int y = i < b.length && !b[i].isEmpty() ? Integer.parseInt(b[i]) : 0;
            if (x != y) {
                return x > y;
            }
        }
        return false;
    }

    static void browse(String url) {
        try {
            if (Desktop.isDesktopSupported() && Desktop.getDesktop().isSupported(Desktop.Action.BROWSE)) {
                Desktop.getDesktop().browse(URI.create(url));
                return;
            }
        } catch (Exception e) {
            Log.warn("Desktop browse failed: " + e);
        }
        String[] command = Layout.isWindows() ? new String[]{"rundll32", "url.dll,FileProtocolHandler", url}
                : Layout.isMac() ? new String[]{"open", url} : new String[]{"xdg-open", url};
        try {
            new ProcessBuilder(command).start();
        } catch (IOException e) {
            Log.warn("Couldn't open the browser: " + e);
        }
    }

    static void openFolder(File folder) {
        try {
            if (Desktop.isDesktopSupported() && Desktop.getDesktop().isSupported(Desktop.Action.OPEN)) {
                Desktop.getDesktop().open(folder);
                return;
            }
        } catch (Exception e) {
            Log.warn("Desktop open failed: " + e);
        }
        String[] command = Layout.isWindows() ? new String[]{"explorer", folder.getAbsolutePath()}
                : Layout.isMac() ? new String[]{"open", folder.getAbsolutePath()} : new String[]{"xdg-open", folder.getAbsolutePath()};
        try {
            new ProcessBuilder(command).start();
        } catch (IOException e) {
            Log.warn("Couldn't open " + folder + ": " + e);
        }
    }
}
