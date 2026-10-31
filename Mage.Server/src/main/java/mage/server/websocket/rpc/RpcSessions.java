package mage.server.websocket.rpc;

import mage.server.DisconnectReason;
import org.jboss.remoting.callback.InvokerCallbackHandler;

import java.util.Optional;
import java.util.UUID;

/**
 * Web client bridge: the session operations the dispatcher needs from the server.
 */
public interface RpcSessions {

    void create(String sessionId, InvokerCallbackHandler callbackHandler, String host);

    boolean exists(String sessionId);

    void disconnect(String sessionId, DisconnectReason reason);

    /**
     * @return whether the user is connected now through a live session other than this one: another login as the same
     * user (another tab, device or client) took it over and ended this session
     */
    boolean isSignedInElsewhere(String userName, String sessionId);

    /**
     * @return name of the user logged in with this session
     */
    Optional<String> userName(String sessionId);

    /**
     * @return the token that lets a later connection take this session's user back (connectUser's restoreSessionId),
     * empty when the session has no logged in user
     */
    Optional<String> restoreToken(String sessionId);

    /**
     * "Keep me signed in": remembers this device for the session's account.
     *
     * @return the token connectUser accepts instead of the password, empty without a logged in account (anonymous servers)
     */
    Optional<String> rememberLogin(String sessionId);

    /**
     * Lets the session's next login as this user skip the password when the token is one of the user's remembered devices.
     */
    void trustRememberedLogin(String sessionId, String userName, String token);

    /**
     * Single sign-on: lets the session's next login skip the password for the account the sign-on proxy named, linking
     * the account of that name or creating one first.
     *
     * @return the account's name as stored
     * @throws IllegalArgumentException with the reason for the player when there is no account and none can be made
     */
    default String trustSignedOnLogin(String sessionId, String signedOnUser) {
        throw new IllegalArgumentException("This server has no single sign-on");
    }

    /**
     * Forgets a remembered device (signing out on it).
     */
    void forgetLogin(String userName, String token);

    /**
     * @return whether the session's user has joined the chat
     */
    boolean isChatMember(String sessionId, UUID chatId);
}
