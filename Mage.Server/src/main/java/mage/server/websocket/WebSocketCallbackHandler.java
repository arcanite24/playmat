package mage.server.websocket;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import mage.interfaces.callback.ClientCallback;
import mage.interfaces.callback.ClientCallbackMethod;
import mage.server.ClientChannel;
import mage.server.websocket.rpc.JsonCodec;
import org.java_websocket.WebSocket;

import java.util.EnumSet;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.Executor;

/**
 * Web client bridge: pushes server callbacks (game updates, prompts, chat...) to a WebSocket as JSON.
 * <p>
 * Message shape: {@code {"method": "GAME_UPDATE", "objectId": "...", "messageId": 12, "data": {...}}}, plus a
 * {@code state} field for clients that take state patches (see {@link GameStates}).
 */
public class WebSocketCallbackHandler implements ClientChannel {

    private static final Set<ClientCallbackMethod> PROMPTS = EnumSet.of(
            ClientCallbackMethod.GAME_TARGET, ClientCallbackMethod.GAME_CHOOSE_ABILITY, ClientCallbackMethod.GAME_CHOOSE_PILE,
            ClientCallbackMethod.GAME_CHOOSE_CHOICE, ClientCallbackMethod.GAME_ASK, ClientCallbackMethod.GAME_SELECT,
            ClientCallbackMethod.GAME_PLAY_MANA, ClientCallbackMethod.GAME_PLAY_XMANA, ClientCallbackMethod.GAME_GET_AMOUNT,
            ClientCallbackMethod.GAME_GET_MULTI_AMOUNT);

    private final WebSocket conn;
    /**
     * The last question of each game this connection has not answered yet, as sent (complete, without a state
     * patch), for {@code gameResync}.
     */
    private final Map<UUID, Prompt> pendingPrompts = new ConcurrentHashMap<>();

    /**
     * A question as sent complete. For clients with patches it is kept as its tree (which the game's baseline shares),
     * and written out only if it has to be sent again, which is rare.
     */
    private static final class Prompt {
        private final JsonObject tree;
        private String json;

        Prompt(JsonObject tree, String json) {
            this.tree = tree;
            this.json = json;
        }

        synchronized String json() {
            if (json == null) {
                json = JsonCodec.GSON.toJson(tree);
            }
            return json;
        }

        JsonObject tree() {
            return tree != null ? tree : JsonParser.parseString(json).getAsJsonObject();
        }
    }
    /** the connection's game states, for clients that take patches; its lock orders encoding and sending */
    private final GameStates states;
    /** diffs, writes and sends the game messages of clients with patches (see {@link SerialSender}) */
    private final Executor sender;

    public WebSocketCallbackHandler(WebSocket conn) {
        this(conn, new GameStates());
    }

    WebSocketCallbackHandler(WebSocket conn, GameStates states) {
        this(conn, states, SerialSender.DIRECT);
    }

    WebSocketCallbackHandler(WebSocket conn, GameStates states, Executor sender) {
        this.conn = conn;
        this.states = states;
        this.sender = sender;
    }

    /** whether this connection's gateway hosts games (it asked, from a trusted address) */
    private volatile java.util.function.BooleanSupplier hostsGames = () -> false;

    void setHostsGames(java.util.function.BooleanSupplier hostsGames) {
        this.hostsGames = hostsGames;
    }

    @Override
    public boolean hostsGames() {
        return hostsGames.getAsBoolean();
    }

    /**
     * The player answered (or passed): the question is no longer open.
     */
    void promptAnswered(UUID gameId) {
        if (gameId != null) {
            pendingPrompts.remove(gameId);
        }
    }

    /**
     * Sends the game's open question again (a reply or the question itself was lost on the way).
     *
     * @return false when nothing is waiting for an answer from this player
     */
    boolean resendPrompt(UUID gameId) {
        Prompt prompt = gameId == null ? null : pendingPrompts.get(gameId);
        if (prompt == null || !conn.isOpen()) {
            return false;
        }
        synchronized (states) {
            if (!states.isEnabled()) {
                WebSocketServerImpl.sendText(conn, prompt.json());
                return true;
            }
            // the question's view becomes the client's latest state again
            JsonObject message = prompt.tree();
            ClientCallbackMethod method = ClientCallbackMethod.valueOf(message.get("method").getAsString());
            GameStates.Encoded encoded = states.encode(method, gameId, message);
            sendState(encoded == null ? prompt.json() : encoded.json, encoded);
        }
        return true;
    }

    /**
     * The client could not apply a state patch: sends the game's last state complete, then its open question.
     *
     * @return false when nothing is remembered for the game (its next state goes out complete anyway)
     */
    boolean resendState(UUID gameId) {
        synchronized (states) {
            String json = states.resync(gameId);
            if (json == null || !conn.isOpen()) {
                return false;
            }
            BridgeMetrics.get().stateResync();
            WebSocketServerImpl.sendText(conn, json);
            resendPrompt(gameId);
            return true;
        }
    }

    @Override
    public void send(ClientCallback clientCallback) throws Closed {
        if (!conn.isOpen()) {
            // lets the session mark itself invalid and disconnect, instead of silently dropping data
            throw new Closed("WebSocket connection is closed");
        }
        clientCallback.decompressData(); // no-op unless the callback was compressed
        try {
            if (states.isEnabled()) {
                // the views are read here, on the caller's thread (usually the game's), as they were built; the
                // tree that comes out belongs to this message alone, so the rest can happen on the sender
                JsonObject message = serializeTree(clientCallback);
                ClientCallbackMethod method = clientCallback.getMethod();
                UUID gameId = clientCallback.getObjectId();
                int messageId = clientCallback.getMessageId();
                sender.execute(() -> sendWithState(method, gameId, messageId, message));
                return;
            }
            String json = serialize(clientCallback);
            BridgeMetrics.get().callback(clientCallback.getMethod().name(), json.length(), clientCallback.getObjectId());
            remember(clientCallback.getMethod(), clientCallback.getObjectId(), new Prompt(null, json));
            WebSocketServerImpl.sendText(conn, json);
        } catch (Exception e) {
            throw new Closed("Error sending WebSocket message", e);
        }
    }

    /**
     * For clients that take state patches: game views go out as patches against the last state sent, when smaller.
     */
    private void sendWithState(ClientCallbackMethod method, UUID gameId, int messageId, JsonObject message) {
        if (!conn.isOpen()) {
            return;
        }
        synchronized (states) {
            // (null when the client turned patches off meanwhile)
            GameStates.Encoded encoded = states.isEnabled() ? states.encode(method, gameId, message) : null;
            String json = encoded == null ? JsonCodec.GSON.toJson(message) : encoded.json;
            BridgeMetrics.get().callback(method.name(), json.length(), gameId);
            states.sent(messageId);
            // a question is kept complete (encoding leaves the tree as it is): when it is sent again, the client may
            // have moved past this state
            remember(method, gameId, new Prompt(message, encoded == null ? json : null));
            sendState(json, encoded);
        }
    }

    private void sendState(String json, GameStates.Encoded encoded) {
        if (encoded != null) {
            BridgeMetrics.get().state(encoded.patch, encoded.fullChars, json.length());
        }
        WebSocketServerImpl.sendText(conn, json);
    }

    private void remember(ClientCallbackMethod method, UUID gameId, Prompt json) {
        if (gameId == null) {
            return;
        }
        if (PROMPTS.contains(method)) {
            pendingPrompts.put(gameId, json);
        } else if (method == ClientCallbackMethod.GAME_OVER) {
            pendingPrompts.remove(gameId);
        }
    }

    /**
     * The JSON message for a callback; pre-game prompts get a marker for the web client (see {@link PromptMarkers}), and
     * the end of game summary leaves the players' zones out (see {@link EndGameInfo}).
     */
    static String serialize(ClientCallback clientCallback) {
        ClientCallbackMethod method = clientCallback.getMethod();
        if (method != ClientCallbackMethod.GAME_ASK && method != ClientCallbackMethod.GAME_TARGET
                && method != ClientCallbackMethod.END_GAME_INFO) {
            return JsonCodec.GSON.toJson(clientCallback);
        }
        return JsonCodec.GSON.toJson(serializeTree(clientCallback));
    }

    /**
     * Same as {@link #serialize}, as a tree.
     */
    static JsonObject serializeTree(ClientCallback clientCallback) {
        JsonObject json = JsonCodec.GSON.toJsonTree(clientCallback).getAsJsonObject();
        ClientCallbackMethod method = clientCallback.getMethod();
        if (method == ClientCallbackMethod.GAME_ASK || method == ClientCallbackMethod.GAME_TARGET) {
            PromptMarkers.annotate(json);
        } else if (method == ClientCallbackMethod.END_GAME_INFO) {
            EndGameInfo.slim(json);
        }
        return json;
    }

}
