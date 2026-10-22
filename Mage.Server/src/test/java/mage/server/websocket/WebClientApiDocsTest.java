package mage.server.websocket;

import mage.interfaces.MageServer;
import mage.interfaces.callback.ClientCallbackMethod;
import mage.server.websocket.api.ApiContext;
import mage.server.websocket.api.WebClientApi;
import mage.server.websocket.rpc.RpcMethod;
import mage.server.websocket.rpc.RpcParam;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.lang.reflect.Proxy;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;

import static org.assertj.core.api.Assertions.assertThat;

/**
 * Keeps the web client protocol reference and the client's generated method map in sync with the server.
 * <p>
 * After changing RPC methods or callbacks, regenerate with:
 * {@code mvn -pl Mage.Server test -Dtest=WebClientApiDocsTest -Dxmage.updateWebApiDocs=true}
 */
public class WebClientApiDocsTest {

    private static final Path DOC_FILE = Paths.get("..", "docs", "WebSocketAPI.md");
    private static final Path TS_FILE = Paths.get("..", "Mage.Web.Client", "src", "protocol", "generated", "protocol.ts");
    private static final Path API_FILE = Paths.get("..", "Mage.Web.Client", "src", "protocol", "generated", "api.ts");
    private static final String BEGIN_METHODS = "<!-- BEGIN GENERATED: methods -->";
    private static final String END_METHODS = "<!-- END GENERATED: methods -->";
    private static final String BEGIN_CALLBACKS = "<!-- BEGIN GENERATED: callbacks -->";
    private static final String END_CALLBACKS = "<!-- END GENERATED: callbacks -->";

    // payload of each server callback ("data" field), see where the server fires them
    private static final Map<ClientCallbackMethod, String> CALLBACK_DATA = new HashMap<>();

    static {
        CALLBACK_DATA.put(ClientCallbackMethod.CHATMESSAGE, "ChatMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.SHOW_USERMESSAGE, "string[]");
        CALLBACK_DATA.put(ClientCallbackMethod.SERVER_MESSAGE, "ChatMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.CAREER_PROGRESS, "string");
        CALLBACK_DATA.put(ClientCallbackMethod.GAME_EMOTE, "PlayerEmote");
        CALLBACK_DATA.put(ClientCallbackMethod.JOINED_TABLE, "TableClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.START_TOURNAMENT, "TableClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.TOURNAMENT_INIT, "TournamentView");
        CALLBACK_DATA.put(ClientCallbackMethod.TOURNAMENT_UPDATE, "TournamentView");
        CALLBACK_DATA.put(ClientCallbackMethod.TOURNAMENT_OVER, "string");
        CALLBACK_DATA.put(ClientCallbackMethod.START_DRAFT, "TableClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.SIDEBOARD, "TableClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.CONSTRUCT, "TableClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.DRAFT_OVER, "null");
        CALLBACK_DATA.put(ClientCallbackMethod.DRAFT_INIT, "DraftClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.DRAFT_PICK, "DraftClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.DRAFT_UPDATE, "DraftClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.SHOW_TOURNAMENT, "TableClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.WATCHGAME, "TableClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.VIEW_LIMITED_DECK, "TableClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.VIEW_SIDEBOARD, "TableClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.USER_REQUEST_DIALOG, "UserRequestMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.GAME_REDRAW_GUI, "null");
        CALLBACK_DATA.put(ClientCallbackMethod.START_GAME, "TableClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.GAME_INIT, "GameView");
        CALLBACK_DATA.put(ClientCallbackMethod.GAME_UPDATE_AND_INFORM, "GameClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.GAME_INFORM_PERSONAL, "GameClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.GAME_ERROR, "string");
        CALLBACK_DATA.put(ClientCallbackMethod.GAME_UPDATE, "GameView");
        CALLBACK_DATA.put(ClientCallbackMethod.GAME_TARGET, "GameClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.GAME_CHOOSE_ABILITY, "AbilityPickerView");
        CALLBACK_DATA.put(ClientCallbackMethod.GAME_CHOOSE_PILE, "GameClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.GAME_CHOOSE_CHOICE, "GameClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.GAME_ASK, "GameClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.GAME_SELECT, "GameClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.GAME_PLAY_MANA, "GameClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.GAME_PLAY_XMANA, "GameClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.GAME_GET_AMOUNT, "GameClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.GAME_GET_MULTI_AMOUNT, "GameClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.GAME_OVER, "GameClientMessage");
        CALLBACK_DATA.put(ClientCallbackMethod.END_GAME_INFO, "GameEndView");
        CALLBACK_DATA.put(ClientCallbackMethod.REPLAY_GAME, "null");
        CALLBACK_DATA.put(ClientCallbackMethod.REPLAY_INIT, "GameView");
        CALLBACK_DATA.put(ClientCallbackMethod.REPLAY_UPDATE, "GameView");
        CALLBACK_DATA.put(ClientCallbackMethod.REPLAY_DONE, "string");
    }

    @Test
    void protocolReferenceIsUpToDate() throws IOException {
        List<RpcMethod> methods = createMethods();

        String doc = new String(Files.readAllBytes(DOC_FILE), StandardCharsets.UTF_8);
        String expectedDoc = replaceSection(replaceSection(doc, BEGIN_METHODS, END_METHODS, methodsMarkdown(methods)),
                BEGIN_CALLBACKS, END_CALLBACKS, callbacksMarkdown());
        String expectedTs = typescript(methods);
        String expectedApi = apiTypescript(methods);

        if (Boolean.getBoolean("xmage.updateWebApiDocs")) {
            Files.write(DOC_FILE, expectedDoc.getBytes(StandardCharsets.UTF_8));
            Files.createDirectories(TS_FILE.getParent());
            Files.write(TS_FILE, expectedTs.getBytes(StandardCharsets.UTF_8));
            Files.write(API_FILE, expectedApi.getBytes(StandardCharsets.UTF_8));
            doc = expectedDoc;
        }

        String hint = "regenerate with -Dxmage.updateWebApiDocs=true (see class javadoc)";
        assertThat(doc).as("docs/WebSocketAPI.md is stale, " + hint).isEqualTo(expectedDoc);
        assertThat(TS_FILE).as("generated client protocol is missing, " + hint).exists();
        assertThat(new String(Files.readAllBytes(TS_FILE), StandardCharsets.UTF_8))
                .as("generated client protocol is stale, " + hint).isEqualTo(expectedTs);
        assertThat(API_FILE).as("generated client api is missing, " + hint).exists();
        assertThat(new String(Files.readAllBytes(API_FILE), StandardCharsets.UTF_8))
                .as("generated client api is stale, " + hint).isEqualTo(expectedApi);
    }

    @Test
    void everyCallbackHasADocumentedPayload() {
        for (ClientCallbackMethod method : ClientCallbackMethod.values()) {
            assertThat(CALLBACK_DATA).as("payload type of " + method).containsKey(method);
        }
    }

    private static List<RpcMethod> createMethods() {
        MageServer server = (MageServer) Proxy.newProxyInstance(MageServer.class.getClassLoader(),
                new Class<?>[]{MageServer.class}, (proxy, method, args) -> null);
        return WebClientApi.createDispatcher(new ApiContext(server, null, null)).getMethods();
    }

    private static String replaceSection(String text, String begin, String end, String content) {
        int start = text.indexOf(begin);
        int finish = text.indexOf(end);
        assertThat(start).as("marker " + begin + " in " + DOC_FILE).isGreaterThanOrEqualTo(0);
        assertThat(finish).as("marker " + end + " in " + DOC_FILE).isGreaterThan(start);
        return text.substring(0, start + begin.length()) + "\n" + content + text.substring(finish);
    }

    private static String methodsMarkdown(List<RpcMethod> methods) {
        StringBuilder sb = new StringBuilder();
        sb.append("| Method | Params | Result | Access | Description |\n");
        sb.append("|--------|--------|--------|--------|-------------|\n");
        for (RpcMethod method : methods) {
            String params = method.getParams().stream()
                    .map(p -> "`" + p.getName() + (p.isOptional() ? "?" : "") + ": " + p.getTsType() + "`")
                    .collect(Collectors.joining(", "));
            sb.append("| `").append(method.getName()).append("` | ")
                    .append(params.isEmpty() ? "-" : params).append(" | `")
                    .append(method.getResultType().replace("|", "\\|")).append("` | ")
                    .append(accessLabel(method)).append(" | ")
                    .append(method.getDescription().replace("|", "\\|")).append(" |\n");
        }
        return sb.toString();
    }

    private static String accessLabel(RpcMethod method) {
        switch (method.getAccess()) {
            case PUBLIC:
                return "public";
            case ESTABLISH:
                return "login";
            case SESSION:
            default:
                return "session";
        }
    }

    private static String callbacksMarkdown() {
        StringBuilder sb = new StringBuilder();
        sb.append("| Method | Data | Delivery | Description |\n");
        sb.append("|--------|------|----------|-------------|\n");
        for (ClientCallbackMethod method : ClientCallbackMethod.values()) {
            sb.append("| `").append(method.name()).append("` | `").append(CALLBACK_DATA.get(method)).append("` | ")
                    .append(method.getType().name())
                    .append(method.getType().canComeInAnyOrder() ? " (may be dropped when outdated)" : " (ordered)")
                    .append(" | ").append(callbackDescription(method)).append(" |\n");
        }
        return sb.toString();
    }

    private static String callbackDescription(ClientCallbackMethod method) {
        switch (method.getType()) {
            case DIALOG:
                return "Prompt: the player must answer with a sendPlayer* call.";
            case UPDATE:
                return "State snapshot; newer ones replace older ones.";
            case MESSAGE:
                return "Message for the player or a log.";
            case CLIENT_SIDE_EVENT:
                return "Not sent by the server.";
            case TABLE_CHANGE:
            default:
                return "Lifecycle event: open, switch or close a screen.";
        }
    }

    /**
     * One function per RPC method, without the sessionId parameter: the server always uses the connection's session.
     */
    private static String apiTypescript(List<RpcMethod> methods) throws IOException {
        java.util.Set<String> viewTypes = readViewTypes();
        java.util.Set<String> usedViews = new java.util.TreeSet<>();
        java.util.Set<String> usedOptions = new java.util.TreeSet<>();
        StringBuilder body = new StringBuilder();
        body.append("export interface RpcCaller {\n");
        body.append("  call<M extends RpcMethodName>(method: M, ...params: RpcMethods[M]['params']): Promise<RpcMethods[M]['result']>;\n");
        body.append("}\n\n");
        body.append("/** Placeholder for the sessionId parameter, which the server replaces with the connection's session. */\n");
        body.append("const SESSION = '';\n\n");
        body.append("/** Every server method as a typed function. Session parameters are filled in automatically. */\n");
        body.append("export function createApi(rpc: RpcCaller) {\n");
        body.append("  return {\n");
        for (RpcMethod method : methods) {
            List<RpcParam> params = method.getParams();
            List<RpcParam> visible = new java.util.ArrayList<>();
            for (int i = 0; i < params.size(); i++) {
                if (i != method.getSessionParam()) {
                    visible.add(params.get(i));
                }
            }
            StringBuilder signature = new StringBuilder();
            for (int i = 0; i < visible.size(); i++) {
                RpcParam p = visible.get(i);
                boolean trailingOptional = true;
                for (int j = i; j < visible.size(); j++) {
                    trailingOptional &= visible.get(j).isOptional();
                }
                if (i > 0) {
                    signature.append(", ");
                }
                String type = p.getTsType();
                collectTypes(type, viewTypes, usedViews, usedOptions);
                if (p.isOptional() && trailingOptional) {
                    signature.append(p.getName()).append("?: ").append(type).append(" | null");
                } else if (p.isOptional()) {
                    signature.append(p.getName()).append(": ").append(type).append(" | null");
                } else {
                    signature.append(p.getName()).append(": ").append(type);
                }
            }
            StringBuilder args = new StringBuilder("'").append(method.getName()).append("'");
            for (int i = 0; i < params.size(); i++) {
                args.append(", ").append(i == method.getSessionParam() ? "SESSION" : params.get(i).getName());
            }
            // omitted optional params travel as null, which the server treats as "not given"
            body.append("    ").append(method.getName()).append(": (").append(signature).append(") =>\n");
            body.append("      rpc.call(").append(args).append("),\n");
        }
        body.append("  };\n");
        body.append("}\n\n");
        body.append("export type Api = ReturnType<typeof createApi>;\n");

        StringBuilder sb = new StringBuilder();
        sb.append("// Generated by Mage.Server WebClientApiDocsTest from the server's RPC registry. Do not edit.\n");
        sb.append("// Regenerate: mvn -pl Mage.Server test -Dtest=WebClientApiDocsTest -Dxmage.updateWebApiDocs=true\n\n");
        sb.append("import type { RpcMethodName, RpcMethods, UUID } from './protocol';\n");
        if (!usedViews.isEmpty()) {
            sb.append("import type { ").append(String.join(", ", usedViews)).append(" } from './views';\n");
        }
        if (!usedOptions.isEmpty()) {
            sb.append("import type { ").append(String.join(", ", usedOptions)).append(" } from '../options';\n");
        }
        sb.append("\n").append(body);
        return sb.toString();
    }

    private static java.util.Set<String> readViewTypes() throws IOException {
        java.util.Set<String> viewTypes = new java.util.TreeSet<>();
        java.util.regex.Matcher exported = java.util.regex.Pattern.compile("export (?:interface|type) (\\w+)")
                .matcher(new String(Files.readAllBytes(VIEWS_FILE), StandardCharsets.UTF_8));
        while (exported.find()) {
            viewTypes.add(exported.group(1));
        }
        return viewTypes;
    }

    private static final Path VIEWS_FILE = Paths.get("..", "Mage.Web.Client", "src", "protocol", "generated", "views.ts");
    // request options and results assembled as plain JSON, typed only on the web side: see src/protocol/options.ts
    private static final java.util.Set<String> CLIENT_OPTION_TYPES = new java.util.HashSet<>(
            java.util.Arrays.asList("WebMatchOptions", "WebTournamentOptions", "WebCardFilters", "ReplayPage"));

    private static String typescript(List<RpcMethod> methods) throws IOException {
        java.util.Set<String> viewTypes = new java.util.TreeSet<>();
        java.util.regex.Matcher exported = java.util.regex.Pattern.compile("export (?:interface|type) (\\w+)")
                .matcher(new String(Files.readAllBytes(VIEWS_FILE), StandardCharsets.UTF_8));
        while (exported.find()) {
            viewTypes.add(exported.group(1));
        }

        java.util.Set<String> usedViews = new java.util.TreeSet<>();
        java.util.Set<String> usedOptions = new java.util.TreeSet<>();
        StringBuilder body = new StringBuilder();

        body.append("export type UUID = string;\n\n");
        body.append("export type RpcAccess = 'public' | 'session' | 'login';\n\n");
        body.append("/** Parameters and result of every RPC method. */\n");
        body.append("export interface RpcMethods {\n");
        for (RpcMethod method : methods) {
            List<RpcParam> params = method.getParams();
            StringBuilder tuple = new StringBuilder();
            for (int i = 0; i < params.size(); i++) {
                RpcParam p = params.get(i);
                boolean trailingOptional = true;
                for (int j = i; j < params.size(); j++) {
                    trailingOptional &= params.get(j).isOptional();
                }
                String type = p.getTsType();
                collectTypes(type, viewTypes, usedViews, usedOptions);
                if (i > 0) {
                    tuple.append(", ");
                }
                if (p.isOptional() && trailingOptional) {
                    tuple.append(p.getName()).append("?: ").append(type).append(" | null");
                } else if (p.isOptional()) {
                    tuple.append(p.getName()).append(": ").append(type).append(" | null");
                } else {
                    tuple.append(p.getName()).append(": ").append(type);
                }
            }
            collectTypes(method.getResultType(), viewTypes, usedViews, usedOptions);
            body.append("  ").append(method.getName()).append(": { params: [").append(tuple)
                    .append("]; result: ").append(method.getResultType()).append(" };\n");
        }
        body.append("}\n\n");
        body.append("export type RpcMethodName = keyof RpcMethods;\n\n");

        body.append("/** Access rule and session parameter position of every RPC method. */\n");
        body.append("export const RPC_METHODS: { readonly [M in RpcMethodName]: { readonly access: RpcAccess; readonly sessionParam: number } } = {\n");
        for (RpcMethod method : methods) {
            body.append("  ").append(method.getName()).append(": { access: '").append(accessLabel(method))
                    .append("', sessionParam: ").append(method.getSessionParam()).append(" },\n");
        }
        body.append("};\n\n");

        body.append("/** Payload (\"data\") of every server event. */\n");
        body.append("export interface CallbackPayloads {\n");
        for (ClientCallbackMethod method : ClientCallbackMethod.values()) {
            String type = CALLBACK_DATA.get(method);
            collectTypes(type, viewTypes, usedViews, usedOptions);
            body.append("  ").append(method.name()).append(": ").append(type).append(";\n");
        }
        body.append("}\n\n");
        body.append("export type CallbackMethodName = keyof CallbackPayloads;\n\n");
        body.append("export type CallbackDeliveryType = 'TABLE_CHANGE' | 'UPDATE' | 'MESSAGE' | 'DIALOG' | 'CLIENT_SIDE_EVENT';\n\n");
        body.append("/** Delivery type of every server event: UPDATE events may be dropped by the server when outdated. */\n");
        body.append("export const CALLBACK_DELIVERY: { readonly [M in CallbackMethodName]: CallbackDeliveryType } = {\n");
        for (ClientCallbackMethod method : ClientCallbackMethod.values()) {
            body.append("  ").append(method.name()).append(": '").append(method.getType().name()).append("',\n");
        }
        body.append("};\n");

        StringBuilder sb = new StringBuilder();
        sb.append("// Generated by Mage.Server WebClientApiDocsTest from the server's RPC registry. Do not edit.\n");
        sb.append("// Regenerate: mvn -pl Mage.Server test -Dtest=WebClientApiDocsTest -Dxmage.updateWebApiDocs=true\n\n");
        if (!usedViews.isEmpty()) {
            sb.append("import type { ").append(String.join(", ", usedViews)).append(" } from './views';\n");
        }
        if (!usedOptions.isEmpty()) {
            sb.append("import type { ").append(String.join(", ", usedOptions)).append(" } from '../options';\n");
        }
        sb.append("\n").append(body);
        return sb.toString();
    }

    private static void collectTypes(String tsType, java.util.Set<String> viewTypes,
                                     java.util.Set<String> usedViews, java.util.Set<String> usedOptions) {
        java.util.regex.Matcher names = java.util.regex.Pattern.compile("[A-Z][A-Za-z0-9]*").matcher(tsType);
        while (names.find()) {
            String name = names.group();
            if (name.equals("UUID")) {
                continue;
            }
            if (CLIENT_OPTION_TYPES.contains(name)) {
                usedOptions.add(name);
            } else if (viewTypes.contains(name)) {
                usedViews.add(name);
            } else {
                throw new IllegalStateException("Unknown TypeScript type '" + name + "' in RPC registry; "
                        + "add its Java class to the web-client-types profile in Mage.Server/pom.xml");
            }
        }
    }
}
