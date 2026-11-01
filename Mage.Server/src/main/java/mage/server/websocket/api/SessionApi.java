package mage.server.websocket.api;

import com.google.gson.JsonElement;
import mage.server.AccountService;
import mage.server.Session;
import mage.players.net.UserData;
import mage.server.DisconnectReason;
import mage.server.Main;
import mage.server.User;
import mage.server.websocket.rpc.RpcCall;
import mage.server.websocket.rpc.RpcException;
import mage.server.websocket.rpc.RpcMethod;
import mage.utils.MageVersion;
import org.apache.log4j.Logger;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

import static mage.server.websocket.rpc.RpcParam.Type.ARRAY;
import static mage.server.websocket.rpc.RpcParam.Type.BOOLEAN;
import static mage.server.websocket.rpc.RpcParam.Type.STRING;
import static mage.server.websocket.rpc.RpcParam.object;
import static mage.server.websocket.rpc.RpcParam.of;
import static mage.server.websocket.rpc.RpcParam.optional;

/**
 * Web client bridge: login, registration, user settings and connection keep-alive.
 */
final class SessionApi {

    private static final Logger logger = Logger.getLogger(SessionApi.class);

    private SessionApi() {
    }

    static List<RpcMethod> methods(ApiContext ctx) {
        // the web client is released together with the server, so it always reports the server version
        MageVersion serverVersion = new MageVersion(Main.class);

        RpcMethod disconnect = RpcMethod.named("disconnectSession")
                .session(0)
                .params(of("sessionId", STRING), optional("keepGames", BOOLEAN))
                .doc("Log out. With keepGames the user's active tables survive for a later reconnect.")
                .handler(call -> {
                    DisconnectReason reason = call.optBool(1, false)
                            ? DisconnectReason.DisconnectedByUserButKeepTables
                            : DisconnectReason.DisconnectedByUser;
                    ctx.sessions.disconnect(call.sessionId(), reason);
                    call.getConnection().bindSession(null);
                    return true;
                });

        return Arrays.asList(
                RpcMethod.named("ping")
                        .optionalSession(0)
                        .params(optional("sessionId", STRING), optional("pingInfo", STRING))
                        .doc("Keep-alive. Extends the logged in user's session; always succeeds before login.")
                        .handler(call -> !call.has(0) || ctx.server.ping(call.string(0), call.optString(1, "websocket"))),

                RpcMethod.named("setCapabilities")
                        .publicAccess()
                        .params(of("capabilities", ARRAY).typed("string[]"))
                        .returns("string[]")
                        .doc("Turn on optional protocol features for this connection; returns the ones now on. "
                                + "\"stateDiffs\": game states are sent as patches against the previous one (see State patches). "
                                + "Call it right after connecting, before logging in; a new connection starts without them.")
                        .handler(call -> {
                            List<String> requested = new ArrayList<>();
                            for (JsonElement capability : call.jsonArray(0)) {
                                if (capability.isJsonPrimitive()) {
                                    requested.add(capability.getAsString());
                                }
                            }
                            return call.getConnection().setCapabilities(requested);
                        }),

                RpcMethod.named("connectUser")
                        .establishes(2)
                        .params(of("userName", STRING), of("password", STRING), of("sessionId", STRING),
                                optional("restoreSessionId", STRING), optional("clientVersion", STRING), optional("userIdStr", STRING),
                                optional("rememberToken", STRING))
                        .doc("Log in. Creates the connection's session; restoreSessionId reattaches a user that lost its connection. "
                                + "rememberToken (from authRememberLogin) stands in for an account's password. Logging in as a user "
                                + "that is connected elsewhere takes it over: the other connection is closed with code 4001. "
                                + "On a connection that came through the server's single sign-on (serverInfo's signOn), only that "
                                + "name signs in, without a password; its account is linked or created on the first sign-in.")
                        .handler(call -> {
                            String signedOn = call.getConnection().getSignedOnUser();
                            String rememberToken = call.optString(6, "");
                            if (signedOn == null && !rememberToken.isEmpty()) {
                                ctx.sessions.trustRememberedLogin(call.string(2), call.string(0), rememberToken);
                            }
                            boolean ok = signedOn != null ? connectSignedOn(ctx, call, signedOn, serverVersion)
                                    : connectUser(ctx, call, serverVersion);
                            // the name the server knows the user by: a later login as it elsewhere ends this session
                            call.getConnection().setUserName(ok ? ctx.sessions.userName(call.sessionId()).orElse(null) : null);
                            return ok;
                        }),

                RpcMethod.named("connectAdmin")
                        .establishes(1)
                        .params(of("password", STRING), of("sessionId", STRING))
                        .doc("Log in as server admin. Disabled unless the server was started with a non-empty admin password "
                                + "(-adminPassword=... or -Dxmage.adminPassword=...). After a wrong password the caller's IP is "
                                + "locked out for 1, 2, 4... seconds (at most 5 minutes).")
                        .handler(call -> connectAdmin(ctx, call, serverVersion)),

                RpcMethod.named("sessionGetRestoreToken")
                        .session(0)
                        .params(of("sessionId", STRING))
                        .returns("string")
                        .doc("Token that reattaches this user (and its tables) from a later connection: pass it as "
                                + "connectUser's restoreSessionId. It changes on every login, so fetch it again after each one.")
                        .handler(call -> ctx.sessions.restoreToken(call.sessionId())
                                .orElseThrow(() -> RpcException.notAuthorized("Log in before asking for a restore token"))),

                RpcMethod.named("sessionUserName")
                        .session(0)
                        .params(of("sessionId", STRING))
                        .returns("string")
                        .doc("The signed in user's name as the server knows it: an account's own spelling, which can differ "
                                + "in case from the name typed into connectUser.")
                        .handler(call -> ctx.managers.sessionManager().getSession(call.sessionId())
                                .flatMap(session -> ctx.managers.userManager().getUser(session.getUserId()))
                                .map(User::getName)
                                .orElseThrow(() -> RpcException.notAuthorized("Log in first"))),

                RpcMethod.named("authRememberLogin")
                        .session(0)
                        .params(of("sessionId", STRING))
                        .returns("string")
                        .doc("\"Keep me signed in\": a token that signs this account in from this device without the password "
                                + "(connectUser's rememberToken) until it goes unused for 90 days, the password changes or "
                                + "authForgetLogin. Fails on servers without accounts.")
                        .handler(call -> ctx.sessions.rememberLogin(call.sessionId())
                                .orElseThrow(() -> RpcException.invalidParams("This server has no accounts to remember"))),

                RpcMethod.named("authForgetLogin")
                        .publicAccess()
                        .params(of("userName", STRING), of("rememberToken", STRING))
                        .doc("Signs a remembered device out: its token stops working. Always succeeds.")
                        .handler(call -> {
                            ctx.sessions.forgetLogin(call.string(0), call.string(1));
                            return true;
                        }),

                RpcMethod.named("authRegister")
                        .establishes(0)
                        .params(of("sessionId", STRING), of("userName", STRING), of("password", STRING), of("email", STRING),
                                optional("inviteCode", STRING), optional("turnstileToken", STRING))
                        .doc("Register a new account (servers with authentication enabled). With a password it becomes the "
                                + "account's password; without one the server emails a generated password. On a contributors-only "
                                + "server the name or email must be on the allowlist, or inviteCode an invite that still works "
                                + "(which puts the account on the list). " + RegistrationLimiter.PER_HOUR + " accounts an hour and "
                                + RegistrationLimiter.PER_DAY + " a day from one address. When serverInfo has a turnstileSiteKey, "
                                + "turnstileToken is the Turnstile widget's token. Fails with the reason.")
                        .handler(call -> {
                            Session session = ctx.managers.sessionManager().getSession(call.string(0))
                                    .orElseThrow(() -> RpcException.notAuthorized("No session"));
                            String address = call.getConnection().getRemoteHost();
                            String problem = ctx.registrations.take("register:" + address);
                            if (problem != null) {
                                logger.warn("Registration refused, too many from " + address);
                                throw new RpcException(RpcException.RATE_LIMITED, problem);
                            }
                            problem = ctx.turnstile.check(call.optString(5, null), address);
                            if (problem == null) {
                                problem = session.registerUser(call.string(1), call.string(2), call.string(3), call.optString(4, null));
                            }
                            if (problem != null) {
                                ctx.registrations.refund("register:" + address);
                                throw RpcException.invalidParams(problem);
                            }
                            return true;
                        }),

                RpcMethod.named("authSendTokenToEmail")
                        .establishes(0)
                        .params(of("sessionId", STRING), of("email", STRING))
                        .doc("Email a password reset code to an account's address. Succeeds whether or not an account has the "
                                + "address, so addresses can't be probed; fails only when the server can't send mail.")
                        .handler(call -> {
                            String problem = new AccountService(ctx.managers).sendResetCode(call.string(1));
                            if (problem != null && !AccountService.NO_SUCH_EMAIL.equals(problem)) {
                                throw RpcException.invalidParams(problem);
                            }
                            return true;
                        }),

                RpcMethod.named("authResetPassword")
                        .establishes(0)
                        .params(of("sessionId", STRING), of("email", STRING), of("authToken", STRING), of("password", STRING))
                        .doc("Set a new password with an emailed code (valid 30 minutes, 5 tries). Fails with the reason.")
                        .handler(call -> {
                            String problem = new AccountService(ctx.managers).resetPassword(call.string(1), call.string(2), call.string(3));
                            if (problem != null) {
                                throw RpcException.invalidParams(problem);
                            }
                            return true;
                        }),

                disconnect,
                disconnect.alias("playerLogout"),

                RpcMethod.named("connectSetUserData")
                        .session(1)
                        .params(optional("userName", STRING), of("sessionId", STRING), object("userData", "UserData"),
                                optional("clientVersion", STRING), optional("userIdStr", STRING))
                        .doc("Store user preferences (avatar, flag, auto-pass settings). userName is ignored; the session identifies the user.")
                        .handler(call -> ctx.server.connectSetUserData(call.string(1), call.object(2, UserData.class),
                                call.optString(3, ""), call.optString(4, ""))),

                RpcMethod.named("sendFeedback")
                        .session(0)
                        .params(of("sessionId", STRING), of("userName", STRING), of("title", STRING), of("type", STRING),
                                of("message", STRING), optional("email", STRING))
                        .doc("Send feedback to the server's staff (the admin Feedback inbox). type is bug, idea or other; title "
                                + "may be empty; email carries where the app was and its version. " + FEEDBACK_PER_HOUR
                                + " an hour per player.")
                        .handler(call -> {
                            String userName = ctx.sessions.userName(call.sessionId()).orElse(call.string(1));
                            if (!AccessApi.takeSlot("feedback:" + userName.toLowerCase(java.util.Locale.ROOT), System.currentTimeMillis(), FEEDBACK_PER_HOUR)) {
                                throw new RpcException(RpcException.RATE_LIMITED, "Thanks! That's plenty for now. Try again in an hour.");
                            }
                            AccessApi.store(() -> mage.server.admin.AccessPolicy.get().store().addFeedback(userName, call.string(3),
                                    call.string(2), call.string(4), call.optString(5, ""), System.currentTimeMillis()));
                            return true;
                        })
        );
    }

    /**
     * Password sign-ins are throttled per address and per account name: after a few failures each further try waits
     * longer, so a closed server doesn't invite password guessing. Remembered devices and restored sessions aren't
     * guesses and aren't counted.
     */
    static final int FEEDBACK_PER_HOUR = 5;

    private static boolean connectUser(ApiContext ctx, RpcCall call, MageVersion serverVersion) throws Exception {
        String password = call.string(1);
        if (password.isEmpty()) {
            return ctx.server.connectUser(call.string(0), password, call.string(2), call.optString(3, ""), serverVersion, call.optString(5, ""));
        }
        String ipKey = "ip:" + call.getConnection().getRemoteHost();
        String nameKey = "name:" + call.string(0).trim().toLowerCase(java.util.Locale.ROOT);
        long lockMillis = Math.max(ctx.playerLoginThrottle.tryBegin(ipKey), ctx.playerLoginThrottle.tryBegin(nameKey));
        if (lockMillis > 0) {
            ctx.sessions.disconnect(call.sessionId(), DisconnectReason.LostConnection);
            call.getConnection().bindSession(null);
            logger.warn("Sign-in throttled: " + call.string(0) + " from " + call.getConnection().getRemoteHost());
            throw new RpcException(RpcException.RATE_LIMITED,
                    "Too many failed sign-ins. Try again in " + ((lockMillis + 999) / 1000) + " s");
        }
        boolean ok = ctx.server.connectUser(call.string(0), password, call.string(2), call.optString(3, ""), serverVersion, call.optString(5, ""));
        if (ok) {
            ctx.playerLoginThrottle.recordSuccess(ipKey);
            ctx.playerLoginThrottle.recordSuccess(nameKey);
        } else {
            ctx.playerLoginThrottle.recordFailure(ipKey);
            ctx.playerLoginThrottle.recordFailure(nameKey);
        }
        return ok;
    }

    /**
     * The sign-on proxy in front named this connection's user: that account signs in without a password (and is made on
     * its first sign-in), and no other name does on this connection.
     */
    private static boolean connectSignedOn(ApiContext ctx, RpcCall call, String signedOn, MageVersion serverVersion) throws Exception {
        String account;
        try {
            if (!signedOn.equalsIgnoreCase(call.string(0).trim())) {
                throw new IllegalArgumentException("You're signed on as " + signedOn + ". Sign out of the single sign-on to "
                        + "use another account.");
            }
            account = ctx.sessions.trustSignedOnLogin(call.sessionId(), signedOn);
        } catch (IllegalArgumentException e) {
            // the dispatcher already opened a session for this login: do not leave it behind
            ctx.sessions.disconnect(call.sessionId(), DisconnectReason.LostConnection);
            call.getConnection().bindSession(null);
            logger.info("Single sign-on refused for " + signedOn + " from " + call.getConnection().getRemoteHost() + ": " + e.getMessage());
            throw RpcException.notAuthorized(e.getMessage());
        }
        return ctx.server.connectUser(account, "", call.string(2), call.optString(3, ""), serverVersion, call.optString(5, ""));
    }

    private static boolean connectAdmin(ApiContext ctx, RpcCall call, MageVersion serverVersion) throws Exception {
        String ip = call.getConnection().getRemoteHost();
        String password = call.string(0);
        RpcException rejection = null;
        long lockMillis = ctx.adminPassword.isEmpty() ? 0 : ctx.adminLoginThrottle.tryBegin(ip);
        if (ctx.adminPassword.isEmpty()) {
            rejection = RpcException.notAuthorized("Admin login is disabled on this server");
        } else if (lockMillis > 0) {
            rejection = new RpcException(RpcException.RATE_LIMITED,
                    "Too many failed admin logins, try again in " + ((lockMillis + 999) / 1000) + " s");
        } else if (!MessageDigest.isEqual(password.getBytes(StandardCharsets.UTF_8),
                ctx.adminPassword.getBytes(StandardCharsets.UTF_8))) {
            ctx.adminLoginThrottle.recordFailure(ip);
            logger.warn("Failed admin login over WebSocket from " + ip);
            rejection = RpcException.notAuthorized("Wrong admin password");
        }
        if (rejection != null) {
            // the dispatcher already opened a session for this login: do not leave it behind
            ctx.sessions.disconnect(call.sessionId(), DisconnectReason.LostConnection);
            call.getConnection().bindSession(null);
            throw rejection;
        }
        ctx.adminLoginThrottle.recordSuccess(ip);
        return ctx.server.connectAdmin(password, call.sessionId(), serverVersion);
    }
}
