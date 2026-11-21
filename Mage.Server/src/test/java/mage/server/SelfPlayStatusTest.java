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
import java.time.Instant;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

/** The Self-play section's side of the files it shares with the worker (ops/web/host/rl-worker.sh). */
public class SelfPlayStatusTest {

    @TempDir
    Path dir;
    /** 2026-11-21 20:30 UTC */
    private long now = 1_795_293_000L;
    private SelfPlayStatus status;

    @BeforeEach
    void setUp() {
        status = new SelfPlayStatus(dir, () -> now);
    }

    private void file(String name, String text) throws IOException {
        Files.write(dir.resolve(name), text.getBytes(StandardCharsets.UTF_8));
    }

    private static String iso(long epoch) {
        return Instant.ofEpochSecond(epoch).toString();
    }

    private static String chunk(long at, String outcome, int games) {
        return "{\"at\": \"" + iso(at) + "\", \"job\": \"loop5-r1\", \"chunk\": \"c000001\", \"outcome\": \"" + outcome
                + "\", \"games\": " + games + ", \"seconds\": 80, \"reason\": \"" + ("done".equals(outcome) ? "" : "the server uses 33% CPU") + "\"}\n";
    }

    @Test
    public void noVolumeIsNotInstalled() {
        status = new SelfPlayStatus(dir.resolve("missing"), () -> now);
        SelfPlayStatus.SelfPlayView view = status.view();
        assertThat(view.installed).isFalse();
        assertThat(view.running).isFalse();
        assertThat(view.hours).isEmpty();
        assertThatThrownBy(() -> status.save(true, null, null, null, null, "neri")).hasMessageContaining("isn't set up");
    }

    @Test
    public void readsTheWorkerAndKnowsWhenItStopped() throws IOException {
        file("worker.json", "{\"at\": \"" + iso(now - 60) + "\", \"state\": \"playing\", \"detail\": \"job loop5-r1 c000003\","
                + " \"job\": {\"id\": \"loop5-r1\", \"image\": \"ghcr.io/x@sha256:00\", \"tag\": 1, \"seedBase\": 1500000},"
                + " \"settings\": {\"paused\": false, \"quietUtc\": \"\", \"cpuMax\": 25, \"gamesMax\": 4, \"jobs\": 3, \"chunkGames\": 150},"
                + " \"spoolMb\": 3, \"chunksWaiting\": 2, \"totals\": {\"games\": 450, \"chunksDone\": 3, \"chunksStopped\": 1}}");
        SelfPlayStatus.SelfPlayView view = status.view();
        assertThat(view.installed).isTrue();
        assertThat(view.running).isTrue();
        assertThat(view.state).isEqualTo("playing");
        assertThat(view.job.id).isEqualTo("loop5-r1");
        assertThat(view.job.seedBase).isEqualTo(1_500_000);
        assertThat(view.effective.cpuMax).isEqualTo(25);
        assertThat(view.chunksWaiting).isEqualTo(2);
        assertThat(view.totals.games).isEqualTo(450);
        assertThat(view.console).isNull();

        now += SelfPlayStatus.STALE_SECONDS + 61;
        assertThat(status.view().running).isFalse();
    }

    @Test
    public void bucketsTheDayAndSumsTheWeek() throws IOException {
        long hourStart = now / 3600 * 3600;
        file("history.jsonl", chunk(now - 8 * 86400L, "done", 150) // too old for the week
                + chunk(now - 3 * 86400L, "done", 150) // the week only
                + chunk(hourStart - 3600 + 10, "done", 150) // the hour before this one
                + chunk(hourStart + 5, "done", 150)
                + chunk(hourStart + 60, "stopped", 0)
                + "{\"at\": \"half a li");
        SelfPlayStatus.SelfPlayView view = status.view();
        assertThat(view.games7d).isEqualTo(450);
        assertThat(view.games24h).isEqualTo(300);
        assertThat(view.chunksDone24h).isEqualTo(2);
        assertThat(view.chunksStopped24h).isEqualTo(1);
        assertThat(view.hours).hasSize(SelfPlayStatus.HOURS);
        SelfPlayStatus.SelfPlayHour last = view.hours.get(SelfPlayStatus.HOURS - 1);
        assertThat(last.start).isEqualTo(hourStart);
        assertThat(last.games).isEqualTo(150);
        assertThat(last.chunksStopped).isEqualTo(1);
        assertThat(view.hours.get(SelfPlayStatus.HOURS - 2).games).isEqualTo(150);
        assertThat(view.hours.get(0).games).isZero();
        // newest first; the half-written line is skipped
        assertThat(view.recent).hasSize(5);
        assertThat(view.recent.get(0).outcome).isEqualTo("stopped");
        assertThat(view.recent.get(0).reason).contains("33%");
        assertThat(view.recent.get(1).reason).isNull();
    }

    @Test
    public void readsTheTrainerNewestRoundFirst() throws IOException {
        file("trainer.json", "{\"run\": \"loop5\", \"round\": 2, \"rounds\": 3, \"phase\": \"train\", \"at\": \"" + iso(now - 30)
                + "\", \"local_games\": 20000, \"remote_games\": 2400, \"history\": ["
                + "{\"round\": 1, \"accepted\": true, \"elo\": \"elo +5.2 [-2.6, +13.1]\", \"remote_games\": 1800,"
                + " \"selfplay_s\": 2500, \"train_s\": 1600, \"sprt_s\": 160},"
                + "{\"round\": 2, \"accepted\": false, \"elo\": null, \"remote_games\": null, \"selfplay_s\": 2400}]}");
        SelfPlayStatus.SelfPlayTrainer trainer = status.view().trainer;
        assertThat(trainer.run).isEqualTo("loop5");
        assertThat(trainer.phase).isEqualTo("train");
        assertThat(trainer.at).isEqualTo(now - 30);
        assertThat(trainer.localGames).isEqualTo(20000);
        assertThat(trainer.remoteGames).isEqualTo(2400);
        assertThat(trainer.history).hasSize(2);
        assertThat(trainer.history.get(0).round).isEqualTo(2);
        assertThat(trainer.history.get(1).elo).isEqualTo("elo +5.2 [-2.6, +13.1]");
        assertThat(trainer.history.get(1).trainSeconds).isEqualTo(1600);
    }

    @Test
    public void junkFilesReadAsNothing() throws IOException {
        file("worker.json", "not json");
        file("trainer.json", "[1, 2]");
        SelfPlayStatus.SelfPlayView view = status.view();
        assertThat(view.installed).isTrue();
        assertThat(view.running).isFalse();
        assertThat(view.state).isNull();
        assertThat(view.trainer).isNull();
    }

    @Test
    public void savesSettingsTheWorkerReads() throws IOException {
        SelfPlayStatus.SelfPlayView view = status.save(true, " 02-12 ", 40, null, 2, "neri");
        JsonObject saved = JsonParser.parseString(new String(Files.readAllBytes(dir.resolve("settings.json")), StandardCharsets.UTF_8))
                .getAsJsonObject();
        assertThat(saved.get("paused").getAsBoolean()).isTrue();
        assertThat(saved.get("quiet_utc").getAsString()).isEqualTo("02-12");
        assertThat(saved.get("cpu_max").getAsInt()).isEqualTo(40);
        assertThat(saved.has("games_max")).isFalse();
        assertThat(saved.get("jobs").getAsInt()).isEqualTo(2);
        assertThat(saved.get("by").getAsString()).isEqualTo("neri");
        assertThat(view.console.paused).isTrue();
        assertThat(view.console.quietUtc).isEqualTo("02-12");
        assertThat(view.console.gamesMax).isNull();
        assertThat(dir.resolve(".settings.json.tmp")).doesNotExist();

        status.save(false, "", null, null, null, "neri");
        assertThat(status.view().console.quietUtc).isEmpty();
    }

    @Test
    public void refusesOutOfRangeSettings() {
        assertThatThrownBy(() -> status.save(false, "25-12", null, null, null, "neri")).hasMessageContaining("HH-HH");
        assertThatThrownBy(() -> status.save(false, "02-12; rm", null, null, null, "neri")).hasMessageContaining("HH-HH");
        assertThatThrownBy(() -> status.save(false, null, 4, null, null, "neri")).hasMessageContaining("CPU");
        assertThatThrownBy(() -> status.save(false, null, 201, null, null, "neri")).hasMessageContaining("CPU");
        assertThatThrownBy(() -> status.save(false, null, null, 51, null, "neri")).hasMessageContaining("open games");
        assertThatThrownBy(() -> status.save(false, null, null, null, 4, "neri")).hasMessageContaining("Cores");
        assertThat(dir.resolve("settings.json")).doesNotExist();
        assertThat(status.save(false, "02:30-11:15", 5, 0, 1, "neri").console.cpuMax).isEqualTo(5);
    }
}
