package mage.server.admin;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

public class RuntimeSettingsTest {

    @TempDir
    Path dir;
    private AdminStore store;
    private RuntimeSettings settings;

    @BeforeEach
    void open() throws Exception {
        store = new AdminStore("jdbc:sqlite:" + dir.resolve("admin.db"));
        settings = RuntimeSettings.create(store);
    }

    @Test
    void followTheConfigUntilChanged() throws Exception {
        assertThat(settings.registrationOpen()).isTrue();
        assertThat(settings.careerOn()).isTrue();
        assertThat(settings.discord(RuntimeSettings.DISCORD_TABLES)).isTrue();
        assertThat(settings.maxAiOpponents("3")).isEqualTo("3");
        assertThat(settings.maxSecondsIdle(300)).isEqualTo(300);

        settings.set(RuntimeSettings.REGISTRATION, " Invite ", "ana", 1000);
        settings.set(RuntimeSettings.MAX_AI_OPPONENTS, "1", "ana", 1000);
        settings.set(RuntimeSettings.MAX_SECONDS_IDLE, "120", "ana", 1000);
        settings.set(RuntimeSettings.DISCORD_TABLES, "off", "ana", 1000);
        assertThat(settings.registrationOpen()).isFalse();
        assertThat(settings.maxAiOpponents("3")).isEqualTo("1");
        assertThat(settings.maxSecondsIdle(300)).isEqualTo(120);
        assertThat(settings.discord(RuntimeSettings.DISCORD_TABLES)).isFalse();
        assertThat(settings.row(RuntimeSettings.REGISTRATION).changedBy).isEqualTo("ana");

        settings.set(RuntimeSettings.MAX_AI_OPPONENTS, "", "ana", 2000);
        assertThat(settings.maxAiOpponents("3")).as("empty goes back to the config").isEqualTo("3");
    }

    @Test
    void survivesARestart() throws Exception {
        settings.set(RuntimeSettings.CAREER, "off", "ana", 1000);
        assertThat(RuntimeSettings.create(store).careerOn()).isFalse();
    }

    @Test
    void refusesWhatASettingCantTake() {
        assertThatThrownBy(() -> settings.set("colour", "red", "ana", 1)).isInstanceOf(IllegalArgumentException.class);
        assertThatThrownBy(() -> settings.set(RuntimeSettings.CAREER, "maybe", "ana", 1)).isInstanceOf(IllegalArgumentException.class);
        assertThatThrownBy(() -> settings.set(RuntimeSettings.MAX_SECONDS_IDLE, "5", "ana", 1)).hasMessageContaining("30 to 3600");
        assertThatThrownBy(() -> settings.set(RuntimeSettings.MAX_AI_OPPONENTS, "lots", "ana", 1)).isInstanceOf(IllegalArgumentException.class);
    }

    @Test
    void joinPostsWaitToBeTurnedOnAndTheDirectoryIsOn() throws Exception {
        assertThat(settings.discord(RuntimeSettings.DISCORD_JOINS)).as("a busy server would flood the channel").isFalse();
        assertThat(settings.directoryOn()).isTrue();

        settings.set(RuntimeSettings.DISCORD_JOINS, "on", "ana", 1000);
        settings.set(RuntimeSettings.DIRECTORY, "off", "ana", 1000);
        assertThat(settings.discord(RuntimeSettings.DISCORD_JOINS)).isTrue();
        assertThat(settings.directoryOn()).isFalse();
        assertThatThrownBy(() -> settings.set(RuntimeSettings.DIRECTORY, "hidden", "ana", 1)).isInstanceOf(IllegalArgumentException.class);
    }
}
