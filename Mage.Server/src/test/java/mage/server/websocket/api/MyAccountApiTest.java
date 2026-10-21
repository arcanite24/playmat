package mage.server.websocket.api;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import mage.server.AuthorizedUserRepository;
import mage.server.admin.AccessPolicy;
import mage.server.admin.AdminStore;
import mage.server.admin.StaffRole;
import mage.server.managers.ManagerFactory;
import mage.server.websocket.rpc.RpcCall;
import mage.server.websocket.rpc.RpcConnection;
import mage.server.websocket.rpc.RpcMethod;
import mage.server.websocket.rpc.RpcSessions;
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
 * A player's own data: downloading it, and deleting the account with its password (or its exact name, for an account
 * that signs in with GitHub), never the owner's.
 */
public class MyAccountApiTest {

    private static final String PASSWORD = "correct horse 7";

    @TempDir
    Path dir;

    private final Map<String, Function<Object[], Object>> overrides = new HashMap<>();
    private final Map<String, String> sessionNames = new HashMap<>();
    private AuthorizedUserRepository previousAccounts;
    private AuthorizedUserRepository accounts;
    private AdminStore store;
    private Map<String, RpcMethod> methods;
    // names no other test or local server uses: deletion reaches the server's own deck and stats stores
    private final String pat = "pat" + UUID.randomUUID().toString().substring(0, 6);
    private final String gita = "gita" + UUID.randomUUID().toString().substring(0, 6);

    @BeforeEach
    void setUp() throws Exception {
        previousAccounts = AuthorizedUserRepository.getInstance();
        accounts = new AuthorizedUserRepository("jdbc:h2:mem:self-" + UUID.randomUUID() + ";DB_CLOSE_DELAY=-1");
        AuthorizedUserRepository.use(accounts);
        store = new AdminStore("jdbc:sqlite:" + dir.resolve("admin.db"));
        AccessPolicy.use(AccessPolicy.create(store, "Owner", "open", "", 1000));
        ManagerFactory managers = deepProxy(ManagerFactory.class);
        overrides.put("isAuthenticationActivated", args -> true);
        RpcSessions sessions = (RpcSessions) Proxy.newProxyInstance(RpcSessions.class.getClassLoader(), new Class<?>[]{RpcSessions.class},
                (proxy, method, args) -> "userName".equals(method.getName()) ? Optional.ofNullable(sessionNames.get((String) args[0])) : null);
        methods = new HashMap<>();
        for (RpcMethod method : MyAccountApi.methods(new ApiContext(null, managers, sessions))) {
            methods.put(method.getName(), method);
        }
        accounts.add(pat, PASSWORD, pat + "@example.test");
        accounts.add("Owner", PASSWORD, "owner@example.test");
        accounts.add(gita, "unknown to anyone 9", gita + "@example.test");
        store.linkGithub(42, gita + "-gh", gita, 1);
        sessionNames.put("s-pat", pat);
        sessionNames.put("s-owner", "Owner");
        sessionNames.put("s-gita", gita);
    }

    @AfterEach
    void tearDown() {
        AuthorizedUserRepository.use(previousAccounts);
        AccessPolicy.use(null);
    }

    @Test
    void theExportHasTheAccountWithoutItsPassword() throws Exception {
        String json = (String) call("accountExport", "s-pat");
        JsonObject export = JsonParser.parseString(json).getAsJsonObject();
        assertThat(export.getAsJsonObject("account").get("name").getAsString()).isEqualTo(pat);
        assertThat(export.getAsJsonObject("account").get("email").getAsString()).isEqualTo(pat + "@example.test");
        assertThat(json).doesNotContain("salt").doesNotContain("password").doesNotContain(PASSWORD);
        assertThat(export.has("decks")).isTrue();
    }

    @Test
    void deletingNeedsThePasswordAndTheExactName() throws Exception {
        store.grantStaff(pat, StaffRole.MODERATOR, "Owner", 1);

        assertThatThrownBy(() -> call("accountDelete", "s-pat", PASSWORD, pat.toUpperCase())).hasMessageContaining("exactly");
        assertThatThrownBy(() -> call("accountDelete", "s-pat", "wrong horse 1", pat)).hasMessageContaining("password is wrong");
        assertThatThrownBy(() -> call("accountDelete", "s-pat", "", pat)).hasMessageContaining("password is wrong");
        assertThat(accounts.getByName(pat)).isNotNull();

        assertThat(call("accountDelete", "s-pat", PASSWORD, pat)).isEqualTo(true);
        assertThat(accounts.getByName(pat)).isNull();
        assertThat(store.staffRole(pat)).as("the staff role goes too").isNull();
        assertThat(store.auditLog(pat, "user.delete.self", pat, 0, 10)).hasSize(1);
    }

    @Test
    void aGithubAccountConfirmsWithItsName() throws Exception {
        assertThatThrownBy(() -> call("accountDelete", "s-gita", "", "someone")).hasMessageContaining("exactly");
        assertThat(call("accountDelete", "s-gita", "", gita)).isEqualTo(true);
        assertThat(accounts.getByName(gita)).isNull();
        assertThat(store.githubLinkOf(gita)).isNull();
    }

    @Test
    void theOwnerCantDeleteThemself() {
        assertThatThrownBy(() -> call("accountDelete", "s-owner", PASSWORD, "Owner")).hasMessageContaining("owner");
        assertThat(accounts.getByName("Owner")).isNotNull();
    }

    @Test
    void passwordGuessesThroughDeletionAreLimited() {
        for (int i = 0; i < MyAccountApi.DELETE_TRIES_PER_HOUR; i++) {
            assertThatThrownBy(() -> call("accountDelete", "s-pat", "wrong horse 1", pat)).hasMessageContaining("password is wrong");
        }
        assertThatThrownBy(() -> call("accountDelete", "s-pat", PASSWORD, pat)).hasMessageContaining("Too many tries");
        assertThat(accounts.getByName(pat)).isNotNull();
    }

    @Test
    void signedOutCallersGetNothing() {
        assertThatThrownBy(() -> call("accountExport", "s-nobody")).hasMessageContaining("Sign in");
    }

    private Object call(String name, String sessionId, String... params) throws Exception {
        JsonArray array = new JsonArray();
        for (String param : params) {
            array.add(param);
        }
        RpcConnection connection = (RpcConnection) Proxy.newProxyInstance(RpcConnection.class.getClassLoader(),
                new Class<?>[]{RpcConnection.class}, (proxy, method, args) -> "getSessionId".equals(method.getName()) ? sessionId : null);
        RpcMethod method = methods.get(name);
        return method.getHandler().handle(new RpcCall(method, array, connection));
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
