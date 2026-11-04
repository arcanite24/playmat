package playmat.launcher;

import org.w3c.dom.Document;
import org.w3c.dom.Element;

import javax.xml.parsers.DocumentBuilderFactory;
import javax.xml.transform.OutputKeys;
import javax.xml.transform.Transformer;
import javax.xml.transform.TransformerFactory;
import javax.xml.transform.dom.DOMSource;
import javax.xml.transform.stream.StreamResult;
import java.io.File;
import java.io.IOException;
import java.io.RandomAccessFile;
import java.lang.management.ManagementFactory;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.nio.file.DirectoryStream;
import java.nio.file.FileVisitResult;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.SimpleFileVisitor;
import java.nio.file.StandardCopyOption;
import java.nio.file.attribute.BasicFileAttributes;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.concurrent.TimeUnit;

/**
 * The XMage server, run as a child process with the data folder's server/ as its working folder (XMage reads
 * config/, plugins/ and db/ relative to it) and the installed jars on its class path.
 */
final class GameServer {

    /** The WebSocket bridge; the web client connects to this port on the host it was loaded from. */
    static final int BRIDGE_PORT = 17172;
    static final int HEALTH_PORT = 17173;

    private final Layout layout;
    private final Settings settings;
    private volatile Process process;
    private volatile boolean ready;

    GameServer(Layout layout, Settings settings) {
        this.layout = layout;
        this.settings = settings;
    }

    boolean isReady() {
        return ready;
    }

    boolean isRunning() {
        Process p = process;
        return p != null && p.isAlive();
    }

    /** True when this start will build the card database, which takes minutes. */
    boolean firstStart() {
        Path cards = layout.serverHome().resolve("db").resolve("cards.h2.mv.db");
        try {
            return !Files.isRegularFile(cards) || Files.size(cards) < 1024 * 1024;
        } catch (IOException e) {
            return true;
        }
    }

    /** Lays out the working folder for this version and renders config.xml. */
    void prepare(List<String> allowedOrigins) throws Exception {
        Path home = layout.serverHome();
        Path release = layout.serverRelease();
        Files.createDirectories(home.resolve("db"));
        Files.createDirectories(home.resolve("saved"));

        // A new version brings a new card pool: drop the card database so it's rebuilt from this version's cards.
        // User data (accounts, decks, Career, stats) lives in other files in db/ and is kept.
        Path marker = home.resolve(".version");
        String version = layout.version() + " " + jarStamp(release.resolve("lib"));
        String previous = Files.isRegularFile(marker) ? new String(Files.readAllBytes(marker), StandardCharsets.UTF_8).trim() : "";
        if (!version.equals(previous)) {
            Log.info("New version (" + version + "), the card database will be rebuilt");
            try (DirectoryStream<Path> files = Files.newDirectoryStream(home.resolve("db"), "cards.h2.*")) {
                for (Path f : files) {
                    Files.deleteIfExists(f);
                }
            }
        }

        // plugins and config come from the release every start, so they always match the installed jars
        replaceTree(release.resolve("plugins"), home.resolve("plugins"));
        replaceTree(release.resolve("config"), home.resolve("config"));
        renderConfig(home.resolve("config").resolve("config.xml"), allowedOrigins);
        Files.write(home.resolve("config").resolve("log4j-playmat.properties"), (
                "log4j.rootLogger=info, console\n"
                        + "log4j.logger.com.j256.ormlite=warn\n"
                        + "log4j.appender.console=org.apache.log4j.ConsoleAppender\n"
                        + "log4j.appender.console.layout=org.apache.log4j.PatternLayout\n"
                        + "log4j.appender.console.layout.ConversionPattern=%-5p %d{yyyy-MM-dd HH:mm:ss} %m%n\n"
        ).getBytes(StandardCharsets.UTF_8));
        Files.write(marker, version.getBytes(StandardCharsets.UTF_8));
    }

    void start() throws IOException {
        Path home = layout.serverHome();
        Path log = layout.logs().resolve("server.log");
        if (Files.isRegularFile(log)) {
            Files.move(log, layout.logs().resolve("server.previous.log"), StandardCopyOption.REPLACE_EXISTING);
        }
        List<String> command = new ArrayList<>();
        command.add(Layout.javaExecutable().toString());
        command.add("-Xmx" + memory());
        command.add("-Dfile.encoding=UTF-8");
        // a server: no Dock icon or taskbar entry of its own
        command.add("-Djava.awt.headless=true");
        command.add("-Dxmage.testMode=false");
        command.add("-Dxmage.replays=true");
        command.add("-Dxmage.replays.dir=" + home.resolve("saved").resolve("replays"));
        command.add("-Dxmage.healthPort=" + HEALTH_PORT);
        command.add("-Dlog4j.configuration=" + home.resolve("config").resolve("log4j-playmat.properties").toUri());
        command.add("-cp");
        command.add(layout.serverRelease().resolve("lib") + File.separator + "*");
        command.add("mage.server.Main");
        Log.info("Starting the game server: " + String.join(" ", command));
        ProcessBuilder builder = new ProcessBuilder(command)
                .directory(home.toFile())
                .redirectErrorStream(true)
                .redirectOutput(log.toFile());
        process = builder.start();
        Files.write(layout.data.resolve("server.pid"), Long.toString(process.pid()).getBytes(StandardCharsets.UTF_8));
    }

    /**
     * Waits until the bridge port answers. The server opens it after the card database is ready. Returns false
     * when the process ended first.
     */
    boolean awaitReady() throws InterruptedException {
        while (isRunning()) {
            if (canConnect(BRIDGE_PORT)) {
                ready = true;
                return true;
            }
            Thread.sleep(1000);
        }
        return false;
    }

    /** Waits for the server to exit after it was ready (a crash, or it was stopped). */
    int awaitExit() throws InterruptedException {
        Process p = process;
        int code = p == null ? 0 : p.waitFor();
        ready = false;
        return code;
    }

    void stop() {
        Process p = process;
        ready = false;
        if (p == null || !p.isAlive()) {
            return;
        }
        Log.info("Stopping the game server");
        p.destroy();
        try {
            if (!p.waitFor(15, TimeUnit.SECONDS)) {
                p.destroyForcibly();
                p.waitFor(5, TimeUnit.SECONDS);
            }
        } catch (InterruptedException e) {
            p.destroyForcibly();
            Thread.currentThread().interrupt();
        }
        try {
            Files.deleteIfExists(layout.data.resolve("server.pid"));
        } catch (IOException ignored) {
            // harmless: the next start checks whether that process is still ours
        }
    }

    /**
     * A server left behind by a launcher that was killed (Task Manager, a crash) still holds the ports: stop it.
     * Only a process started from this installation's runtime is touched.
     */
    void stopLeftover() {
        Path pidFile = layout.data.resolve("server.pid");
        if (!Files.isRegularFile(pidFile)) {
            return;
        }
        try {
            long pid = Long.parseLong(new String(Files.readAllBytes(pidFile), StandardCharsets.UTF_8).trim());
            Optional<ProcessHandle> handle = ProcessHandle.of(pid);
            String runtime = Layout.javaExecutable().getParent().toString();
            if (handle.isPresent() && handle.get().isAlive()
                    && handle.get().info().command().map(c -> c.startsWith(runtime)).orElse(false)) {
                Log.info("Stopping a game server left running by an earlier session (pid " + pid + ")");
                handle.get().destroy();
                handle.get().onExit().get(15, TimeUnit.SECONDS);
            }
        } catch (Exception e) {
            Log.warn("Couldn't stop the earlier game server: " + e);
        }
        try {
            Files.deleteIfExists(pidFile);
        } catch (IOException ignored) {
            // retried next time
        }
    }

    /** The last line of the server's output, for the status window; null when there's nothing to show. */
    String lastLogLine() {
        Path log = layout.logs().resolve("server.log");
        try (RandomAccessFile file = new RandomAccessFile(log.toFile(), "r")) {
            long length = file.length();
            int size = (int) Math.min(length, 4096);
            byte[] tail = new byte[size];
            file.seek(length - size);
            file.readFully(tail);
            String[] lines = new String(tail, StandardCharsets.UTF_8).split("\\R");
            for (int i = lines.length - 1; i >= 0; i--) {
                String line = lines[i].trim();
                if (!line.isEmpty()) {
                    // drop the level and timestamp the layout puts first
                    return line.replaceFirst("^(INFO|WARN|ERROR|DEBUG|FATAL)\\s+\\d{4}-\\d\\d-\\d\\d \\d\\d:\\d\\d:\\d\\d\\s*", "");
                }
            }
        } catch (IOException e) {
            return null;
        }
        return null;
    }

    static boolean canConnect(int port) {
        try (Socket socket = new Socket()) {
            socket.connect(new InetSocketAddress("127.0.0.1", port), 500);
            return true;
        } catch (IOException e) {
            return false;
        }
    }

    private void renderConfig(Path config, List<String> allowedOrigins) throws Exception {
        DocumentBuilderFactory factory = DocumentBuilderFactory.newInstance();
        factory.setNamespaceAware(true);
        Document doc = factory.newDocumentBuilder().parse(config.toFile());
        Element server = (Element) doc.getElementsByTagName("server").item(0);
        server.setAttribute("serverName", "Playmat");
        server.setAttribute("authenticationActivated", Boolean.toString(settings.accounts()));
        server.setAttribute("saveGameActivated", "false");
        // only pages served by this launcher may open a game connection, not any site the player visits
        server.setAttribute("websocketAllowedOrigins", String.join(",", allowedOrigins));
        Transformer transformer = TransformerFactory.newInstance().newTransformer();
        transformer.setOutputProperty(OutputKeys.ENCODING, "UTF-8");
        transformer.transform(new DOMSource(doc), new StreamResult(config.toFile()));
    }

    private String memory() {
        String configured = settings.memory();
        if (!configured.isEmpty()) {
            return configured;
        }
        long total = 0;
        if (ManagementFactory.getOperatingSystemMXBean() instanceof com.sun.management.OperatingSystemMXBean) {
            total = ((com.sun.management.OperatingSystemMXBean) ManagementFactory.getOperatingSystemMXBean()).getTotalMemorySize();
        }
        long gb = total / (1024L * 1024 * 1024);
        if (gb >= 7) {
            return "2g";
        }
        if (gb >= 3) {
            return "1536m";
        }
        return "1g";
    }

    /** Changes when the installed server jars change, so a rebuilt dev bundle with the same VERSION counts too. */
    private static String jarStamp(Path lib) throws IOException {
        long newest = 0;
        long count = 0;
        try (DirectoryStream<Path> jars = Files.newDirectoryStream(lib, "*.jar")) {
            for (Path jar : jars) {
                newest = Math.max(newest, Files.getLastModifiedTime(jar).toMillis());
                count++;
            }
        }
        return count + "-" + newest;
    }

    private static void replaceTree(Path from, Path to) throws IOException {
        deleteTree(to);
        Files.walkFileTree(from, new SimpleFileVisitor<Path>() {
            @Override
            public FileVisitResult preVisitDirectory(Path dir, BasicFileAttributes attrs) throws IOException {
                Files.createDirectories(to.resolve(from.relativize(dir).toString()));
                return FileVisitResult.CONTINUE;
            }

            @Override
            public FileVisitResult visitFile(Path file, BasicFileAttributes attrs) throws IOException {
                Files.copy(file, to.resolve(from.relativize(file).toString()), StandardCopyOption.REPLACE_EXISTING);
                return FileVisitResult.CONTINUE;
            }
        });
    }

    private static void deleteTree(Path root) throws IOException {
        if (!Files.exists(root)) {
            return;
        }
        Files.walkFileTree(root, new SimpleFileVisitor<Path>() {
            @Override
            public FileVisitResult visitFile(Path file, BasicFileAttributes attrs) throws IOException {
                Files.delete(file);
                return FileVisitResult.CONTINUE;
            }

            @Override
            public FileVisitResult postVisitDirectory(Path dir, IOException e) throws IOException {
                Files.delete(dir);
                return FileVisitResult.CONTINUE;
            }
        });
    }
}
