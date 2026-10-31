package mage.server.websocket.api;

import com.google.gson.JsonArray;
import mage.interfaces.MageServer;
import mage.server.AccountService;
import mage.server.AuthorizedUser;
import mage.server.AuthorizedUserRepository;
import mage.server.Session;
import mage.server.User;
import mage.server.admin.AccessPolicy;
import mage.server.admin.AccessRequest;
import mage.server.admin.AdminStore;
import mage.server.admin.Invite;
import mage.server.admin.OidcAuth;
import mage.server.admin.RuntimeSettings;
import mage.server.managers.ManagerFactory;
import mage.server.notify.DiscordNotifier;
import mage.server.websocket.rpc.RpcConnection;
import mage.server.websocket.rpc.RpcDispatcher;
import mage.server.websocket.rpc.RpcException;
import mage.server.websocket.rpc.RpcSessions;
import mage.server.ClientChannel;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.lang.reflect.Proxy;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.UUID;
import java.util.function.Function;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

/**
 * The OIDC-only mode (xmage.oidc.only): the identity provider is the only first sign-in method. Typed passwords,
 * registration (invites included) and password resets are refused with one shared message, and so is GitHub;
 * remembered devices, restored sessions, access requests and the admin console keep working. With the mode off,
 * every one of these behaves exactly as it does today.
 */
public class OidcOnlyTest {

    private static final String PASSWORD = "correct horse 7";
    private static final String NEW_PASSWORD = "new horse 8";
    private static final String LABEL = "Pocket ID";
    private static final String ADMIN_PASSWORD = "admin-secret";
    /** the one player-readable refusal every password path gets in the mode */
    private static final String REFUSAL = "This server signs in with " + LABEL + " only.";
    private static final String GITHUB_OFF = "Sign in with GitHub is off: this server signs in with " + LABEL + " only.";

    @TempDir
    Path dir;

    private final Map<String, Function<Object[], Object>> overrides = new HashMap<>();
    private final Map<String, User> users = new HashMap<>();
    private final List<String[]> mails = new ArrayList<>();

    private AuthorizedUserRepository previousAccounts;
    private AuthorizedUserRepository accounts;
    private AdminStore store;
    private ManagerFactory managers;
    private FakeSessions sessions;
    private RpcDispatcher dispatcher;
    private OidcAuth oidc;

    @BeforeEach
    void setUp() throws Exception {
        previousAccounts = AuthorizedUserRepository.getInstance();
        accounts = new AuthorizedUserRepository("jdbc:h2:mem:oidconly-" + UUID.randomUUID() + ";DB_CLOSE_DELAY=-1");
        AuthorizedUserRepository.use(accounts);
        store = new AdminStore("jdbc:sqlite:" + dir.resolve("admin.db"));
        AccessPolicy.use(AccessPolicy.create(store, "owner", "open", "", 1000));
        RuntimeSettings.use(RuntimeSettings.create(store));

        managers = deepProxy(ManagerFactory.class);
        overrides.put("isAuthenticationActivated", args -> true);
        overrides.put("getMinUserNameLength", args -> 3);
        overrides.put("getMaxUserNameLength", args -> 20);
        overrides.put("getMinPasswordLength", args -> 8);
        overrides.put("getMaxPasswordLength", args -> 100);
        overrides.put("getInvalidUserNamePattern", args -> "[^a-zA-Z0-9_-]");
        overrides.put("getMailUser", args -> "mailer@example.test");
        overrides.put("getMailgunApiKey", args -> "");
        overrides.put("getServerName", args -> "providers-only");
        overrides.put("sendMessage", args -> mails.add(new String[]{(String) args[0], (String) args[1], (String) args[2]}));
        overrides.put("createUser", args -> {
            String name = (String) args[0];
            if (users.containsKey(name)) {
                return Optional.empty();
            }
            User user = new User(managers, name, (String) args[1], (AuthorizedUser) args[2]);
            users.put(name, user);
            return Optional.of(user);
        });
        overrides.put("getUserByName", args -> Optional.ofNullable(users.get((String) args[0])));
        overrides.put("connectToSession", args -> {
            String sessionId = (String) args[0];
            UUID id = (UUID) args[1];
            for (User user : users.values()) {
                if (user.getId().equals(id)) {
                    user.setSessionId(sessionId);
                }
            }
            return true;
        });

        sessions = new FakeSessions();
        dispatcher = WebClientApi.createDispatcher(new ApiContext(proxyServer(), managers, sessions, ADMIN_PASSWORD)
                .withStaff(sessionId -> Optional.of("staff")));

        // the web client's address, so an approval email can point at it
        System.setProperty(DiscordNotifier.PUBLIC_URL_PROP, "https://play.example.test/");
        useMode(false);
    }

    @AfterEach
    void tearDown() {
        AuthorizedUserRepository.use(previousAccounts);
        AccessPolicy.use(null);
        RuntimeSettings.use(null);
        OidcAuth.use(null);
        System.clearProperty(DiscordNotifier.PUBLIC_URL_PROP);
    }

    /** The server's configured provider, with the OIDC-only mode on or off. */
    private void useMode(boolean only) {
        oidc = new OidcAuth("https://id.example.test/.well-known/openid-configuration", "playmat", "s3cret",
                LABEL, "https://play.example.test/", only, (method, url, token, form) -> {
                    throw new java.io.IOException("the provider is never reached by the mode's gates");
                }, () -> 1_000_000L);
        OidcAuth.use(oidc);
    }

    private Session newSession() {
        Session session = new Session(managers, UUID.randomUUID().toString(), null);
        session.setHost("192.0.2.10");
        return session;
    }

    private void login(FakeConnection connection) throws Exception {
        dispatcher.dispatch("connectUser", params("user", "", "ignored"), connection);
    }

    private static JsonArray params(String... values) {
        JsonArray array = new JsonArray();
        for (String value : values) {
            array.add(value);
        }
        return array;
    }

    private static String code(String text) {
        Matcher matcher = Pattern.compile("(\\d{6})").matcher(text);
        assertThat(matcher.find()).as("a code in: " + text).isTrue();
        return matcher.group(1);
    }

    // --- the mode on: what stops -------------------------------------------------------------------

    @Test
    void aTypedPasswordSignInIsRefusedWithTheModeMessage() throws Exception {
        useMode(true);
        accounts.add("alice", PASSWORD, "alice@example.test");

        assertThat(newSession().connectUser("alice", PASSWORD, "")).isEqualTo(REFUSAL);
    }

    @Test
    void aRememberedDeviceStillSignsIn() throws Exception {
        useMode(true);
        accounts.add("alice", PASSWORD, "alice@example.test");
        Session session = newSession();
        session.trustRememberedLogin("alice");

        assertThat(session.connectUser("alice", PASSWORD, "")).isNull();
    }

    @Test
    void aLiveRestoreSessionStillSignsIn() throws Exception {
        useMode(true);
        accounts.add("alice", PASSWORD, "alice@example.test");
        // a password sign-in is refused in the mode, so the first login is a remembered device
        Session first = newSession();
        first.trustRememberedLogin("alice");
        assertThat(first.connectUser("alice", "", "")).isNull();
        String restoreId = users.get("alice").getRestoreSessionId();
        assertThat(restoreId).isNotEmpty();

        // the browser reload: the live restore token stands in for the password, so it signs in
        assertThat(newSession().connectUser("alice", PASSWORD, restoreId)).isNull();
    }

    @Test
    void aStaleRestoreWithAPasswordIsRefused() throws Exception {
        useMode(true);
        accounts.add("alice", PASSWORD, "alice@example.test");

        assertThat(newSession().connectUser("alice", PASSWORD, "stale-restore-id")).isEqualTo(REFUSAL);
    }

    @Test
    void registrationIsRefusedEvenWithAnInvite() throws Exception {
        useMode(true);

        assertThat(newSession().registerUser("bob", PASSWORD, "bob@example.test")).isEqualTo(REFUSAL);
        assertThat(accounts.getByName("bob")).isNull();

        Invite invite = store.createInvite("", 1, 7, "console", System.currentTimeMillis());
        assertThat(newSession().registerUser("carol", PASSWORD, "carol@example.test", invite.code)).isEqualTo(REFUSAL);
        assertThat(accounts.getByName("carol")).isNull();
        assertThat(store.invite(invite.code).uses).as("the invite is not consumed").isZero();
    }

    @Test
    void passwordResetsAreRefusedButStaffCodesAreNot() {
        useMode(true);
        accounts.add("alice", PASSWORD, "alice@example.test");
        AccountService service = new AccountService(managers);

        assertThat(service.sendResetCode("alice@example.test")).isEqualTo(REFUSAL);
        assertThat(service.resetPassword("alice@example.test", "123456", NEW_PASSWORD)).isEqualTo(REFUSAL);
        assertThat(mails).as("no reset email goes out").isEmpty();
        assertThat(service.staffResetCode("alice").code).isNotEmpty();
    }

    @Test
    void theAdminConsoleKeepsWorking() throws Exception {
        useMode(true);
        FakeConnection connection = new FakeConnection();

        assertThat(dispatcher.dispatch("connectAdmin", params(ADMIN_PASSWORD, "x"), connection)).isEqualTo(true);
    }

    @Test
    void theBridgeRefusesAPasswordSignInWithTheModeMessage() {
        useMode(true);
        accounts.add("alice", PASSWORD, "alice@example.test");
        FakeConnection connection = new FakeConnection();

        assertThatThrownBy(() -> dispatcher.dispatch("connectUser", params("alice", PASSWORD, "x"), connection))
                .isInstanceOfSatisfying(RpcException.class, e -> {
                    assertThat(e.getCode()).isEqualTo(RpcException.INVALID_PARAMS);
                    assertThat(e.getMessage()).isEqualTo(REFUSAL);
                });
    }

    @Test
    void theBridgeLetsRememberedAndRestoredLoginsThrough() throws Exception {
        useMode(true);
        accounts.add("alice", PASSWORD, "alice@example.test");

        FakeConnection remembered = new FakeConnection("192.0.2.44");
        assertThat(dispatcher.dispatch("connectUser", params("alice", "", "x", "", "", "", "a-remember-token"), remembered)).isEqualTo(true);
        assertThat(sessions.trusted).containsExactly(remembered.getSessionId() + ":alice");

        // a password only rides along when a restore token is presented: that login is a reconnect, not a guess
        FakeConnection restored = new FakeConnection("192.0.2.45");
        assertThat(dispatcher.dispatch("connectUser", params("alice", PASSWORD, "x", "a-restore-id"), restored)).isEqualTo(true);
    }

    @Test
    void githubSignInIsOff() {
        useMode(true);
        FakeConnection connection = new FakeConnection();

        assertThatThrownBy(() -> dispatcher.dispatch("githubStart", new JsonArray(), connection))
                .isInstanceOfSatisfying(RpcException.class, e -> {
                    assertThat(e.getCode()).isEqualTo(RpcException.SERVER_ERROR);
                    assertThat(e.getMessage()).isEqualTo(GITHUB_OFF);
                });
        assertThatThrownBy(() -> dispatcher.dispatch("githubFinish", params("state", "code"), connection))
                .isInstanceOfSatisfying(RpcException.class, e -> {
                    assertThat(e.getCode()).isEqualTo(RpcException.SERVER_ERROR);
                    assertThat(e.getMessage()).isEqualTo(GITHUB_OFF);
                });
    }

    @Test
    void theCapabilityFlagsReportTheMode() throws Exception {
        useMode(true);
        FakeConnection connection = new FakeConnection();

        AccountApi.ServerInfo info = (AccountApi.ServerInfo) dispatcher.dispatch("serverInfo", new JsonArray(), connection);
        assertThat(info.accounts).isTrue();
        assertThat(info.oidc).isTrue();
        assertThat(info.oidcName).isEqualTo(LABEL);
        assertThat(info.oidcOnly).isTrue();

        login(connection);
        AccessApi.AccessOverview overview = (AccessApi.AccessOverview) dispatcher.dispatch("adminAccess", new JsonArray(), connection);
        assertThat(overview.oidc).isTrue();
        assertThat(overview.oidcName).isEqualTo(LABEL);
        assertThat(overview.oidcIssuer).as("the test's provider never answers discovery").isEmpty();
        assertThat(overview.oidcOnly).isTrue();
    }

    @Test
    void anApprovedRequestIsToldToSignInWithTheProvider() throws Exception {
        useMode(true);
        AccessRequest request = store.requestAccess("bob", "bob@example.test", "", System.currentTimeMillis());
        FakeConnection connection = new FakeConnection();
        login(connection);

        JsonArray params = new JsonArray();
        params.add(String.valueOf(request.id));
        params.add(true);
        dispatcher.dispatch("adminAccessDecide", params, connection);

        assertThat(mails).hasSize(1);
        assertThat(mails.get(0)[0]).isEqualTo("bob@example.test");
        assertThat(mails.get(0)[2]).isEqualTo("You can play on providers-only now. Sign in with " + LABEL
                + " at https://play.example.test/.");
    }

    @Test
    void anAccessRequestOnAnOpenServerIsToldToSignInWithTheProvider() {
        useMode(true);
        FakeConnection connection = new FakeConnection();

        assertThatThrownBy(() -> dispatcher.dispatch("accessRequest", params("bob", "bob@example.test"), connection))
                .isInstanceOfSatisfying(RpcException.class, e -> {
                    assertThat(e.getCode()).isEqualTo(RpcException.INVALID_PARAMS);
                    assertThat(e.getMessage()).isEqualTo(REFUSAL);
                });
    }

    // --- the mode off: everything behaves as it does today ------------------------------------------

    @Test
    void withTheModeOffSignInsWorkAsToday() throws Exception {
        useMode(false);
        accounts.add("alice", PASSWORD, "alice@example.test");

        assertThat(newSession().connectUser("alice", PASSWORD, "")).isNull();
        assertThat(newSession().connectUser("alice", "wrong guess 1", "")).contains("Wrong username or password");

        FakeConnection connection = new FakeConnection();
        assertThat(dispatcher.dispatch("connectUser", params("alice", PASSWORD, "x"), connection)).isEqualTo(true);
    }

    @Test
    void withTheModeOffRegistrationWorksAsToday() {
        useMode(false);

        assertThat(newSession().registerUser("bob", PASSWORD, "bob@example.test")).isNull();
        assertThat(accounts.getByName("bob")).isNotNull();
    }

    @Test
    void withTheModeOffPasswordResetsWorkAsToday() {
        useMode(false);
        accounts.add("alice", PASSWORD, "alice@example.test");
        AccountService service = new AccountService(managers);

        assertThat(service.sendResetCode("alice@example.test")).isNull();
        assertThat(mails).hasSize(1);
        assertThat(service.resetPassword("alice@example.test", code(mails.get(0)[2]), NEW_PASSWORD)).isNull();
        assertThat(service.staffResetCode("alice").code).isNotEmpty();
    }

    @Test
    void withTheModeOffGithubSaysItIsNotSetUp() {
        useMode(false);
        FakeConnection connection = new FakeConnection();

        assertThatThrownBy(() -> dispatcher.dispatch("githubStart", new JsonArray(), connection))
                .isInstanceOfSatisfying(RpcException.class, e -> {
                    assertThat(e.getCode()).isEqualTo(RpcException.SERVER_ERROR);
                    assertThat(e.getMessage()).isEqualTo("Sign in with GitHub isn't set up on this server");
                });
    }

    @Test
    void withTheModeOffTheCapabilityFlagsReportOff() throws Exception {
        useMode(false);
        FakeConnection connection = new FakeConnection();

        AccountApi.ServerInfo info = (AccountApi.ServerInfo) dispatcher.dispatch("serverInfo", new JsonArray(), connection);
        assertThat(info.oidc).as("the provider is configured").isTrue();
        assertThat(info.oidcOnly).isFalse();

        login(connection);
        AccessApi.AccessOverview overview = (AccessApi.AccessOverview) dispatcher.dispatch("adminAccess", new JsonArray(), connection);
        assertThat(overview.oidc).isTrue();
        assertThat(overview.oidcOnly).isFalse();
    }

    @Test
    void withTheModeOffAnAccessRequestOnAnOpenServerSaysJustRegister() {
        useMode(false);
        FakeConnection connection = new FakeConnection();

        assertThatThrownBy(() -> dispatcher.dispatch("accessRequest", params("bob", "bob@example.test"), connection))
                .isInstanceOfSatisfying(RpcException.class, e -> {
                    assertThat(e.getCode()).isEqualTo(RpcException.INVALID_PARAMS);
                    assertThat(e.getMessage()).isEqualTo("This server is open to everyone: just register.");
                });
    }

    // --- fakes ---------------------------------------------------------------------------------------

    private MageServer proxyServer() {
        return (MageServer) Proxy.newProxyInstance(
                MageServer.class.getClassLoader(),
                new Class<?>[]{MageServer.class},
                (proxy, method, args) -> method.getReturnType() == boolean.class);
    }

    private static final class FakeConnection implements RpcConnection {
        private final String host;
        private String sessionId;
        private String userName;

        FakeConnection() {
            this("10.0.0.7");
        }

        FakeConnection(String host) {
            this.host = host;
        }

        @Override
        public String getSessionId() {
            return sessionId;
        }

        @Override
        public void bindSession(String sessionId) {
            this.sessionId = sessionId;
        }

        @Override
        public String getUserName() {
            return userName;
        }

        @Override
        public void setUserName(String userName) {
            this.userName = userName;
        }

        @Override
        public String getRemoteHost() {
            return host;
        }

        @Override
        public ClientChannel createChannel() {
            return null;
        }
    }

    /**
     * Like the server's sessions, for the mode's gates: the mode never checks the token's validity here, it checks
     * the login's shape, so every presented token is trusted.
     */
    private static final class FakeSessions implements RpcSessions {
        final List<String> created = new ArrayList<>();
        final List<String> disconnected = new ArrayList<>();
        final Map<String, String> hosts = new HashMap<>();
        final Map<String, String> names = new HashMap<>();
        final List<String> trusted = new ArrayList<>(); // "sessionId:userName"

        @Override
        public void create(String sessionId, ClientChannel channel, String host) {
            created.add(sessionId);
            hosts.put(sessionId, host);
        }

        @Override
        public boolean exists(String sessionId) {
            return created.contains(sessionId) && disconnected.stream().noneMatch(d -> d.startsWith(sessionId + ":"));
        }

        @Override
        public void disconnect(String sessionId, mage.server.DisconnectReason reason) {
            disconnected.add(sessionId + ":" + reason);
        }

        @Override
        public boolean isSignedInElsewhere(String userName, String sessionId) {
            return false;
        }

        @Override
        public Optional<String> userName(String sessionId) {
            return Optional.of(names.getOrDefault(sessionId, "user"));
        }

        @Override
        public Optional<String> restoreToken(String sessionId) {
            return exists(sessionId) ? Optional.of(sessionId) : Optional.empty();
        }

        @Override
        public Optional<String> rememberLogin(String sessionId) {
            return Optional.of("token-" + created.size());
        }

        @Override
        public void trustRememberedLogin(String sessionId, String userName, String token) {
            trusted.add(sessionId + ":" + userName);
        }

        @Override
        public void forgetLogin(String userName, String token) {
        }

        @Override
        public boolean isChatMember(String sessionId, UUID chatId) {
            return true;
        }
    }

    @SuppressWarnings("unchecked")
    private <T> T deepProxy(Class<T> type) {
        return (T) Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[]{type}, (proxy, method, args) -> {
            Function<Object[], Object> override = overrides.get(method.getName());
            if (override != null) {
                return override.apply(args == null ? new Object[0] : args);
            }
            Class<?> result = method.getReturnType();
            if (result == Optional.class) {
                return Optional.empty();
            }
            if (result == boolean.class) {
                return false;
            }
            if (result == int.class || result == long.class) {
                return result == int.class ? (Object) 0 : (Object) 0L;
            }
            if (result.isInterface()) {
                return deepProxy(result);
            }
            return null;
        });
    }
}
