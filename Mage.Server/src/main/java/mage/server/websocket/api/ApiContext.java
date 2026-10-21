package mage.server.websocket.api;

import mage.interfaces.MageServer;
import mage.server.managers.ManagerFactory;
import mage.server.websocket.rpc.RpcSessions;
import mage.server.websocket.service.CardSearchService;
import mage.server.websocket.service.DeckValidationService;
import mage.server.websocket.service.deckimport.DeckImportService;

import java.util.Optional;
import java.util.function.Function;

/**
 * Web client bridge: dependencies shared by the RPC method groups.
 */
public final class ApiContext {

    final MageServer server;
    final ManagerFactory managers;
    final RpcSessions sessions;
    final CardSearchService cardSearch = new CardSearchService();
    final DeckValidationService deckValidation = new DeckValidationService();
    final DeckImportService deckImport = new DeckImportService();
    final LoginThrottle adminLoginThrottle = new LoginThrottle();
    /** player sign-ins with a password, per address and per account name */
    final LoginThrottle playerLoginThrottle = LoginThrottle.forPlayers();
    /** new accounts and access requests per client address */
    final RegistrationLimiter registrations = new RegistrationLimiter();
    /** the bot check on registration and access requests; off unless configured */
    Turnstile turnstile = Turnstile.fromSettings();
    /**
     * Server admin password; admin login over the bridge is disabled when it is empty.
     */
    final String adminPassword;
    /**
     * Who is staff: the session's actor name for the audit log, or empty. Null: the server's own sessions decide.
     */
    Function<String, Optional<String>> staff;

    public ApiContext(MageServer server, ManagerFactory managers, RpcSessions sessions) {
        this(server, managers, sessions, "");
    }

    public ApiContext(MageServer server, ManagerFactory managers, RpcSessions sessions, String adminPassword) {
        this.server = server;
        this.managers = managers;
        this.sessions = sessions;
        this.adminPassword = adminPassword == null ? "" : adminPassword;
    }

    /** For tests without a server: decides who is staff instead of the server's sessions. */
    public ApiContext withStaff(Function<String, Optional<String>> staff) {
        this.staff = staff;
        return this;
    }
}
