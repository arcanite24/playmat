package mage.server.websocket.rpc;

import org.jboss.remoting.callback.InvokerCallbackHandler;

import java.util.Collections;
import java.util.List;
import java.util.UUID;

/**
 * Web client bridge: what the dispatcher may know about the calling connection.
 */
public interface RpcConnection {

    /**
     * @return the server-issued session bound to this connection, or null before login
     */
    String getSessionId();

    void bindSession(String sessionId);

    /**
     * @return the user this connection last logged in as, kept after its session ends (to tell why it ended); null
     * before a login
     */
    default String getUserName() {
        return null;
    }

    default void setUserName(String userName) {
    }

    /**
     * @return remote IP address, used by the server to identify anonymous users on reconnect
     */
    String getRemoteHost();

    /**
     * @return a callback handler that pushes server events to this connection
     */
    InvokerCallbackHandler createCallbackHandler();

    /**
     * The player answered the open question of a game (see {@link #resendPrompt}).
     */
    default void promptAnswered(UUID gameId) {
    }

    /**
     * Sends the game's unanswered question to this connection again.
     *
     * @return false when no question is waiting for an answer
     */
    default boolean resendPrompt(UUID gameId) {
        return false;
    }

    /**
     * Turns on the optional protocol features the client asked for (and the bridge knows).
     *
     * @return the features now on
     */
    default List<String> setCapabilities(List<String> requested) {
        return Collections.emptyList();
    }

    /**
     * The client could not apply a game state patch: sends it the game's last state complete.
     *
     * @return false when there is none (the next state goes out complete anyway)
     */
    default boolean resendGameState(UUID gameId) {
        return false;
    }

    /**
     * The client stopped following a game: its last state is no longer needed for patches.
     */
    default void forgetGameState(UUID gameId) {
    }
}
