package mage.server.websocket;

import mage.interfaces.callback.ClientCallback;
import mage.interfaces.callback.ClientCallbackMethod;
import mage.view.GameClientMessage;
import org.java_websocket.WebSocket;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.UUID;

import static org.assertj.core.api.Assertions.assertThat;

/**
 * Web client bridge: a lost question can be sent again ({@code gameResync}) until the player answers it.
 */
public class PendingPromptTest {

    private final List<String> sent = new ArrayList<>();
    private final UUID gameId = UUID.randomUUID();
    private WebSocketCallbackHandler handler;

    @BeforeEach
    void setUp() {
        WebSocket socket = (WebSocket) Proxy.newProxyInstance(WebSocket.class.getClassLoader(), new Class<?>[]{WebSocket.class},
                (proxy, method, args) -> {
                    switch (method.getName()) {
                        case "isOpen":
                            return true;
                        case "send":
                            sent.add((String) args[0]);
                            return null;
                        default:
                            return null;
                    }
                });
        handler = new WebSocketCallbackHandler(socket);
    }

    private void push(ClientCallbackMethod method, String message) throws Exception {
        handler.send(new ClientCallback(method, gameId, new GameClientMessage(null, new HashMap<>(), message)));
    }

    @Test
    void theOpenQuestionIsSentAgain() throws Exception {
        push(ClientCallbackMethod.GAME_SELECT, "first");
        push(ClientCallbackMethod.GAME_ASK, "second");
        sent.clear();

        assertThat(handler.resendPrompt(gameId)).isTrue();
        assertThat(sent).hasSize(1);
        assertThat(sent.get(0)).contains("GAME_ASK").contains("second");
    }

    @Test
    void answeredQuestionsAreNotSentAgain() throws Exception {
        push(ClientCallbackMethod.GAME_SELECT, "pass?");
        handler.promptAnswered(gameId);
        sent.clear();

        assertThat(handler.resendPrompt(gameId)).isFalse();
        assertThat(sent).isEmpty();
    }

    @Test
    void updatesKeepTheQuestionAndGameOverEndsIt() throws Exception {
        push(ClientCallbackMethod.GAME_TARGET, "target");
        push(ClientCallbackMethod.GAME_UPDATE_AND_INFORM, "Bob casts a spell");
        assertThat(handler.resendPrompt(gameId)).isTrue();

        push(ClientCallbackMethod.GAME_OVER, "You won");
        assertThat(handler.resendPrompt(gameId)).isFalse();
        assertThat(handler.resendPrompt(UUID.randomUUID())).isFalse();
    }
}
