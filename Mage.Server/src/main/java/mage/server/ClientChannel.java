package mage.server;

import mage.interfaces.callback.ClientCallback;

/**
 * Where a session's messages to its client go: the web client's WebSocket (the desktop protocol, JBoss Remoting, is
 * gone; docs/RustMigration.md phase 1).
 */
public interface ClientChannel {

    /**
     * Sends one message. The caller (the session) keeps messages in order and sends one at a time.
     *
     * @throws Closed when the client can't be reached any more: the session then disconnects
     */
    void send(ClientCallback callback) throws Closed;

    /** whether games can be handed to the Rust host on this connection: it comes through the gateway, which hosts them */
    default boolean hostsGames() {
        return false;
    }

    /** the id of the gateway that hosts this connection's games, or null: a game between people goes to the Rust host
     * only when both players' connections come through the same one */
    default String gameHostId() {
        return null;
    }

    final class Closed extends Exception {

        public Closed(String message) {
            super(message);
        }

        public Closed(String message, Throwable cause) {
            super(message, cause);
        }
    }
}
