package mage.server.websocket;

import mage.server.AuthorizedUserRepository;
import mage.server.DisconnectReason;
import mage.server.User;
import mage.server.managers.ManagerFactory;
import mage.server.websocket.rpc.RpcSessions;
import org.jboss.remoting.callback.InvokerCallbackHandler;

import java.util.Optional;
import java.util.UUID;

/**
 * Web client bridge: sessions are regular server sessions, created with a WebSocket callback handler.
 */
final class ServerRpcSessions implements RpcSessions {

    private final ManagerFactory managers;

    ServerRpcSessions(ManagerFactory managers) {
        this.managers = managers;
    }

    @Override
    public void create(String sessionId, InvokerCallbackHandler callbackHandler, String host) {
        managers.sessionManager().createSession(sessionId, callbackHandler);
        // anonymous users are identified by host on reconnect, like desktop clients
        managers.sessionManager().getSession(sessionId).ifPresent(session -> session.setHost(host));
    }

    @Override
    public boolean exists(String sessionId) {
        return managers.sessionManager().getSession(sessionId).isPresent();
    }

    @Override
    public void disconnect(String sessionId, DisconnectReason reason) {
        managers.sessionManager().disconnect(sessionId, reason, true);
    }

    @Override
    public boolean isSignedInElsewhere(String userName, String sessionId) {
        if (userName == null || userName.isEmpty()) {
            return false;
        }
        return managers.userManager().getUserByName(userName)
                .map(User::getSessionId)
                .filter(current -> !current.isEmpty() && !current.equals(sessionId) && exists(current))
                .isPresent();
    }

    @Override
    public Optional<String> userName(String sessionId) {
        return managers.sessionManager().getSession(sessionId)
                .flatMap(session -> managers.userManager().getUser(session.getUserId()))
                .map(User::getName);
    }

    @Override
    public Optional<String> restoreToken(String sessionId) {
        return managers.sessionManager().getSession(sessionId)
                .flatMap(session -> managers.userManager().getUser(session.getUserId()))
                .map(User::getRestoreSessionId)
                .filter(token -> !token.isEmpty());
    }

    @Override
    public Optional<String> rememberLogin(String sessionId) {
        if (!managers.configSettings().isAuthenticationActivated()) {
            return Optional.empty();
        }
        return userName(sessionId).map(name -> AuthorizedUserRepository.getInstance().rememberLogin(name));
    }

    @Override
    public void trustRememberedLogin(String sessionId, String userName, String token) {
        if (managers.configSettings().isAuthenticationActivated()
                && AuthorizedUserRepository.getInstance().isRememberedLogin(userName, token)) {
            managers.sessionManager().getSession(sessionId).ifPresent(session -> session.trustRememberedLogin(userName));
        }
    }

    @Override
    public void forgetLogin(String userName, String token) {
        if (managers.configSettings().isAuthenticationActivated()) {
            AuthorizedUserRepository.getInstance().forgetLogin(userName, token);
        }
    }

    @Override
    public boolean isChatMember(String sessionId, UUID chatId) {
        UUID userId = managers.sessionManager().getSession(sessionId).map(session -> session.getUserId()).orElse(null);
        if (userId == null || chatId == null) {
            return false;
        }
        return managers.chatManager().getChatSessions().stream()
                .anyMatch(chat -> chatId.equals(chat.getChatId()) && chat.hasUser(userId, false));
    }
}
