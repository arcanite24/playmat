package mage.server.websocket.rpc;

import com.google.gson.JsonArray;
import com.google.gson.JsonNull;
import com.google.gson.JsonPrimitive;
import mage.server.DisconnectReason;

import java.util.ArrayList;
import java.util.Collection;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;

/**
 * Web client bridge: method registry plus access control.
 * <p>
 * Sessions are issued by the server, one per connection. Clients still send a sessionId in the documented
 * position for wire compatibility, but the dispatcher always replaces it with the connection's own session,
 * so a client can never act on (or take over) another session by sending its id.
 */
public final class RpcDispatcher {

    private final Map<String, RpcMethod> methods = new LinkedHashMap<>();
    private final RpcSessions sessions;

    public RpcDispatcher(RpcSessions sessions) {
        this.sessions = sessions;
    }

    public void register(RpcMethod method) {
        if (methods.containsKey(method.getName())) {
            throw new IllegalStateException("Duplicate RPC method: " + method.getName());
        }
        methods.put(method.getName(), method);
    }

    public void registerAll(Collection<RpcMethod> list) {
        for (RpcMethod method : list) {
            register(method);
        }
    }

    public RpcMethod find(String name) {
        return methods.get(name);
    }

    public List<RpcMethod> getMethods() {
        return Collections.unmodifiableList(new ArrayList<>(methods.values()));
    }

    public RpcSessions getSessions() {
        return sessions;
    }

    public Object dispatch(String methodName, JsonArray rawParams, RpcConnection connection) throws Exception {
        RpcMethod method = methods.get(methodName);
        if (method == null) {
            throw new RpcException(RpcException.METHOD_NOT_FOUND, "Unknown method: " + methodName);
        }

        JsonArray params = rawParams == null ? new JsonArray() : rawParams.deepCopy();
        if (params.size() < method.getRequiredParamCount()) {
            throw RpcException.invalidParams(String.format("%s expects at least %d parameters, got %d",
                    methodName, method.getRequiredParamCount(), params.size()));
        }

        switch (method.getAccess()) {
            case ESTABLISH:
                establishSession(method, params, connection);
                break;
            case SESSION:
                requireSession(method, params, connection);
                break;
            case PUBLIC:
            default:
                if (method.getSessionParam() >= 0) {
                    // optional session, e.g. ping: use the bound one, never a client supplied one
                    setParam(params, method.getSessionParam(), connection.getSessionId());
                }
                break;
        }

        return method.getHandler().handle(new RpcCall(method, params, connection));
    }

    private void requireSession(RpcMethod method, JsonArray params, RpcConnection connection) throws RpcException {
        String sessionId = connection.getSessionId();
        if (sessionId == null || !sessions.exists(sessionId)) {
            if (sessionId != null && sessions.isSignedInElsewhere(connection.getUserName(), sessionId)) {
                throw RpcException.notAuthorized("Not connected: you signed in somewhere else")
                        .withReason(RpcException.SIGNED_IN_ELSEWHERE);
            }
            throw RpcException.notAuthorized("Not connected: log in before calling " + method.getName());
        }
        if (method.getSessionParam() >= 0) {
            setParam(params, method.getSessionParam(), sessionId);
        }
    }

    private void establishSession(RpcMethod method, JsonArray params, RpcConnection connection) {
        String previous = connection.getSessionId();
        if (previous != null && sessions.exists(previous)) {
            // same connection logs in again (e.g. register then login): drop the old session, keep its tables
            sessions.disconnect(previous, DisconnectReason.LostConnection);
        }

        String sessionId = UUID.randomUUID().toString();
        sessions.create(sessionId, connection.createCallbackHandler(), connection.getRemoteHost());
        connection.bindSession(sessionId);
        setParam(params, method.getSessionParam(), sessionId);
    }

    private static void setParam(JsonArray params, int index, String value) {
        while (params.size() <= index) {
            params.add(JsonNull.INSTANCE);
        }
        params.set(index, value == null ? JsonNull.INSTANCE : new JsonPrimitive(value));
    }
}
