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

    final class Closed extends Exception {

        public Closed(String message) {
            super(message);
        }

        public Closed(String message, Throwable cause) {
            super(message, cause);
        }
    }
}
