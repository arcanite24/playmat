package mage.server;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.concurrent.TimeUnit;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

/** The build's commit, the backup status and the maintenance gate. */
public class OperationsTest {

    @TempDir
    Path dir;

    @Test
    void readsTheCommitTheBuildWrote() throws Exception {
        Path file = dir.resolve(".commit");
        assertThat(Health.readCommitFile(file)).isEmpty();
        Files.write(file, "0123456789abcdef0123456789abcdef01234567\n".getBytes(StandardCharsets.UTF_8));
        assertThat(Health.readCommitFile(file)).isEqualTo("0123456789abcdef0123456789abcdef01234567");
        Files.write(file, "ref: refs/heads/master".getBytes(StandardCharsets.UTF_8));
        assertThat(Health.readCommitFile(file)).as("not a commit").isEmpty();
    }

    @Test
    void backupStatusSaysWhatIsWrong() throws Exception {
        long now = TimeUnit.DAYS.toMillis(100);
        Path file = dir.resolve("backup-status.json");
        assertThat(BackupStatus.read(file, now)).as("no backup service").isNull();

        long hourAgo = TimeUnit.MILLISECONDS.toSeconds(now) - 3600;
        write(file, "{\"at\":" + hourAgo + ",\"ok\":true,\"name\":\"20261011T033000Z\",\"size\":\"4.0K\",\"lastOkAt\":" + hourAgo + "}");
        BackupStatus.BackupRun ok = BackupStatus.read(file, now);
        assertThat(ok.problem).isNull();
        assertThat(ok.size).isEqualTo("4.0K");

        write(file, "{\"at\":" + hourAgo + ",\"ok\":false,\"name\":\"20261011T033000Z\",\"size\":\"\",\"lastOkAt\":" + (hourAgo - 86400) + "}");
        assertThat(BackupStatus.read(file, now).problem).startsWith("The last backup failed");

        // backup.sh skips a run on a nearly full disk, and says why
        write(file, "{\"at\":" + hourAgo + ",\"ok\":false,\"name\":\"20261011T033000Z\",\"size\":\"\",\"lastOkAt\":" + (hourAgo - 86400)
                + ",\"skipped\":\"only 900 MB free on the backup disk, a backup needs 6400 MB (twice the last one)\"}");
        assertThat(BackupStatus.read(file, now).problem).isEqualTo("The last backup was skipped: only 900 MB free on the backup"
                + " disk, a backup needs 6400 MB (twice the last one)");

        long old = hourAgo - TimeUnit.HOURS.toSeconds(40);
        write(file, "{\"at\":" + old + ",\"ok\":true,\"name\":\"x\",\"size\":\"1K\",\"lastOkAt\":" + old + "}");
        assertThat(BackupStatus.read(file, now).problem).contains("41 hours old");

        write(file, "not json");
        assertThat(BackupStatus.read(file, now).problem).contains("can't be read");
    }

    @Test
    void noNewTablesWhileMaintenanceIsPlanned() throws Exception {
        Maintenance maintenance = Maintenance.get();
        assertThat(maintenance.isPlanned()).isFalse();
        maintenance.refuseNewTables();
        // announcing needs a running server to warn its players; the gate itself only looks at the plan
        java.lang.reflect.Field plan = Maintenance.class.getDeclaredField("plan");
        plan.setAccessible(true);
        Maintenance.MaintenancePlan planned = new Maintenance.MaintenancePlan();
        planned.message = "Back at 10.";
        plan.set(maintenance, planned);
        try {
            assertThatThrownBy(maintenance::refuseNewTables).hasMessageContaining("maintenance").hasMessageContaining("Back at 10.");
        } finally {
            plan.set(maintenance, null);
        }
    }

    private static void write(Path file, String text) throws Exception {
        Files.write(file, text.getBytes(StandardCharsets.UTF_8));
    }
}
