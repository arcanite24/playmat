package mage.server.websocket.api;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import com.nimbusds.jose.JOSEException;
import com.nimbusds.jose.JWSAlgorithm;
import com.nimbusds.jose.JWSHeader;
import com.nimbusds.jose.crypto.RSASSASigner;
import com.nimbusds.jose.jwk.JWKSet;
import com.nimbusds.jose.jwk.KeyUse;
import com.nimbusds.jose.jwk.RSAKey;
import com.nimbusds.jwt.JWTClaimsSet;
import com.nimbusds.jwt.SignedJWT;
import mage.interfaces.MageServer;
import mage.server.AuthorizedUser;
import mage.server.AuthorizedUserRepository;
import mage.server.Session;
import mage.server.User;
import mage.server.admin.AccessPolicy;
import mage.server.admin.AdminStore;
import mage.server.admin.OidcAuth;
import mage.server.admin.RuntimeSettings;
import mage.server.admin.StaffRole;
import mage.server.managers.ConfigSettings;
import mage.server.managers.ManagerFactory;
import mage.server.managers.SessionManager;
import mage.server.websocket.rpc.RpcConnection;
import mage.server.websocket.rpc.RpcDispatcher;
import mage.server.websocket.rpc.RpcException;
import mage.server.websocket.rpc.RpcSessions;
import mage.server.ClientChannel;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.io.IOException;
import java.lang.reflect.Proxy;
import java.nio.file.Path;
import java.security.KeyPair;
import java.security.KeyPairGenerator;
import java.security.interfaces.RSAPublicKey;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Date;
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
 * The identity provider's admin group (xmage.oidc.adminGroup): a sign-in whose verified ID token carries it
 * holds the server's admin console for the session's life, on its own session, with no password. The marker
 * rides the same one-use ref binding as the ID token, so a spent or foreign ref, and every sign-in that did
 * not come through the provider, hold no console.
 */
public class OidcAdminTest {

    private static final String PASSWORD = "correct horse 7";
    private static final String LABEL = "Pocket ID";
    private static final String ADMIN_PASSWORD = "admin-secret";
    private static final String ADMIN_GROUP = "admins";
    private static final String DISCOVERY_URL = "https://id.example.test/.well-known/openid-configuration";
    private static final String ISSUER = "https://id.example.test/";
    private static final String CLIENT_ID = "playmat";
    private static final String PUBLIC_URL = "https://play.example.test/";

    private static final String DISCOVERY_JSON =
            "{\"issuer\": \"https://id.example.test/\","
                    + "\"authorization_endpoint\": \"/authorize\","
                    + "\"token_endpoint\": \"/api/oidc/token\","
                    + "\"jwks_uri\": \"/.well-known/jwks.json\","
                    + "\"end_session_endpoint\": \"/api/oidc/end-session\"}";

    @TempDir
    Path dir;

    private final Map<String, Function<Object[], Object>> managerOverrides = new HashMap<>();
    private final Map<String, Function<Object[], Object>> sessionOverrides = new HashMap<>();
    private final Map<String, Function<Object[], Object>> configOverrides = new HashMap<>();
    private final Map<String, User> users = new HashMap<>();
    /** the engine sessions of logged-in bridge connections, so Staff.member can read them like the server does */
    private final Map<String, Session> engineSessions = new HashMap<>();
    private final Map<String, User> engineUserOf = new HashMap<>();

    private AuthorizedUserRepository previousAccounts;
    private AuthorizedUserRepository accounts;
    private AdminStore store;
    private ManagerFactory managers;
    private FakeSessions sessions;
    private RpcDispatcher dispatcher;
    private OidcAuth oidc;

    private long now = 1_000_000;
    private String jwksJson;
    private String tokenJson = "{\"id_token\": \"never served\"}";
    private String sub = "sub-boss";
    private List<String> groups = new ArrayList<>();
    private KeyPair rsa;

    @BeforeEach
    void setUp() throws Exception {
        previousAccounts = AuthorizedUserRepository.getInstance();
        accounts = new AuthorizedUserRepository("jdbc:h2:mem:oidcadmin-" + UUID.randomUUID() + ";DB_CLOSE_DELAY=-1");
        AuthorizedUserRepository.use(accounts);
        store = new AdminStore("jdbc:sqlite:" + dir.resolve("admin.db"));
        AccessPolicy.use(AccessPolicy.create(store, "owner", "open", "", now));
        RuntimeSettings.use(RuntimeSettings.create(store));

        KeyPairGenerator generator = KeyPairGenerator.getInstance("RSA");
        generator.initialize(2048);
        rsa = generator.generateKeyPair();
        jwksJson = new JWKSet(new RSAKey.Builder((RSAPublicKey) rsa.getPublic()).keyUse(KeyUse.SIGNATURE).algorithm(JWSAlgorithm.RS256).keyID("k1").build()).toString();

        managers = deepProxy(ManagerFactory.class, managerOverrides);
        managerOverrides.put("isAuthenticationActivated", args -> true);
        managerOverrides.put("configSettings", args -> deepProxy(ConfigSettings.class, configOverrides));
        configOverrides.put("isAuthenticationActivated", args -> true);
        managerOverrides.put("sessionManager", args -> deepProxy(SessionManager.class, sessionOverrides));
        sessionOverrides.put("getSession", args -> Optional.ofNullable(engineSessions.get((String) args[0])));
        sessionOverrides.put("getUser", args -> Optional.ofNullable(engineUserOf.get((String) args[0])));
        managerOverrides.put("createUser", args -> {
            String name = (String) args[0];
            if (users.containsKey(name)) {
                return Optional.empty();
            }
            User user = new User(managers, name, (String) args[1], (AuthorizedUser) args[2]);
            users.put(name, user);
            return Optional.of(user);
        });
        managerOverrides.put("getUserByName", args -> Optional.ofNullable(users.get((String) args[0])));
        managerOverrides.put("connectToSession", args -> true);

        sessions = new FakeSessions();
        // no withStaff seam: Staff.member decides from the session's own state, like on the real server
        dispatcher = WebClientApi.createDispatcher(new ApiContext(proxyServer(), managers, sessions, ADMIN_PASSWORD));
    }

    @AfterEach
    void tearDown() {
        AuthorizedUserRepository.use(previousAccounts);
        AccessPolicy.use(null);
        RuntimeSettings.use(null);
        OidcAuth.use(null);
    }

    /** The server's configured provider, with or without an admin group. */
    private void useProvider(String adminGroup) {
        oidc = new OidcAuth(DISCOVERY_URL, CLIENT_ID, "s3cret", LABEL, PUBLIC_URL, false, adminGroup, "", "", (method, url, token, form) -> {
            if (url.endsWith("/.well-known/openid-configuration")) {
                return DISCOVERY_JSON;
            }
            if (url.endsWith("/.well-known/jwks.json")) {
                return jwksJson;
            }
            if (url.endsWith("/api/oidc/token")) {
                return tokenJson;
            }
            throw new IOException("unexpected " + url);
        }, () -> now);
        OidcAuth.use(oidc);
        oidc.fetchDiscovery();
    }

    /** The provider user's groups claim, as its ID token carries it. */
    private void tokenCarries(String... groupNames) {
        groups = new ArrayList<>(Arrays.asList(groupNames));
    }

    private String tokenResponse(String nonce) {
        try {
            JWTClaimsSet.Builder builder = new JWTClaimsSet.Builder()
                    .issuer(ISSUER)
                    .audience(CLIENT_ID)
                    .subject(sub)
                    .claim("nonce", nonce)
                    .issueTime(new Date(now))
                    .expirationTime(new Date(now + 300_000));
            if (!groups.isEmpty()) {
                builder.claim("groups", groups);
            }
            SignedJWT jwt = new SignedJWT(new JWSHeader.Builder(JWSAlgorithm.RS256).keyID("k1").build(), builder.build());
            jwt.sign(new RSASSASigner(rsa.getPrivate()));
            JsonObject response = new JsonObject();
            response.addProperty("access_token", "at-1");
            response.addProperty("id_token", jwt.serialize());
            return response.toString();
        } catch (JOSEException e) {
            throw new IllegalStateException(e);
        }
    }

    private String query(String url, String key) {
        Matcher matcher = Pattern.compile(key + "=([^&]+)").matcher(url);
        assertThat(matcher.find()).isTrue();
        return matcher.group(1);
    }

    /** The provider conversation the web client has already had: exchange the code, verify the token, finish. */
    private String providerSignInRef() throws Exception {
        String url = oidc.start();
        tokenJson = tokenResponse(query(url, "nonce"));
        String state = query(url, "state");
        oidc.identify(state, "the-code");
        return oidc.done(state);
    }

    /** An account the provider user is linked to, so a ref for this token may bind. */
    private void linkAccount(String account) throws Exception {
        accounts.add(account, PASSWORD, account + "@example.test");
        store.linkOidc(sub, account + " (the provider)", account + "@example.test", account, now);
    }

    /** The web client's connectUser after a provider sign-in: the ref from oidcFinish rides along. */
    private boolean connect(FakeConnection connection, String account, String password, String idTokenRef) throws Exception {
        JsonArray params = new JsonArray();
        params.add(account);
        params.add(password);
        params.add("x");
        params.add("");
        params.add("");
        params.add("");
        params.add("");
        params.add(idTokenRef);
        boolean ok = (Boolean) dispatcher.dispatch("connectUser", params, connection);
        if (ok) {
            register(account, connection.getSessionId());
        }
        return ok;
    }

    /** The logged-in connection, as the server's session manager would see it. */
    private void register(String account, String sessionId) {
        User user = new User(managers, account, PASSWORD, accounts.getByName(account));
        users.put(account, user);
        engineUserOf.put(sessionId, user);
        engineSessions.put(sessionId, new Session(managers, sessionId, null));
        sessions.names.put(sessionId, account);
    }

    private Object staffMe(FakeConnection connection) throws Exception {
        return dispatcher.dispatch("staffMe", new JsonArray(), connection);
    }

    private Object adminAccess(FakeConnection connection) throws Exception {
        return dispatcher.dispatch("adminAccess", new JsonArray(), connection);
    }

    // --- the console the group member holds -------------------------------------------------------

    @Test
    void aProviderSignInWithTheAdminGroupGetsTheConsole() throws Exception {
        useProvider(ADMIN_GROUP);
        linkAccount("user");
        tokenCarries("players", "admins");
        FakeConnection connection = new FakeConnection();
        assertThat(connect(connection, "user", PASSWORD, providerSignInRef())).isTrue();

        StaffApi.StaffMe me = (StaffApi.StaffMe) staffMe(connection);
        assertThat(me.role).isEqualTo("owner");
        assertThat(me.actor).isEqualTo("user");

        assertThat(adminAccess(connection)).as("the admin surface answers on the player's own session").isNotNull();
    }

    @Test
    void theGroupsConsoleIsTheFullOwnerSurface() throws Exception {
        useProvider(ADMIN_GROUP);
        linkAccount("user");
        accounts.add("sidekick", PASSWORD, "sidekick@example.test");
        tokenCarries("admins");
        FakeConnection connection = new FakeConnection();
        assertThat(connect(connection, "user", PASSWORD, providerSignInRef())).isTrue();

        JsonArray params = new JsonArray();
        params.add("sidekick");
        params.add("moderator");
        params.add("the group's console");
        dispatcher.dispatch("adminStaffGrant", params, connection);

        assertThat(AccessPolicy.get().roleOf("sidekick")).isEqualTo(StaffRole.MODERATOR);
        assertThat(store.auditLog("user", "staff.grant", null, 0, 100))
                .as("the grant is audited under the member, not the console password")
                .extracting(e -> e.actor)
                .contains("user");
    }

    // --- who does not get it ----------------------------------------------------------------------

    @Test
    void aProviderSignInOutsideTheAdminGroupGetsNoConsole() throws Exception {
        useProvider(ADMIN_GROUP);
        linkAccount("user");
        tokenCarries("players");
        FakeConnection connection = new FakeConnection();
        assertThat(connect(connection, "user", PASSWORD, providerSignInRef())).isTrue();

        StaffApi.StaffMe me = (StaffApi.StaffMe) staffMe(connection);
        assertThat(me.role).isNull();
        assertThat(me.actor).isNull();

        assertThatThrownBy(() -> adminAccess(connection))
                .isInstanceOfSatisfying(RpcException.class, e -> assertThat(e.getCode()).isEqualTo(RpcException.NOT_AUTHORIZED));
    }

    @Test
    void anUnsetAdminGroupGivesNoConsoleEvenForAGroupMember() throws Exception {
        useProvider(""); // the server has no admin group configured
        linkAccount("user");
        tokenCarries("admins");
        FakeConnection connection = new FakeConnection();
        assertThat(connect(connection, "user", PASSWORD, providerSignInRef())).isTrue();

        assertThat(((StaffApi.StaffMe) staffMe(connection)).role).as("no group is configured").isNull();
    }

    @Test
    void aPasswordSignInOfAnInGroupMemberGetsNoConsole() throws Exception {
        useProvider(ADMIN_GROUP);
        linkAccount("user");
        tokenCarries("admins");
        FakeConnection connection = new FakeConnection();
        assertThat(connect(connection, "user", PASSWORD, "")).isTrue(); // a password sign-in: no ref

        assertThat(((StaffApi.StaffMe) staffMe(connection)).role)
                .as("even an in-group member's password sign-in is not a provider sign-in").isNull();
    }

    @Test
    void aRefForAnotherProvidersUserGrantsNoConsole() throws Exception {
        useProvider(ADMIN_GROUP);
        accounts.add("user", PASSWORD, "user@example.test");
        store.linkOidc("sub-boss", "user (the provider)", "user@example.test", "user", now); // the account is someone else's
        sub = "sub-someone-else";
        tokenCarries("admins");
        FakeConnection connection = new FakeConnection();
        assertThat(connect(connection, "user", PASSWORD, providerSignInRef())).isTrue(); // the sign-in itself stands

        assertThat(((StaffApi.StaffMe) staffMe(connection)).role)
                .as("the token proves a different provider user than the account is linked to").isNull();
    }

    @Test
    void aSpentRefGrantsNoConsole() throws Exception {
        useProvider(ADMIN_GROUP);
        linkAccount("user");
        tokenCarries("admins");
        String ref = providerSignInRef();

        FakeConnection first = new FakeConnection();
        FakeConnection second = new FakeConnection(); // a second tab carrying the same ref
        assertThat(connect(first, "user", PASSWORD, ref)).isTrue();
        assertThat(connect(second, "user", PASSWORD, ref)).isTrue();

        assertThat(((StaffApi.StaffMe) staffMe(first)).role).isEqualTo("owner");
        assertThat(((StaffApi.StaffMe) staffMe(second)).role).as("the ref was spent on the first connection").isNull();
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
     * Like the server's sessions for this surface: the name, the token, and the group's admin marker of each
     * session, so a later call can find them again. Accounts the provider user is linked to must be named
     * "user": the binding happens inside connectUser, before the test has recorded the name.
     */
    private static final class FakeSessions implements RpcSessions {
        final List<String> created = new ArrayList<>();
        final List<String> disconnected = new ArrayList<>();
        final Map<String, String> hosts = new HashMap<>();
        final Map<String, String> names = new HashMap<>();
        final Map<String, OidcAuth.Token> oidcTokens = new HashMap<>();
        final Map<String, Boolean> oidcAdmins = new HashMap<>();

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
        }

        @Override
        public void markOidcLogin(String sessionId, OidcAuth.Token token) {
            oidcTokens.put(sessionId, token);
        }

        @Override
        public Optional<OidcAuth.Token> oidcIdToken(String sessionId) {
            return Optional.ofNullable(oidcTokens.get(sessionId));
        }

        @Override
        public void markOidcAdmin(String sessionId, boolean admin) {
            oidcAdmins.put(sessionId, admin);
        }

        @Override
        public boolean oidcAdmin(String sessionId) {
            return oidcAdmins.getOrDefault(sessionId, false);
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
    private <T> T deepProxy(Class<T> type, Map<String, Function<Object[], Object>> overrides) {
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
                return deepProxy(result, overrides);
            }
            return null;
        });
    }
}
