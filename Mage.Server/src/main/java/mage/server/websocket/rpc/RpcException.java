package mage.server.websocket.rpc;

/**
 * Web client bridge: an error that is reported to the caller as a JSON-RPC error object.
 */
public class RpcException extends Exception {

    // standard JSON-RPC 2.0 codes
    public static final int PARSE_ERROR = -32700;
    public static final int INVALID_REQUEST = -32600;
    public static final int METHOD_NOT_FOUND = -32601;
    public static final int INVALID_PARAMS = -32602;
    public static final int INTERNAL_ERROR = -32603;

    // implementation defined codes (-32000 to -32099)
    public static final int SERVER_ERROR = -32000;
    public static final int NOT_AUTHORIZED = -32001;
    public static final int SESSION_IN_USE = -32002;
    public static final int RATE_LIMITED = -32003;
    public static final int SERVER_BUSY = -32004;
    /** a deck couldn't be imported from a deck website; the reason is in the error's data */
    public static final int DECK_IMPORT_FAILED = -32005;

    /**
     * NOT_AUTHORIZED's reason when another login as the same user took this connection's session over: the client
     * should not log back in by itself, or the two would keep taking it from each other
     */
    public static final String SIGNED_IN_ELSEWHERE = "signedInElsewhere";

    private final int code;
    /** a machine-readable reason sent as the error's data ({"reason": ...}), or null */
    private String reason;

    public RpcException(int code, String message) {
        super(message);
        this.code = code;
    }

    public RpcException(int code, String message, Throwable cause) {
        super(message, cause);
        this.code = code;
    }

    public int getCode() {
        return code;
    }

    public String getReason() {
        return reason;
    }

    public RpcException withReason(String reason) {
        this.reason = reason;
        return this;
    }

    public static RpcException invalidParams(String message) {
        return new RpcException(INVALID_PARAMS, message);
    }

    public static RpcException notAuthorized(String message) {
        return new RpcException(NOT_AUTHORIZED, message);
    }
}
