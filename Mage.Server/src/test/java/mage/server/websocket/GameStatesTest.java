package mage.server.websocket;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import mage.interfaces.callback.ClientCallback;
import mage.interfaces.callback.ClientCallbackMethod;
import mage.view.GameClientMessage;
import org.java_websocket.WebSocket;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;

import static org.assertj.core.api.Assertions.assertThat;

/**
 * Web client bridge: game states go out as patches to clients that asked for them, and a client that applies them
 * always ends up with the complete state.
 */
public class GameStatesTest {

    private final List<String> sent = new ArrayList<>();
    private final UUID gameId = UUID.randomUUID();
    private final Client client = new Client();
    private GameStates states;
    private WebSocketCallbackHandler handler;
    private int messageId;

    /**
     * What a web client does with the messages: patches its copy of each game's view.
     */
    private static final class Client {
        final Map<String, Long> seqs = new HashMap<>();
        final Map<String, JsonElement> views = new HashMap<>();

        /** @return the message as the rest of the client sees it, with the complete view */
        JsonObject receive(String text) {
            JsonObject message = JsonParser.parseString(text).getAsJsonObject();
            JsonObject state = message.getAsJsonObject("state");
            if (state == null) {
                return message;
            }
            String gameId = message.get("objectId").getAsString();
            if (state.has("patch")) {
                assertThat(seqs.get(gameId)).as("patch base").isEqualTo(state.get("base").getAsLong());
                JsonElement view = StateDiff.apply(views.get(gameId), state.get("patch"));
                if (isViewMethod(message)) {
                    message.add("data", view);
                } else {
                    message.getAsJsonObject("data").add("gameView", view);
                }
            }
            JsonElement view = isViewMethod(message) ? message.get("data") : message.getAsJsonObject("data").get("gameView");
            seqs.put(gameId, state.get("seq").getAsLong());
            views.put(gameId, view.deepCopy());
            message.remove("state");
            return message;
        }

        static boolean isViewMethod(JsonObject message) {
            String method = message.get("method").getAsString();
            return method.equals("GAME_UPDATE") || method.equals("GAME_INIT") || method.startsWith("REPLAY_");
        }
    }

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
        states = new GameStates();
        states.setEnabled(true);
        handler = new WebSocketCallbackHandler(socket, states);
    }

    /** A game view stand-in: Gson serializes the map like a view object. */
    private static LinkedHashMap<String, Object> view(int turn, int life, String... hand) {
        LinkedHashMap<String, Object> view = new LinkedHashMap<>();
        view.put("turn", turn);
        view.put("phase", "PRECOMBAT_MAIN");
        Map<String, Object> cards = new LinkedHashMap<>();
        for (String name : hand) {
            Map<String, Object> card = new LinkedHashMap<>();
            card.put("name", name);
            card.put("rules", String.join(" ", java.util.Collections.nCopies(4, "A long rules text that makes the complete view clearly larger than a patch.")));
            cards.put("id-" + name, card);
        }
        Map<String, Object> player = new LinkedHashMap<>();
        player.put("life", life);
        player.put("hand", cards);
        List<Object> players = new ArrayList<>();
        players.add(player);
        players.add(new LinkedHashMap<>(player));
        view.put("players", players);
        return view;
    }

    private JsonObject push(ClientCallbackMethod method, Object data) throws Exception {
        ClientCallback callback = new ClientCallback(method, gameId, data);
        callback.setMessageId(messageId++);
        handler.send(callback);
        return client.receive(sent.get(sent.size() - 1));
    }

    private JsonObject lastRaw() {
        return JsonParser.parseString(sent.get(sent.size() - 1)).getAsJsonObject();
    }

    @Test
    void theFirstStateIsCompleteAndLaterOnesArePatches() throws Exception {
        LinkedHashMap<String, Object> first = view(1, 20, "Forest", "Shock");
        JsonObject received = push(ClientCallbackMethod.GAME_UPDATE, first);
        assertThat(lastRaw().getAsJsonObject("state").has("patch")).isFalse();
        assertThat(received.get("data").toString()).isEqualTo(new com.google.gson.Gson().toJson(first));

        LinkedHashMap<String, Object> second = view(2, 17, "Shock", "Mountain");
        received = push(ClientCallbackMethod.GAME_UPDATE, second);
        JsonObject raw = lastRaw();
        assertThat(raw.has("data")).isFalse();
        assertThat(raw.getAsJsonObject("state").get("patch").toString()).contains("\"id-Forest\":[]").contains("17");
        assertThat(sent.get(sent.size() - 1).length()).isLessThan(sent.get(0).length());
        assertThat(received.get("data").toString()).isEqualTo(mage.server.websocket.rpc.JsonCodec.GSON.toJson(second));

        // nothing changed: an empty patch
        push(ClientCallbackMethod.GAME_UPDATE, view(2, 17, "Shock", "Mountain"));
        assertThat(lastRaw().getAsJsonObject("state").get("patch").toString()).isEqualTo("{}");
    }

    @Test
    void aFieldThatComesBackIsPatchedInPlace() throws Exception {
        push(ClientCallbackMethod.GAME_UPDATE, view(1, 20, "Forest", "Shock"));
        // like canPlayObjects in a view built with priority: a field between others that was left out as null
        LinkedHashMap<String, Object> withPlayable = new LinkedHashMap<>();
        withPlayable.put("turn", 1);
        withPlayable.put("canPlay", "Forest");
        view(1, 20, "Forest", "Shock").forEach((key, value) -> withPlayable.putIfAbsent(key, value));
        JsonObject received = push(ClientCallbackMethod.GAME_UPDATE, withPlayable);
        assertThat(lastRaw().getAsJsonObject("state").get("patch").toString()).startsWith("[\"o\"");
        assertThat(received.get("data").toString()).isEqualTo(mage.server.websocket.rpc.JsonCodec.GSON.toJson(withPlayable));
    }

    @Test
    void viewsInsideMessagesArePatchedToo() throws Exception {
        push(ClientCallbackMethod.GAME_UPDATE, view(1, 20, "Forest"));
        JsonObject received = push(ClientCallbackMethod.GAME_ASK, question("Play a land?", view(1, 19, "Forest")));
        JsonObject raw = lastRaw();
        assertThat(raw.getAsJsonObject("state").has("patch")).isTrue();
        assertThat(raw.getAsJsonObject("data").has("gameView")).isFalse();
        assertThat(raw.getAsJsonObject("data").get("message").getAsString()).isEqualTo("Play a land?");
        assertThat(received.getAsJsonObject("data").getAsJsonObject("gameView").getAsJsonArray("players")
                .get(0).getAsJsonObject().get("life").getAsInt()).isEqualTo(19);
    }

    /** A GameClientMessage stand-in: its gameView field takes a real GameView. */
    private static Map<String, Object> question(String text, Object view) {
        Map<String, Object> message = new LinkedHashMap<>();
        message.put("gameView", view);
        message.put("message", text);
        return message;
    }

    @Test
    void largerPatchesAreSentComplete() throws Exception {
        LinkedHashMap<String, Object> small = new LinkedHashMap<>();
        small.put("a", 1);
        push(ClientCallbackMethod.GAME_UPDATE, small);
        LinkedHashMap<String, Object> other = new LinkedHashMap<>();
        other.put("b", 2);
        JsonObject received = push(ClientCallbackMethod.GAME_UPDATE, other);
        assertThat(lastRaw().getAsJsonObject("state").has("patch")).isFalse();
        assertThat(received.get("data").toString()).isEqualTo("{\"b\":2}");
    }

    @Test
    void longRunsOfPatchesStayExactAcrossSizeChecks() throws Exception {
        int patches = 0;
        for (int i = 0; i < GameStates.PATCHES_PER_SIZE_CHECK * 3; i++) {
            // small changes (life, turn, now and then a card drawn), so every state after the first is a patch
            LinkedHashMap<String, Object> next = i % 5 == 4
                    ? view(i / 4 + 1, 20 - i % 7, "Forest", "Shock", "Island")
                    : view(i / 4 + 1, 20 - i % 7, "Forest", "Shock");
            JsonObject received = push(ClientCallbackMethod.GAME_UPDATE, next);
            assertThat(received.get("data").toString()).isEqualTo(mage.server.websocket.rpc.JsonCodec.GSON.toJson(next));
            patches += lastRaw().getAsJsonObject("state").has("patch") ? 1 : 0;
        }
        assertThat(patches).isEqualTo(GameStates.PATCHES_PER_SIZE_CHECK * 3 - 1);
    }

    @Test
    void gameOverIsCompleteAndForgetsTheGame() throws Exception {
        push(ClientCallbackMethod.GAME_UPDATE, view(1, 20, "Forest"));
        push(ClientCallbackMethod.GAME_OVER, new GameClientMessage(null, new HashMap<>(), "You won"));
        assertThat(lastRaw().has("state")).isFalse();
        assertThat(states.size()).isZero();
        assertThat(handler.resendState(gameId)).isFalse();
    }

    @Test
    void aResyncSendsTheLastStateCompleteAndThenTheOpenQuestion() throws Exception {
        push(ClientCallbackMethod.GAME_UPDATE, view(1, 20, "Forest"));
        push(ClientCallbackMethod.GAME_UPDATE, view(1, 18, "Forest"));
        push(ClientCallbackMethod.GAME_SELECT, new GameClientMessage(null, new HashMap<>(), "Pass priority?"));
        sent.clear();
        client.seqs.clear(); // the client lost track

        assertThat(handler.resendState(gameId)).isTrue();
        assertThat(sent).hasSize(2);
        JsonObject update = client.receive(sent.get(0));
        assertThat(update.get("method").getAsString()).isEqualTo("GAME_UPDATE");
        assertThat(update.get("messageId").getAsInt()).isEqualTo(2); // the newest sent: never dropped as outdated
        assertThat(update.getAsJsonObject("data").getAsJsonArray("players").get(0).getAsJsonObject().get("life").getAsInt())
                .isEqualTo(18);
        assertThat(client.receive(sent.get(1)).get("method").getAsString()).isEqualTo("GAME_SELECT");

        // patches continue from the resent state
        push(ClientCallbackMethod.GAME_UPDATE, view(2, 18, "Forest"));
        assertThat(lastRaw().getAsJsonObject("state").has("patch")).isTrue();
    }

    @Test
    void aResentQuestionIsCompleteEvenAfterLaterPatches() throws Exception {
        push(ClientCallbackMethod.GAME_SELECT, new GameClientMessage(null, new HashMap<>(), "Pass priority?"));
        push(ClientCallbackMethod.GAME_UPDATE, view(1, 20, "Forest"));
        push(ClientCallbackMethod.GAME_UPDATE, view(1, 19, "Forest"));
        sent.clear();
        assertThat(handler.resendPrompt(gameId)).isTrue();
        assertThat(client.receive(sent.get(0)).getAsJsonObject("data").get("message").getAsString()).isEqualTo("Pass priority?");
    }

    @Test
    void onlyAFewGamesAreRemembered() throws Exception {
        for (int i = 0; i < GameStates.MAX_GAMES + 3; i++) {
            ClientCallback callback = new ClientCallback(ClientCallbackMethod.GAME_UPDATE, UUID.randomUUID(), view(1, 20, "Forest"));
            handler.send(callback);
        }
        assertThat(states.size()).isEqualTo(GameStates.MAX_GAMES);
        states.newSession();
        assertThat(states.size()).isZero();
    }

    @Test
    void clientsWithoutPatchesGetTheSameMessagesAsBefore() throws Exception {
        states.setEnabled(false);
        ClientCallback callback = new ClientCallback(ClientCallbackMethod.GAME_UPDATE, gameId, view(1, 20, "Forest"));
        handler.send(callback);
        handler.send(callback);
        assertThat(sent).hasSize(2);
        assertThat(sent.get(1)).isEqualTo(WebSocketCallbackHandler.serialize(callback)).doesNotContain("\"state\"");
        assertThat(states.size()).isZero();
    }
}
