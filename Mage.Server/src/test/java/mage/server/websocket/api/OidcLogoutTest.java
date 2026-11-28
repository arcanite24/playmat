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
import mage.server.AuthorizedUserRepository;
import mage.server.DisconnectReason;
import mage.server.Session;
import mage.server.User;
import mage.server.admin.AccessPolicy;
import mage.server.admin.AdminStore;
import mage.server.admin.OidcAuth;
import mage.server.admin.RuntimeSettings;
import mage.server.managers.ManagerFactory;
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
import java.util.Date;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.UUID;
import java.util.function.Function;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

/**
 * The end of an identity-provider sign-in, part two (RP-initiated logout, RFC 7063): the sign-in's verified ID
 * token is bound onto the session through a one-use ref, and kept on the account's link as the account's last
 * verified one. A sign-in through the provider ends the provider's session on sign-out with its own token; so
 * does any other session of a linked account (a password, a remembered device, a restored session) - with the
 * account's last verified token, so a sign-out never leaves a warm session that the next button click would
 * silently re-authenticate. An account the provider never signed in gets the plain sign-out; a spent or
 * foreign ref marks nothing.
 */
public class OidcLogoutTest {

    private static final String PASSWORD = "correct horse 7";
    private static final String LABEL = "Pocket ID";
    private static final String ADMIN_PASSWORD = "admin-secret";
    private static final String DISCOVERY_URL = "https://id.example.test/.well-known/openid-configuration";
    private static final String ISSUER = "https://id.example.test/";
    private static final String CLIENT_ID = "playmat";
    private static final String PUBLIC_URL = "https://play.example.test/";
    /** shaped like the scratch provider's: browser-facing authorize, plus an end_session endpoint */
    private static final String DISCOVERY_JSON =
            "{\"issuer\": \"https://id.example.test/\","
                    + "\"authorization_endpoint\": \"/authorize\","
                    + "\"token_endpoint\": \"/api/oidc/token\","
                    + "\"jwks_uri\": \"/.well-known/jwks.json\","
                    + "\"end_session_endpoint\": \"/api/oidc/end-session\"}";
    /** a provider without RP-initiated logout omits the endpoint: the sign-out degrades to the plain one */
    private static final String DISCOVERY_WITHOUT_LOGOUT_JSON =
            "{\"issuer\": \"https://id.example.test/\","
                    + "\"authorization_endpoint\": \"/authorize\","
                    + "\"token_endpoint\": \"/api/oidc/token\","
                    + "\"jwks_uri\": \"/.well-known/jwks.json\"}";

    @TempDir
    Path dir;

    private final Map<String, Function<Object[], Object>> overrides = new HashMap<>();
    private final Map<String, User> users = new HashMap<>();

    private AuthorizedUserRepository previousAccounts;
    private AuthorizedUserRepository accounts;
    private AdminStore store;
    private ManagerFactory managers;
    private FakeSessions sessions;
    private RpcDispatcher dispatcher;
    private OidcAuth oidc;

    private long now = 1_000_000;
    private String discoveryJson = DISCOVERY_JSON;
    private String jwksJson;
    private String tokenJson = "{\"id_token\": \"never served\"}";
    private String sub = "sub-octo";
    private KeyPair rsa;

    @BeforeEach
    void setUp() throws Exception {
        previousAccounts = AuthorizedUserRepository.getInstance();
        accounts = new AuthorizedUserRepository("jdbc:h2:mem:oidclogout-" + UUID.randomUUID() + ";DB_CLOSE_DELAY=-1");
        AuthorizedUserRepository.use(accounts);
        store = new AdminStore("jdbc:sqlite:" + dir.resolve("admin.db"));
        AccessPolicy.use(AccessPolicy.create(store, "owner", "open", "", now));
        RuntimeSettings.use(RuntimeSettings.create(store));

        KeyPairGenerator generator = KeyPairGenerator.getInstance("RSA");
        generator.initialize(2048);
        rsa = generator.generateKeyPair();
        jwksJson = new JWKSet(new RSAKey.Builder((RSAPublicKey) rsa.getPublic()).keyUse(KeyUse.SIGNATURE).algorithm(JWSAlgorithm.RS256).keyID("k1").build()).toString();

        managers = deepProxy(ManagerFactory.class);
        overrides.put("isAuthenticationActivated", args -> true);
        overrides.put("getMinUserNameLength", args -> 3);
        overrides.put("getMaxUserNameLength", args -> 20);
        overrides.put("getMinPasswordLength", args -> 8);
        overrides.put("getMaxPasswordLength", args -> 100);
        overrides.put("getInvalidUserNamePattern", args -> "[^a-zA-Z0-9_-]");
        overrides.put("createUser", args -> {
            String name = (String) args[0];
            if (users.containsKey(name)) {
                return Optional.empty();
            }
            User user = new User(managers, name, (String) args[1], (mage.server.AuthorizedUser) args[2]);
            users.put(name, user);
            return Optional.of(user);
        });
        overrides.put("getUserByName", args -> Optional.ofNullable(users.get((String) args[0])));
        overrides.put("connectToSession", args -> true);

        sessions = new FakeSessions();
        dispatcher = WebClientApi.createDispatcher(new ApiContext(proxyServer(), managers, sessions, ADMIN_PASSWORD)
                .withStaff(sessionId -> Optional.of("staff")));
    }

    @AfterEach
    void tearDown() {
        AuthorizedUserRepository.use(previousAccounts);
        AccessPolicy.use(null);
        RuntimeSettings.use(null);
        OidcAuth.use(null);
    }

    /** The server's configured provider, with or without an end_session endpoint. */
    private void useProvider(boolean endSession) {
        discoveryJson = endSession ? DISCOVERY_JSON : DISCOVERY_WITHOUT_LOGOUT_JSON;
        oidc = new OidcAuth(DISCOVERY_URL, CLIENT_ID, "s3cret", LABEL, PUBLIC_URL, false, "", "", "", (method, url, token, form) -> {
            if (url.endsWith("/.well-known/openid-configuration")) {
                return discoveryJson;
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

    private Session newSession() {
        Session session = new Session(managers, UUID.randomUUID().toString(), null);
        session.setHost("192.0.2.10");
        return session;
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

    /** The provider conversation the web client has already had: exchange the code, verify the token, finish. */
    private String providerSignInRef() throws Exception {
        String url = oidc.start();
        tokenJson = tokenResponse(query(url, "nonce"));
        String state = query(url, "state");
        oidc.identify(state, "the-code");
        return oidc.done(state);
    }

    private String query(String url, String key) {
        java.util.regex.Matcher matcher = java.util.regex.Pattern.compile(key + "=([^&]+)").matcher(url);
        assertThat(matcher.find()).isTrue();
        return matcher.group(1);
    }

    /** An account the provider user is linked to, so a ref for this token may bind. */
    private void linkAccount(String account) throws Exception {
        accounts.add(account, PASSWORD, account + "@example.test");
        store.linkOidc(sub, account, account + "@example.test", account, now);
    }

    /** The web client's connectUser after a provider sign-in: the ref from oidcFinish rides along. */
    private boolean connect(FakeConnection connection, String password, String idTokenRef) throws Exception {
        JsonArray params = new JsonArray();
        params.add("user");
        params.add(password);
        params.add("x");
        params.add("");
        params.add("");
        params.add("");
        params.add("");
        params.add(idTokenRef);
        return (Boolean) dispatcher.dispatch("connectUser", params, connection);
    }

    /** Like the client's generated method: the current session rides along as the sessionId parameter. */
    private Object logout(FakeConnection connection) throws Exception {
        JsonArray params = new JsonArray();
        params.add(connection.getSessionId());
        return dispatcher.dispatch("oidcLogout", params, connection);
    }

    // --- the four cases --------------------------------------------------------------------------

    @Test
    void aProviderSignInEndsTheProviderSessionOnSignOut() throws Exception {
        useProvider(true);
        linkAccount("user");
        FakeConnection connection = new FakeConnection();
        assertThat(connect(connection, "", providerSignInRef())).isTrue();

        SessionApi.OidcLogout out = (SessionApi.OidcLogout) logout(connection);

        assertThat(out).isNotNull();
        assertThat(out.url)
                .startsWith("https://id.example.test/api/oidc/end-session?")
                .contains("id_token_hint=" + sessions.oidcIdToken(connection.getSessionId()).get().raw)
                .contains("post_logout_redirect_uri=" + java.net.URLEncoder.encode(PUBLIC_URL, "UTF-8"))
                .contains("state=" + out.state);
        assertThat(out.state).isNotBlank();
        assertThat(out.expired).isFalse();
    }

    @Test
    void aProviderSignInOnAProviderWithoutEndSessionGetsNoUrl() throws Exception {
        useProvider(false);
        linkAccount("user");
        FakeConnection connection = new FakeConnection();
        assertThat(connect(connection, "", providerSignInRef())).isTrue();

        assertThat(logout(connection)).as("no end_session_endpoint: the plain sign-out stands").isNull();
    }

    @Test
    void aPasswordSignInGetsNoUrl() throws Exception {
        useProvider(true);
        accounts.add("user", PASSWORD, "user@example.test");
        FakeConnection connection = new FakeConnection();
        assertThat(connect(connection, PASSWORD, "")).isTrue();

        assertThat(logout(connection)).isNull();
    }

    @Test
    void aRememberedDeviceSignInGetsNoUrl() throws Exception {
        useProvider(true);
        accounts.add("user", PASSWORD, "user@example.test");
        FakeConnection connection = new FakeConnection();
        JsonArray params = new JsonArray();
        params.add("user");
        params.add("");
        params.add("x");
        params.add("");
        params.add("");
        params.add("");
        params.add("a-remember-token");
        assertThat((Boolean) dispatcher.dispatch("connectUser", params, connection)).isTrue();

        assertThat(logout(connection)).isNull();
    }

    @Test
    void anExpiredTokenStillGetsAUrl() throws Exception {
        useProvider(true);
        linkAccount("user");
        FakeConnection connection = new FakeConnection();
        assertThat(connect(connection, "", providerSignInRef())).isTrue();

        now += 301_000; // past the ID token's five-minute lifetime

        SessionApi.OidcLogout out = (SessionApi.OidcLogout) logout(connection);

        assertThat(out).as("the hint is sent anyway; the provider decides whether to accept it").isNotNull();
        assertThat(out.expired).isTrue();
    }

    // --- a session that holds no token of its own (a reload, a device) --------------------------------

    @Test
    void aRememberedDeviceSignInOfALinkedAccountEndsTheProviderSession() throws Exception {
        useProvider(true);
        linkAccount("user");
        FakeConnection first = new FakeConnection();
        assertThat(connect(first, "", providerSignInRef())).isTrue(); // binds the token, and keeps it on the link

        FakeConnection reloaded = new FakeConnection(); // a reload: the remembered device signs back in, no ref
        JsonArray params = new JsonArray();
        params.add("user");
        params.add("");
        params.add("x");
        params.add("");
        params.add("");
        params.add("");
        params.add("a-remember-token");
        assertThat((Boolean) dispatcher.dispatch("connectUser", params, reloaded)).isTrue();

        SessionApi.OidcLogout out = (SessionApi.OidcLogout) logout(reloaded);

        assertThat(out).as("the account's last verified token is the hint").isNotNull();
        assertThat(out.url).contains("id_token_hint=" + sessions.oidcIdToken(first.getSessionId()).get().raw);
    }

    @Test
    void aPasswordSignInOfALinkedAccountEndsTheProviderSession() throws Exception {
        useProvider(true);
        linkAccount("user");
        FakeConnection providerOne = new FakeConnection();
        assertThat(connect(providerOne, "", providerSignInRef())).isTrue(); // the account's last token

        FakeConnection passwordOne = new FakeConnection();
        assertThat(connect(passwordOne, PASSWORD, "")).isTrue(); // a password sign-in, no token of its own

        SessionApi.OidcLogout out = (SessionApi.OidcLogout) logout(passwordOne);

        assertThat(out).as("the sign-out ends the provider's session, so the next button click asks again").isNotNull();
        assertThat(out.url).contains("id_token_hint=" + sessions.oidcIdToken(providerOne.getSessionId()).get().raw);
    }

    @Test
    void anExpiredStoredTokenStillGetsAUrl() throws Exception {
        useProvider(true);
        linkAccount("user");
        FakeConnection first = new FakeConnection();
        assertThat(connect(first, "", providerSignInRef())).isTrue();

        now += 301_000; // past the ID token's five-minute lifetime

        FakeConnection reloaded = new FakeConnection();
        JsonArray params = new JsonArray();
        params.add("user");
        params.add("");
        params.add("x");
        params.add("");
        params.add("");
        params.add("");
        params.add("a-remember-token");
        assertThat((Boolean) dispatcher.dispatch("connectUser", params, reloaded)).isTrue();

        SessionApi.OidcLogout out = (SessionApi.OidcLogout) logout(reloaded);

        assertThat(out).as("the stored hint is sent anyway; the provider decides whether to accept it").isNotNull();
        assertThat(out.expired).isTrue();
    }

    @Test
    void aSignInOfAnAccountWhoseLinkKeepsNoTokenGetsNoUrl() throws Exception {
        useProvider(true);
        accounts.add("user", PASSWORD, "user@example.test");
        store.linkOidc(sub, "User", "user@example.test", "user", now); // linked, but no token kept (a pre-token link)
        FakeConnection connection = new FakeConnection();
        assertThat(connect(connection, PASSWORD, "")).isTrue();

        assertThat(logout(connection)).as("nothing verified to hint: the plain sign-out stands").isNull();
    }

    // --- the binding rules -------------------------------------------------------------------------

    @Test
    void aConnectionWithoutASessionIsRefused() {
        useProvider(true);
        FakeConnection connection = new FakeConnection(); // never signed in

        assertThatThrownBy(() -> logout(connection))
                .isInstanceOfSatisfying(RpcException.class, e -> assertThat(e.getCode()).isEqualTo(RpcException.NOT_AUTHORIZED));
    }

    @Test
    void aRefForAnotherProvidersUserMarksNothing() throws Exception {
        useProvider(true);
        accounts.add("user", PASSWORD, "user@example.test");
        store.linkOidc("sub-someone-else", "Other", "other@example.test", "user", now); // the account is linked to someone else
        FakeConnection connection = new FakeConnection();
        assertThat(connect(connection, "", providerSignInRef())).isTrue(); // the sign-in itself stands

        assertThat(sessions.oidcTokens).as("the token proves a different provider user").isEmpty();
        assertThat(logout(connection)).isNull();
    }

    @Test
    void aSpentRefMarksNothing() throws Exception {
        useProvider(true);
        linkAccount("user");
        String ref = providerSignInRef();

        FakeConnection first = new FakeConnection();
        FakeConnection second = new FakeConnection(); // a second tab carrying the same ref
        assertThat(connect(first, "", ref)).isTrue();
        assertThat(connect(second, "", ref)).isTrue();

        assertThat(sessions.oidcIdToken(first.getSessionId())).isPresent();
        assertThat(sessions.oidcIdToken(second.getSessionId())).as("the ref was spent on the first connection").isEmpty();
        // a session without its own token still ends the provider session: the account's last verified one is the hint
        assertThat(logout(second)).isNotNull();
        assertThat((SessionApi.OidcLogout) logout(first)).isNotNull();
    }

    @Test
    void aNewLoginClearsTheProviderToken() throws Exception {
        useProvider(true);
        accounts.add("user", PASSWORD, "user@example.test");
        Session session = newSession();
        OidcAuth.Token token = new OidcAuth.Token("raw", now + 300_000, sub, false);

        session.markOidcLogin(token);
        assertThat(session.oidcLoginToken()).isSameAs(token);

        session.connectUser("user", PASSWORD, ""); // any next login starts without a provider

        assertThat(session.oidcLoginToken()).isNull();
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
     * Like the server's sessions for this surface: the token the bridge bound onto each session, so a later
     * sign-out can find it again.
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
        public void disconnect(String sessionId, DisconnectReason reason) {
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
