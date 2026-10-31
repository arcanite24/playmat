package mage.server;

import mage.server.admin.AccessPolicy;
import mage.server.admin.AdminStore;
import mage.server.managers.ManagerFactory;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.lang.reflect.Proxy;
import java.nio.file.Path;
import java.util.HashMap;
import java.util.Map;
import java.util.Optional;
import java.util.UUID;
import java.util.function.Function;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

/**
 * Single sign-on through a trusted header: the name a sign-on proxy sends links or creates an account, a signed-on
 * login needs no password, and with passwords off nothing else signs in.
 */
public class TrustedHeaderAuthTest {

    private static final String PASSWORD = "correct horse 7";

    @TempDir
    Path dir;

    private final Map<String, Function<Object[], Object>> overrides = new HashMap<>();
    private final Map<String, User> users = new HashMap<>();
    private AuthorizedUserRepository previousAccounts;
    private AuthorizedUserRepository accounts;
    private AdminStore store;
    private ManagerFactory managers;

    @BeforeEach
    void setUp() throws Exception {
        previousAccounts = AuthorizedUserRepository.getInstance();
        accounts = new AuthorizedUserRepository("jdbc:h2:mem:sso-" + UUID.randomUUID() + ";DB_CLOSE_DELAY=-1");
        AuthorizedUserRepository.use(accounts);
        store = new AdminStore("jdbc:sqlite:" + dir.resolve("admin.db"));
        AccessPolicy.use(AccessPolicy.create(store, "owner", "open", "", 1000));

        managers = deepProxy(ManagerFactory.class);
        overrides.put("isAuthenticationActivated", args -> true);
        overrides.put("getMinUserNameLength", args -> 3);
        overrides.put("getMaxUserNameLength", args -> 20);
        overrides.put("getMinPasswordLength", args -> 8);
        overrides.put("getMaxPasswordLength", args -> 100);
        overrides.put("getInvalidUserNamePattern", args -> "[^a-zA-Z0-9_-]");
        overrides.put("getMailUser", args -> "");
        overrides.put("getMailgunApiKey", args -> "");
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
        overrides.put("connectToSession", args -> true);
    }

    @AfterEach
    void tearDown() {
        AuthorizedUserRepository.use(previousAccounts);
        AccessPolicy.use(null);
        TrustedHeaderAuth.use(null);
    }

    // --- the header's value -----------------------------------------------------------------------

    @Test
    void offUnlessAHeaderIsNamed() {
        TrustedHeaderAuth off = new TrustedHeaderAuth("", false, false);
        assertThat(off.isEnabled()).isFalse();
        assertThat(off.passwordSignIn()).as("passwords can't be turned off without a sign-on").isTrue();
        assertThat(off.nameFrom("alice")).isNull();
    }

    @Test
    void readsOneNameFromTheHeader() {
        TrustedHeaderAuth auth = new TrustedHeaderAuth("Remote-User", true, false);
        assertThat(auth.nameFrom("  alice ")).isEqualTo("alice");
        assertThat(auth.nameFrom("")).isNull();
        assertThat(auth.nameFrom(null)).isNull();
        assertThat(auth.nameFrom("alice, owner")).as("the header sent twice").isNull();
        assertThat(auth.nameFrom(new String(new char[TrustedHeaderAuth.MAX_VALUE_LENGTH + 1]).replace('\0', 'a'))).isNull();
        assertThat(auth.nameFrom("alice@example.com")).as("kept whole unless asked").isEqualTo("alice@example.com");
        assertThat(new TrustedHeaderAuth("Tailscale-User-Login", true, true).nameFrom("alice@github")).isEqualTo("alice");
    }

    // --- accounts ---------------------------------------------------------------------------------

    @Test
    void linksTheAccountWithThatNameInAnyCase() {
        accounts.add("Alice", PASSWORD, "alice@example.test");
        assertThat(sso().account(accounts, "alice", 2000)).isEqualTo("Alice");
        assertThat(accounts.getByNameIgnoreCase("ALICE").getEmail()).isEqualTo("alice@example.test");
    }

    @Test
    void createsAnAccountWithoutEmailOnTheFirstSignIn() {
        assertThat(sso().account(accounts, "bob", 2000)).isEqualTo("bob");
        AuthorizedUser bob = accounts.getByName("bob");
        assertThat(bob).isNotNull();
        assertThat(bob.getEmail()).isNull();
        assertThat(bob.doCredentialsMatch("bob", "")).as("nobody knows its password").isFalse();
        // a second account without email works too
        assertThat(sso().account(accounts, "carol", 2000)).isEqualTo("carol");
    }

    @Test
    void refusesNamesThatCantBePlayerNames() {
        assertThatThrownBy(() -> sso().account(accounts, "alice@example.com", 2000)).hasMessageContaining("can't be a player name");
        assertThatThrownBy(() -> sso().account(accounts, "Admin", 2000)).hasMessageContaining("can't be a player name");
        assertThatThrownBy(() -> sso().account(accounts, "al", 2000)).hasMessageContaining("can't be a player name");
        assertThat(accounts.getByNameIgnoreCase("alice@example.com")).isNull();
    }

    @Test
    void aContributorsServerStillNeedsTheAllowlist() throws Exception {
        AccessPolicy.get().setMode(AccessPolicy.ALLOWLIST, "console", "test", 1500);
        store.allow("alice", "a contributor", "console", 1500);

        assertThatThrownBy(() -> sso().account(accounts, "mallory", 2000)).hasMessage(AccessPolicy.REFUSED);
        assertThat(accounts.getByName("mallory")).isNull();
        assertThat(sso().account(accounts, "alice", 2000)).isEqualTo("alice");
        assertThat(sso().account(accounts, "owner", 2000)).isEqualTo("owner");
    }

    // --- signing in -------------------------------------------------------------------------------

    @Test
    void aSignedOnLoginNeedsNoPassword() throws Exception {
        TrustedHeaderAuth.use(sso());
        accounts.add("alice", PASSWORD, "alice@example.test");
        Session session = newSession();
        session.trustSignedOnLogin("alice");
        assertThat(session.connectUser("alice", "", "")).isNull();
    }

    @Test
    void theTrustIsForOneLoginAsThatName() throws Exception {
        TrustedHeaderAuth.use(sso());
        accounts.add("alice", PASSWORD, "alice@example.test");
        accounts.add("bob", PASSWORD, "bob@example.test");
        Session session = newSession();
        session.trustSignedOnLogin("alice");
        assertThat(session.connectUser("bob", "", "")).contains("Wrong username");
        assertThat(session.connectUser("alice", "", "")).as("used up by the try").contains("Wrong username");
    }

    @Test
    void passwordsStillWorkWhenNotTurnedOff() throws Exception {
        TrustedHeaderAuth.use(sso());
        assertThat(newSession().registerUser("carol", PASSWORD, "carol@example.test")).isNull();
        assertThat(newSession().connectUser("carol", PASSWORD, "")).isNull();
    }

    @Test
    void withPasswordsOffOnlyTheSignOnSignsIn() throws Exception {
        TrustedHeaderAuth.use(new TrustedHeaderAuth("Remote-User", false, false));
        accounts.add("alice", PASSWORD, "alice@example.test");

        assertThat(newSession().connectUser("alice", PASSWORD, "")).isEqualTo(TrustedHeaderAuth.PASSWORDS_OFF);
        Session remembered = newSession();
        remembered.trustRememberedLogin("alice");
        assertThat(remembered.connectUser("alice", "", "")).isEqualTo(TrustedHeaderAuth.PASSWORDS_OFF);
        assertThat(newSession().registerUser("dave", PASSWORD, "dave@example.test")).isEqualTo(TrustedHeaderAuth.PASSWORDS_OFF);
        assertThat(accounts.getByName("dave")).isNull();
        assertThat(new AccountService(managers).sendResetCode("alice@example.test")).isEqualTo(TrustedHeaderAuth.PASSWORDS_OFF);

        Session signedOn = newSession();
        signedOn.trustSignedOnLogin("alice");
        assertThat(signedOn.connectUser("alice", "", "")).isNull();

        // a restore token alone doesn't sign in either; with the sign-on it does
        users.get("alice").setRestoreSessionId("restore-me");
        assertThat(newSession().connectUser("alice", "", "restore-me")).isEqualTo(TrustedHeaderAuth.PASSWORDS_OFF);
        Session restoring = newSession();
        restoring.trustSignedOnLogin("alice");
        assertThat(restoring.connectUser("alice", "", "restore-me")).isNull();
    }

    @Test
    void deactivatedAccountsStayOut() throws Exception {
        TrustedHeaderAuth.use(sso());
        accounts.add("alice", PASSWORD, "alice@example.test");
        AuthorizedUser alice = accounts.getByName("alice");
        alice.active = false;
        accounts.update(alice);

        Session session = newSession();
        session.trustSignedOnLogin("alice");
        assertThat(session.connectUser("alice", "", "")).contains("deactivated");
    }

    private static TrustedHeaderAuth sso() {
        return new TrustedHeaderAuth("Remote-User", true, false);
    }

    private Session newSession() {
        Session session = new Session(managers, UUID.randomUUID().toString(), null);
        session.setHost("192.0.2.10");
        return session;
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
