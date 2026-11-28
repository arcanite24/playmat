package mage.server.websocket.api;

import mage.server.ladder.BotRoster;
import mage.server.support.PlayerLook;
import org.junit.jupiter.api.Test;

import java.util.Arrays;
import java.util.Collections;
import java.util.List;

import static org.assertj.core.api.Assertions.assertThat;

public class SupportApiBotLooksTest {

    @Test
    void botPersonaIsFilledForLowercasedName() {
        List<PlayerLook> looks = SupportApi.withBotLooks(
                Arrays.asList("brann blackwood", "BRANN BLACKWOOD", "unknown player"),
                Collections.emptyList(),
                BotRoster.get()
        );
        assertThat(looks).hasSize(1);
        PlayerLook brann = looks.get(0);
        assertThat(brann.name).isEqualTo("Brann Blackwood");
        assertThat(brann.avatar).isEqualTo("sphinx");
        assertThat(brann.sleeve).isEqualTo("scene-arena");
        assertThat(brann.cloth).isEqualTo("rose");
        assertThat(brann.frame).isEqualTo("frame-sun");
        assertThat(brann.title).isEqualTo("multicolor-champion");
    }

    @Test
    void humanLookTakesPrecedenceOverBotPersona() {
        PlayerLook human = new PlayerLook();
        human.name = "Brann Blackwood";
        human.avatar = "owl";
        List<PlayerLook> looks = SupportApi.withBotLooks(
                Collections.singletonList("brann blackwood"),
                Collections.singletonList(human),
                BotRoster.get()
        );
        assertThat(looks).hasSize(1);
        assertThat(looks.get(0).avatar).isEqualTo("owl");
    }
}
