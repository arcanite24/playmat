package mage.server.websocket;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import com.google.gson.JsonPrimitive;
import mage.interfaces.MageServer;
import mage.interfaces.ServerState;
import mage.server.DisconnectReason;
import mage.server.websocket.api.ApiContext;
import mage.server.websocket.api.WebClientApi;
import mage.server.websocket.rpc.RpcConnection;
import mage.server.websocket.rpc.RpcDispatcher;
import mage.server.websocket.rpc.RpcException;
import mage.server.websocket.rpc.RpcMethod;
import mage.server.websocket.rpc.RpcParam;
import mage.server.websocket.rpc.RpcSessions;
import org.jboss.remoting.callback.InvokerCallbackHandler;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.DynamicTest;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestFactory;

import java.lang.reflect.Proxy;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.UUID;
import java.util.stream.Collectors;
import java.util.stream.Stream;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

/**
 * Contract tests for the web client bridge: every RPC method maps to the right server call,
 * with the caller's own session, and access rules hold.
 */
public class WebClientApiContractTest {

    // RPC methods that are served by the bridge itself instead of a same-named MageServer method
    private static final Map<String, String> SERVER_METHOD_OVERRIDES = new HashMap<>();
    private static final Set<String> NOT_MAPPED_TO_SERVER = new HashSet<>(Arrays.asList(
            "disconnectSession", "playerLogout", "sessionGetRestoreToken", // session manager
            "getExpansionSets", "getBasicLandSets", "searchCards", "lookupCards", "deckValidate", // card database services
            "deckImportFromUrl", "deckImportSources", // deck website import service
            "testEndGame", "testConcedeMatch", // test mode helpers on table manager
            "gameResync", // the bridge resends its own copy of the open question
            "setCapabilities", "gameStateResync", // bridge protocol options and game state patches
            "chatSetIgnored", "gameWatchers", // user and game managers (social/UserRelations, watcher names)
            "replayList", "replayPage", // replay store (replay/ReplayStoreTest)
            "serverInfo", "deckSyncList", "deckSyncGet", "deckSyncPut", "deckSyncDelete", // config and deck store (decks/DeckStoreTest)
            "reportPlayer", "clientReportError", "adminGetReports", "adminCloseReport", "adminClientErrors", // moderation stores
            "adminServerStats", // bridge metrics
            "gamePractice", // game controller practice tools, with readable errors
            "careerState", "careerStarters", "careerStart", "careerCollection", "careerOpponents", "careerPlay", "careerShop",
            "careerBuyPack", "careerCraft", "careerExport", "careerImport", // career store and service (career/CareerServiceTest)
            "careerQuests", "careerRerollQuest", "careerAchievements", "careerLevels", "careerWeekly", "careerGameResult",
            "careerSetProgress", "careerCountAiGames", // career progress (career/CareerProgressTest)
            "careerCampaigns", "careerCampaignPlay", "careerCampaignDeck", "careerCampaignChoose", "careerPuzzles", "careerPuzzlePlay",
            "careerChallenge", "careerChallengePlay", "careerGauntlet", "careerGauntletStart", "careerGauntletPick",
            "careerGauntletPlay", "careerGauntletAbandon", "careerGauntletMove", "careerLimited", "careerLimitedStart", "careerLimitedPick",
            "careerLimitedDeck", "careerLimitedCoach", "careerLimitedPlay", "careerLimitedAbandon", // career modes (career/CareerModesTest)
            "authRegister", "authSendTokenToEmail", "authResetPassword", // session and AccountService, with readable errors
            "authRememberLogin", "authForgetLogin", // remembered devices in the account database
            "adminAudit", "adminAccess", "adminAccessPreview", "adminSetAccessMode", "adminAllowAdd", "adminAllowRemovePreview",
            "adminAllowRemove", "adminInviteCreate", "adminInviteRevoke", "adminAccessDecide", "adminSetNotice", "inviteCheck",
            "accessRequest", "githubStart", "githubFinish", // access policy and admin store (admin/AccessPolicyTest, AccessGateTest)
            "adminGetUsers", "adminDisconnectUser", "adminEndUserSession", "adminMuteUser", "adminLockUser", "adminActivateUser",
            "adminToggleActivateUser", "adminTableRemove", "adminSendBroadcastMessage", // Moderation, for staff on player sessions too
            "staffMe", "adminStaff", "adminStaffGrant", "adminStaffRevoke", // staff roles (api/StaffRolesTest)
            "adminAccounts", "adminAccount", "adminResetCode", "adminAccountExport", "adminDeleteAccount", "adminFeedback",
            "adminFeedbackMark", "sendFeedback", // accounts and feedback (api/AccountsApiTest)
            "adminTableWatch", "adminClientErrorResolve", "adminSettings", "adminSetSetting", "adminMaintenance",
            "adminMaintenanceCancel", "adminDiscordTest", // watching, client errors, settings and maintenance (M15)
            "soloSaves", "soloResume", "soloDelete", "soloSaveAndLeave", "soloFork", "soloAssists", "soloTurns", "soloWhatIf", "gameAdvise", "soloBatchSubmit",
            "soloBatchJobs", "soloBatchJob", "soloBatchCancel", "leaderboard", "adminBatchJobs", "adminBatchCancel",
            "adminContentPacks", "adminContentPackUpload", "adminContentPackRemove", "adminContentPackReload" // solo (solo/SoloFoundationsTest)
    ));

    static {
        SERVER_METHOD_OVERRIDES.put("submitDeck", "deckSubmit");
        SERVER_METHOD_OVERRIDES.put("getGameTypes", "getServerState");
        SERVER_METHOD_OVERRIDES.put("getTournamentGameTypes", "getServerState");
        SERVER_METHOD_OVERRIDES.put("getTournamentTypes", "getServerState");
        SERVER_METHOD_OVERRIDES.put("getPlayerTypes", "getServerState");
        SERVER_METHOD_OVERRIDES.put("getDeckTypes", "getServerState");
        SERVER_METHOD_OVERRIDES.put("getDraftCubes", "getServerState");
    }

    private static final String ADMIN_PASSWORD = "admin-secret";

    private final List<String> serverCalls = new ArrayList<>();
    private final Map<String, Object[]> serverArgs = new HashMap<>();
    private FakeSessions sessions;
    private RpcDispatcher dispatcher;

    @BeforeEach
    void setUp() {
        sessions = new FakeSessions();
        dispatcher = WebClientApi.createDispatcher(new ApiContext(proxyServer(), null, sessions, ADMIN_PASSWORD)
                .withStaff(sessionId -> everyoneIsStaff ? Optional.of("staff") : Optional.empty()));
    }

    /** whether the staff check accepts the test's sessions */
    private boolean everyoneIsStaff = true;

    private MageServer proxyServer() {
        return (MageServer) Proxy.newProxyInstance(
                MageServer.class.getClassLoader(),
                new Class<?>[]{MageServer.class},
                (proxy, method, args) -> {
                    serverCalls.add(method.getName());
                    serverArgs.put(method.getName(), args == null ? new Object[0] : args);
                    if (method.getReturnType() == boolean.class) {
                        return true;
                    }
                    if (method.getReturnType() == ServerState.class) {
                        return new ServerState(Collections.emptyList(), Collections.emptyList(),
                                new mage.players.PlayerType[0], new String[0], new String[0], false, null, 0, 0);
                    }
                    return null;
                });
    }

    @TestFactory
    Stream<DynamicTest> everyMethodCallsTheServerWithTheCallersSession() {
        return dispatcher.getMethods().stream()
                .filter(method -> !NOT_MAPPED_TO_SERVER.contains(method.getName()))
                .map(method -> DynamicTest.dynamicTest(method.getName(), () -> {
                    setUp();
                    FakeConnection connection = new FakeConnection();
                    if (method.getAccess() != RpcMethod.Access.ESTABLISH) {
                        login(connection);
                    }

                    dispatcher.dispatch(method.getName(), sampleParams(method, "client-chosen-session"), connection);

                    String expected = SERVER_METHOD_OVERRIDES.getOrDefault(method.getName(),
                            method.getAliasOf() != null ? method.getAliasOf() : method.getName());
                    expected = SERVER_METHOD_OVERRIDES.getOrDefault(expected, expected);
                    assertThat(serverCalls).as("server calls of " + method.getName()).contains(expected);

                    if (method.getSessionParam() >= 0) {
                        List<Object> args = Arrays.asList(serverArgs.get(expected));
                        assertThat(args).as("session passed to the server by " + method.getName())
                                .contains(connection.getSessionId())
                                .doesNotContain("client-chosen-session");
                    }
                }));
    }

    @Test
    void adminCallsOfPlayersFailAndNeverReachTheServer() throws Exception {
        everyoneIsStaff = false;
        for (RpcMethod method : dispatcher.getMethods()) {
            if (!method.getName().startsWith("admin")) {
                continue;
            }
            FakeConnection connection = new FakeConnection();
            login(connection);
            assertThatThrownBy(() -> dispatcher.dispatch(method.getName(), sampleParams(method, "x"), connection))
                    .as(method.getName()).hasMessageContaining("Staff only");
            assertThat(serverCalls).as("server calls of " + method.getName()).isEmpty();
        }
    }

    @Test
    void everyMethodIsDocumented() {
        for (RpcMethod method : dispatcher.getMethods()) {
            assertThat(method.getDescription()).as(method.getName()).isNotBlank();
            if (method.getSessionParam() >= 0) {
                assertThat(method.getParams().get(method.getSessionParam()).getName())
                        .as("session param of " + method.getName()).isEqualTo("sessionId");
            }
        }
    }

    @Test
    void sessionMethodsRequireLogin() {
        FakeConnection connection = new FakeConnection();
        assertThatThrownBy(() -> dispatcher.dispatch("roomGetAllTables", params(UUID.randomUUID().toString()), connection))
                .isInstanceOfSatisfying(RpcException.class, e -> assertThat(e.getCode()).isEqualTo(RpcException.NOT_AUTHORIZED));
        assertThat(serverCalls).isEmpty();
    }

    @Test
    void loginIssuesAServerSessionAndIgnoresTheClientsId() throws Exception {
        FakeConnection connection = new FakeConnection();
        dispatcher.dispatch("connectUser", params("alice", "", "my-own-id"), connection);

        assertThat(connection.getSessionId()).isNotNull().isNotEqualTo("my-own-id");
        assertThat(sessions.created).containsExactly(connection.getSessionId());
        assertThat(sessions.hosts.get(connection.getSessionId())).isEqualTo("10.0.0.7");
        assertThat(serverArgs.get("connectUser")[2]).isEqualTo(connection.getSessionId());
    }

    @Test
    void reLoginOnTheSameConnectionReplacesTheOldSession() throws Exception {
        FakeConnection connection = new FakeConnection();
        dispatcher.dispatch("connectUser", params("alice", "", "x"), connection);
        String first = connection.getSessionId();
        dispatcher.dispatch("connectUser", params("alice", "", "x"), connection);

        assertThat(connection.getSessionId()).isNotEqualTo(first);
        assertThat(sessions.disconnected).containsExactly(first + ":" + DisconnectReason.LostConnection);
    }

    @Test
    void anotherConnectionCannotUseASessionIdItDoesNotOwn() throws Exception {
        FakeConnection alice = new FakeConnection();
        login(alice);
        FakeConnection mallory = new FakeConnection();
        login(mallory);

        dispatcher.dispatch("matchQuit", params(UUID.randomUUID().toString(), alice.getSessionId()), mallory);

        assertThat(serverArgs.get("matchQuit")[1]).isEqualTo(mallory.getSessionId());
    }

    @Test
    void chatAuthorIsTheLoggedInUser() throws Exception {
        FakeConnection connection = new FakeConnection();
        login(connection);
        sessions.names.put(connection.getSessionId(), "alice");

        dispatcher.dispatch("chatSendMessage", params(UUID.randomUUID().toString(), "admin", "hi"), connection);

        assertThat(serverArgs.get("chatSendMessage")[1]).isEqualTo("alice");
    }

    @Test
    void pingWorksBeforeLogin() throws Exception {
        assertThat(dispatcher.dispatch("ping", new JsonArray(), new FakeConnection())).isEqualTo(true);
        assertThat(serverCalls).isEmpty();
    }

    @Test
    void unknownMethodAndBadParamsAreReported() throws Exception {
        FakeConnection connection = new FakeConnection();
        login(connection);

        assertThatThrownBy(() -> dispatcher.dispatch("noSuchMethod", new JsonArray(), connection))
                .isInstanceOfSatisfying(RpcException.class, e -> assertThat(e.getCode()).isEqualTo(RpcException.METHOD_NOT_FOUND));
        assertThatThrownBy(() -> dispatcher.dispatch("roomGetAllTables", new JsonArray(), connection))
                .isInstanceOfSatisfying(RpcException.class, e -> assertThat(e.getCode()).isEqualTo(RpcException.INVALID_PARAMS));
        assertThatThrownBy(() -> dispatcher.dispatch("roomGetAllTables", params("not-a-uuid"), connection))
                .isInstanceOfSatisfying(RpcException.class, e -> assertThat(e.getCode()).isEqualTo(RpcException.INVALID_PARAMS));
        assertThatThrownBy(() -> dispatcher.dispatch("sendPlayerAction",
                params("NOT_AN_ACTION", UUID.randomUUID().toString(), "s"), connection))
                .isInstanceOfSatisfying(RpcException.class, e -> assertThat(e.getCode()).isEqualTo(RpcException.INVALID_PARAMS));
    }

    @Test
    void chatPostsRequireMembership() throws Exception {
        FakeConnection connection = new FakeConnection();
        login(connection);
        sessions.everyoneIsInEveryChat = false;
        String joined = UUID.randomUUID().toString();
        String other = UUID.randomUUID().toString();
        sessions.chatMembers.add(connection.getSessionId() + ":" + joined);

        dispatcher.dispatch("chatSendMessage", params(joined, "", "hi"), connection);
        assertThat(serverCalls).containsExactly("chatSendMessage");

        serverCalls.clear();
        assertThatThrownBy(() -> dispatcher.dispatch("chatSendMessage", params(other, "", "hi"), connection))
                .isInstanceOfSatisfying(RpcException.class, e -> assertThat(e.getCode()).isEqualTo(RpcException.NOT_AUTHORIZED));
        assertThat(serverCalls).isEmpty();
    }

    @Test
    void chatAndTableLookupsRequireLogin() {
        for (String method : Arrays.asList("chatSendMessage", "chatFindByGame", "chatFindByTable", "chatFindByTournament",
                "chatFindByRoom", "roomGetAllTables", "roomGetTableById", "roomGetUsers", "roomGetFinishedMatches")) {
            RpcMethod rpc = dispatcher.find(method);
            assertThat(rpc.getAccess()).as(method).isEqualTo(RpcMethod.Access.SESSION);
            assertThatThrownBy(() -> dispatcher.dispatch(method, sampleParams(rpc, "x"), new FakeConnection()))
                    .as(method)
                    .isInstanceOfSatisfying(RpcException.class, e -> assertThat(e.getCode()).isEqualTo(RpcException.NOT_AUTHORIZED));
        }
        assertThat(serverCalls).isEmpty();
    }

    @Test
    void adminLoginIsDisabledWithoutAnAdminPassword() {
        RpcDispatcher noAdmin = WebClientApi.createDispatcher(new ApiContext(proxyServer(), null, sessions, ""));
        FakeConnection connection = new FakeConnection();

        assertThatThrownBy(() -> noAdmin.dispatch("connectAdmin", params("", "x"), connection))
                .isInstanceOfSatisfying(RpcException.class, e -> assertThat(e.getCode()).isEqualTo(RpcException.NOT_AUTHORIZED));
        assertThat(serverCalls).doesNotContain("connectAdmin");
        assertThat(connection.getSessionId()).as("failed admin login leaves no session").isNull();
        assertThat(sessions.created).hasSize(1);
        assertThat(sessions.exists(sessions.created.get(0))).isFalse();
    }

    @Test
    void wrongAdminPasswordLocksTheAddressOutWithoutWaiting() throws Exception {
        FakeConnection attacker = new FakeConnection("203.0.113.9");
        long start = System.nanoTime();
        assertThatThrownBy(() -> dispatcher.dispatch("connectAdmin", params("guess", "x"), attacker))
                .isInstanceOfSatisfying(RpcException.class, e -> assertThat(e.getCode()).isEqualTo(RpcException.NOT_AUTHORIZED));
        // locked out now, even with the right password
        assertThatThrownBy(() -> dispatcher.dispatch("connectAdmin", params(ADMIN_PASSWORD, "x"), new FakeConnection("203.0.113.9")))
                .isInstanceOfSatisfying(RpcException.class, e -> assertThat(e.getCode()).isEqualTo(RpcException.RATE_LIMITED));
        assertThat(System.nanoTime() - start).as("no sleeping on a worker thread").isLessThan(1_000_000_000L);
        assertThat(serverCalls).doesNotContain("connectAdmin");
        assertThat(attacker.getSessionId()).isNull();

        // other addresses are not affected
        FakeConnection admin = new FakeConnection("198.51.100.1");
        assertThat(dispatcher.dispatch("connectAdmin", params(ADMIN_PASSWORD, "x"), admin)).isEqualTo(true);
        assertThat(serverCalls).contains("connectAdmin");
        assertThat(admin.getSessionId()).isNotNull();
    }

    @Test
    void restoreTokenIsTheServerIssuedOneAndIsPassedBackOnLogin() throws Exception {
        FakeConnection first = new FakeConnection();
        dispatcher.dispatch("connectUser", params("alice", "", "x", "client-made-up-id"), first);
        Object token = dispatcher.dispatch("sessionGetRestoreToken", params("x"), first);
        assertThat(token).isEqualTo(first.getSessionId());

        FakeConnection second = new FakeConnection("192.0.2.44");
        dispatcher.dispatch("connectUser", params("alice", "", "x", (String) token), second);
        assertThat(serverArgs.get("connectUser")[3]).as("restoreSessionId reaches the server").isEqualTo(token);
        assertThat(second.getSessionId()).isNotEqualTo(token);

        assertThatThrownBy(() -> dispatcher.dispatch("sessionGetRestoreToken", params("x"), new FakeConnection()))
                .isInstanceOfSatisfying(RpcException.class, e -> assertThat(e.getCode()).isEqualTo(RpcException.NOT_AUTHORIZED));
    }

    @Test
    void rememberedLoginTokenIsCheckedForTheNewSessionBeforeLogin() throws Exception {
        FakeConnection first = new FakeConnection();
        dispatcher.dispatch("connectUser", params("alice", "secret", "x"), first);
        sessions.names.put(first.getSessionId(), "alice");
        Object token = dispatcher.dispatch("authRememberLogin", params("x"), first);

        FakeConnection second = new FakeConnection("192.0.2.44");
        dispatcher.dispatch("connectUser", params("alice", "", "x", "", "", "", (String) token), second);
        assertThat(sessions.trusted).containsExactly(second.getSessionId() + ":alice");

        FakeConnection third = new FakeConnection("192.0.2.45");
        dispatcher.dispatch("connectUser", params("mallory", "", "x", "", "", "", (String) token), third);
        assertThat(sessions.trusted).as("another user's token proves nothing").hasSize(1);

        assertThat(dispatcher.dispatch("authForgetLogin", params("alice", (String) token), new FakeConnection())).isEqualTo(true);
        assertThat(sessions.rememberedLogins).isEmpty();
        assertThatThrownBy(() -> dispatcher.dispatch("authRememberLogin", params("x"), new FakeConnection()))
                .isInstanceOfSatisfying(RpcException.class, e -> assertThat(e.getCode()).isEqualTo(RpcException.NOT_AUTHORIZED));
    }

    @Test
    void methodNamesAreUnique() {
        List<String> names = dispatcher.getMethods().stream().map(RpcMethod::getName).collect(Collectors.toList());
        assertThat(names).doesNotHaveDuplicates();
    }

    private void login(FakeConnection connection) throws Exception {
        dispatcher.dispatch("connectUser", params("user", "", "ignored"), connection);
        serverCalls.clear();
        serverArgs.clear();
    }

    private static JsonArray params(String... values) {
        JsonArray array = new JsonArray();
        for (String value : values) {
            array.add(value);
        }
        return array;
    }

    private static JsonArray sampleParams(RpcMethod method, String sessionValue) {
        JsonArray array = new JsonArray();
        for (int i = 0; i < method.getParams().size(); i++) {
            RpcParam param = method.getParams().get(i);
            if (i == method.getSessionParam()) {
                array.add(sessionValue);
                continue;
            }
            if ("connectAdmin".equals(method.getName()) && "password".equals(param.getName())) {
                array.add(ADMIN_PASSWORD);
                continue;
            }
            switch (param.getTsType()) {
                case "PlayerAction":
                    array.add("PASS_PRIORITY_UNTIL_NEXT_TURN");
                    continue;
                case "ManaType":
                    array.add("RED");
                    continue;
                case "UUID[]":
                    array.add(new JsonArray());
                    continue;
                default:
                    break;
            }
            switch (param.getType()) {
                case UUID:
                    array.add(UUID.randomUUID().toString());
                    break;
                case INT:
                    array.add(new JsonPrimitive(1));
                    break;
                case BOOLEAN:
                    array.add(new JsonPrimitive(true));
                    break;
                case OBJECT:
                    JsonObject object = new JsonObject();
                    if ("DeckCardLists".equals(param.getTsType())) {
                        object.add("cards", new JsonArray());
                        object.add("sideboard", new JsonArray());
                    }
                    array.add(object);
                    break;
                case ARRAY:
                    array.add(new JsonArray());
                    break;
                case STRING:
                    array.add(param.getName().equals("playerType") ? "HUMAN" : "text");
                    break;
                case ANY:
                default:
                    array.add("text");
                    break;
            }
        }
        return array;
    }

    private static final class FakeConnection implements RpcConnection {
        private final String host;
        private String sessionId;

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
        public String getRemoteHost() {
            return host;
        }

        @Override
        public InvokerCallbackHandler createCallbackHandler() {
            return null;
        }
    }

    private static final class FakeSessions implements RpcSessions {
        final List<String> created = new ArrayList<>();
        final List<String> disconnected = new ArrayList<>();
        final Map<String, String> hosts = new HashMap<>();
        final Map<String, String> names = new HashMap<>();
        final Set<String> chatMembers = new HashSet<>(); // "sessionId:chatId"
        final Map<String, String> rememberedLogins = new HashMap<>(); // token -> user name
        final List<String> trusted = new ArrayList<>(); // "sessionId:userName"
        boolean everyoneIsInEveryChat = true;

        @Override
        public void create(String sessionId, InvokerCallbackHandler callbackHandler, String host) {
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
        public Optional<String> userName(String sessionId) {
            return Optional.of(names.getOrDefault(sessionId, "user"));
        }

        @Override
        public Optional<String> restoreToken(String sessionId) {
            // like the server: the restore id of a logged in user is its current session id
            return exists(sessionId) ? Optional.of(sessionId) : Optional.empty();
        }

        @Override
        public Optional<String> rememberLogin(String sessionId) {
            String token = "token-" + rememberedLogins.size();
            rememberedLogins.put(token, userName(sessionId).orElse(""));
            return Optional.of(token);
        }

        @Override
        public void trustRememberedLogin(String sessionId, String userName, String token) {
            if (userName.equals(rememberedLogins.get(token))) {
                trusted.add(sessionId + ":" + userName);
            }
        }

        @Override
        public void forgetLogin(String userName, String token) {
            rememberedLogins.remove(token, userName);
        }

        @Override
        public boolean isChatMember(String sessionId, UUID chatId) {
            return everyoneIsInEveryChat || chatMembers.contains(sessionId + ":" + chatId);
        }
    }
}
