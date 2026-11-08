package mage.server.websocket.api;

import mage.server.BackupStatus;
import mage.server.Health;
import mage.server.Maintenance;
import mage.server.managers.ManagerFactory;
import mage.server.websocket.BridgeMetrics;

import java.lang.management.ManagementFactory;
import java.lang.management.MemoryUsage;
import java.lang.management.OperatingSystemMXBean;

/**
 * Admin console: the server's load at a glance.
 */
public class ServerStats {

    public long now;
    public String version;
    /** the git commit the server was built from, or null when unknown */
    public String commit;
    public long uptimeMillis;
    public int usersOnline;
    public int tables;
    public int activeGames;
    public long heapUsed;
    public long heapMax;
    /** this process's CPU use, 0..1, or -1 when the JVM can't tell */
    public double processCpu;
    /** the system load average over the last minute, or -1 */
    public double systemLoad;
    public int processors;
    public int threads;
    public BridgeMetrics.BridgeSnapshot bridge;
    /** the nightly backup's last run; null when this server has no backup service */
    public BackupStatus.BackupRun backup;
    /** planned maintenance, or null */
    public Maintenance.MaintenancePlan maintenance;
    /** the Rust AI's games since the server started (docs/rust/Ai.md, docs/rust/Views.md) */
    public RustAiStats rustAi;

    /** Rust AI games: started, answers given, stopped partway (back to the Java AI), finished; the views compared */
    public static class RustAiStats {
        public long started;
        public long answered;
        public long stopped;
        public long finished;
        /** the human's views the Rust copies rendered and compared with Java's, and those with a field that differs */
        public long viewsCompared;
        public long viewsDiffering;
        /** the human's prompts the copies compared, and those with a field that differs */
        public long promptsCompared;
        public long promptsDiffering;
        /** the game log's lines the copies compared, and those they didn't write the same */
        public long logLinesCompared;
        public long logLinesDiffering;
    }

    static ServerStats collect(ManagerFactory managers) {
        ServerStats stats = new ServerStats();
        stats.now = System.currentTimeMillis();
        stats.version = Health.version();
        stats.commit = Health.commit();
        stats.uptimeMillis = ManagementFactory.getRuntimeMXBean().getUptime();
        stats.usersOnline = managers.userManager().getUsers().size();
        stats.tables = managers.tableManager().getTables().size();
        stats.activeGames = managers.gameManager().getNumberActiveGames();
        MemoryUsage heap = ManagementFactory.getMemoryMXBean().getHeapMemoryUsage();
        stats.heapUsed = heap.getUsed();
        stats.heapMax = heap.getMax();
        OperatingSystemMXBean os = ManagementFactory.getOperatingSystemMXBean();
        stats.systemLoad = os.getSystemLoadAverage();
        stats.processors = os.getAvailableProcessors();
        stats.processCpu = os instanceof com.sun.management.OperatingSystemMXBean
                ? ((com.sun.management.OperatingSystemMXBean) os).getProcessCpuLoad() : -1;
        stats.threads = ManagementFactory.getThreadMXBean().getThreadCount();
        stats.bridge = BridgeMetrics.get().snapshot();
        stats.backup = BackupStatus.read();
        stats.maintenance = Maintenance.get().plan();
        RustAiStats rust = new RustAiStats();
        rust.started = mage.game.trace.RustAiBridge.STARTED.get();
        rust.answered = mage.game.trace.RustAiBridge.ANSWERED.get();
        rust.stopped = mage.game.trace.RustAiBridge.STOPPED.get();
        rust.finished = mage.game.trace.RustAiBridge.FINISHED.get();
        rust.viewsCompared = mage.game.trace.RustAiBridge.VIEWS_COMPARED.get();
        rust.viewsDiffering = mage.game.trace.RustAiBridge.VIEWS_DIFFERING.get();
        rust.promptsCompared = mage.game.trace.RustAiBridge.PROMPTS_COMPARED.get();
        rust.promptsDiffering = mage.game.trace.RustAiBridge.PROMPTS_DIFFERING.get();
        rust.logLinesCompared = mage.game.trace.RustAiBridge.LOG_LINES_COMPARED.get();
        rust.logLinesDiffering = mage.game.trace.RustAiBridge.LOG_LINES_DIFFERING.get();
        stats.rustAi = rust;
        return stats;
    }
}
