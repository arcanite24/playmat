package mage.server;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

/** The Update button's side of the files it shares with the updater (ops/web/updater/playmat-updater). */
public class UpdateRunnerTest {

    @TempDir
    Path dir;
    private long now = 1_000_000L;
    private UpdateRunner runner;

    @BeforeEach
    void setUp() {
        runner = new UpdateRunner(dir, () -> now);
    }

    private void file(String name, String text) throws IOException {
        Files.write(dir.resolve(name), text.getBytes(StandardCharsets.UTF_8));
    }

    private void heartbeat(boolean ready, String previous) throws IOException {
        file("updater.json", "{\"version\":\"sha-1234567\",\"ready\":" + ready + ",\"problem\":" + (ready ? "null" : "\"needs Linux\"")
                + ",\"current\":\"sha-1234567\",\"previous\":" + (previous == null ? "null" : "\"" + previous + "\"")
                + ",\"running\":false,\"at\":" + now + "}");
    }

    private JsonObject request() throws IOException {
        return JsonParser.parseString(new String(Files.readAllBytes(dir.resolve("request.json")), StandardCharsets.UTF_8)).getAsJsonObject();
    }

    @Test
    public void noUpdaterRefuses() {
        UpdateRunner.UpdaterInfo info = runner.info();
        assertThat(info.running).isFalse();
        assertThat(info.job).isNull();
        assertThatThrownBy(() -> runner.request(UpdateRunner.UPDATE, "sha-abcdef1", "neri")).hasMessageContaining("isn't running");
        assertThat(dir.resolve("request.json")).doesNotExist();
    }

    @Test
    public void aStaleHeartbeatIsNoUpdater() throws IOException {
        heartbeat(true, null);
        now += UpdateRunner.HEARTBEAT_SECONDS + 1;
        assertThat(runner.info().running).isFalse();
    }

    @Test
    public void anUpdaterThatCantWorkSaysWhy() throws IOException {
        heartbeat(false, null);
        assertThat(runner.info().problem).isEqualTo("needs Linux");
        assertThatThrownBy(() -> runner.request(UpdateRunner.UPDATE, "sha-abcdef1", "neri")).hasMessage("needs Linux");
    }

    @Test
    public void anUpdateIsWrittenForTheUpdaterAndBlocksAnother() throws IOException {
        heartbeat(true, null);
        String id = runner.request(UpdateRunner.UPDATE, "sha-abcdef1", "neri");
        JsonObject request = request();
        assertThat(request.get("id").getAsString()).isEqualTo(id);
        assertThat(request.get("action").getAsString()).isEqualTo("update");
        assertThat(request.get("tag").getAsString()).isEqualTo("sha-abcdef1");
        assertThat(request.get("by").getAsString()).isEqualTo("neri");
        assertThat(runner.info().busy).isTrue();
        assertThatThrownBy(() -> runner.request(UpdateRunner.UPDATE, "sha-abcdef1", "neri")).hasMessageContaining("under way");
    }

    @Test
    public void onlyVersionsTheUpdaterAcceptsAreAsked() throws IOException {
        heartbeat(true, null);
        assertThatThrownBy(() -> runner.request(UpdateRunner.UPDATE, "evil; rm -rf /", "neri")).isInstanceOf(IllegalArgumentException.class);
        assertThatThrownBy(() -> runner.request(UpdateRunner.UPDATE, null, "neri")).isInstanceOf(IllegalArgumentException.class);
        assertThatThrownBy(() -> runner.request("shell", "sha-abcdef1", "neri")).isInstanceOf(IllegalArgumentException.class);
        assertThat(dir.resolve("request.json")).doesNotExist();
    }

    @Test
    public void aRunningJobIsBusyAndShowsItsLog() throws IOException {
        heartbeat(true, null);
        file("status.json", "{\"id\":\"j1\",\"action\":\"update\",\"tag\":\"sha-abcdef1\",\"state\":\"running\",\"step\":\"drain\",\"startedAt\":" + now + "}");
        file("update.log", "one\ntwo\n");
        UpdateRunner.UpdaterInfo info = runner.info();
        assertThat(info.busy).isTrue();
        assertThat(info.job.step).isEqualTo("drain");
        assertThat(info.log).containsExactly("one", "two");
        assertThatThrownBy(() -> runner.request(UpdateRunner.UPDATE, "sha-abcdef1", "neri")).hasMessageContaining("under way");
    }

    @Test
    public void rollbackNeedsAnEarlierVersion() throws IOException {
        heartbeat(true, null);
        assertThatThrownBy(() -> runner.request(UpdateRunner.ROLLBACK, null, "neri")).hasMessageContaining("no earlier version");
        heartbeat(true, "sha-7654321");
        runner.request(UpdateRunner.ROLLBACK, null, "neri");
        assertThat(request().get("action").getAsString()).isEqualTo("rollback");
        assertThat(request().has("tag")).isFalse();
    }

    @Test
    public void aRequestTheUpdaterNeverTookStopsBlocking() throws IOException {
        heartbeat(true, null);
        runner.request(UpdateRunner.UPDATE, "sha-abcdef1", "neri");
        Files.setLastModifiedTime(dir.resolve("request.json"), java.nio.file.attribute.FileTime.fromMillis((now - UpdateRunner.REQUEST_SECONDS - 5) * 1000));
        assertThat(runner.info().busy).isFalse();
    }

    @Test
    public void aFinishedJobIsToldAboutOnce() throws IOException {
        file("status.json", "{\"id\":\"j1\",\"action\":\"update\",\"tag\":\"sha-abcdef1\",\"by\":\"neri\",\"state\":\"ok\",\"message\":\"Updated.\",\"startedAt\":1,\"finishedAt\":2}");
        assertThat(runner.newsOfFinishedJob()).isEqualTo("Update to sha-abcdef1 (asked by neri): done. Updated.");
        assertThat(runner.newsOfFinishedJob()).isNull();
        file("status.json", "{\"id\":\"j2\",\"action\":\"rollback\",\"state\":\"running\",\"startedAt\":3}");
        assertThat(runner.newsOfFinishedJob()).isNull();
    }
}
