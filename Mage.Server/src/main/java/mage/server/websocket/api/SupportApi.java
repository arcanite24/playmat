package mage.server.websocket.api;

import com.google.gson.JsonElement;
import mage.server.AuthorizedUser;
import mage.server.admin.StaffRole;
import mage.server.support.Payment;
import mage.server.support.PlayerLook;
import mage.server.support.Support;
import mage.server.websocket.rpc.RpcException;
import mage.server.websocket.rpc.RpcMethod;

import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

import static mage.server.websocket.rpc.RpcParam.Type.ARRAY;
import static mage.server.websocket.rpc.RpcParam.Type.BOOLEAN;
import static mage.server.websocket.rpc.RpcParam.Type.INT;
import static mage.server.websocket.rpc.RpcParam.Type.STRING;
import static mage.server.websocket.rpc.RpcParam.object;
import static mage.server.websocket.rpc.RpcParam.of;
import static mage.server.websocket.rpc.RpcParam.optional;

/**
 * Web client bridge: support for the server. The month's goal for everyone, the signed-in player's code, perks and
 * thank-you, the look players show their opponents, and the ledger for staff (admins and the owner; moderators don't
 * see payments). Support never unlocks gameplay: everything it gives is cosmetic.
 */
final class SupportApi {

    /** names asked about at once, at most */
    static final int MAX_LOOKS = 16;

    private SupportApi() {
    }

    static List<RpcMethod> methods(ApiContext ctx) {
        return Arrays.asList(
                RpcMethod.named("supportStatus")
                        .publicAccess()
                        .returns("SupportStatus")
                        .doc("The month's goal (calendar month, UTC, US dollars), how far it is, how many supported, what it "
                                + "pays for and the public supporters wall. Signed in to an account: your support code and perks "
                                + "too. Callable before login.")
                        .handler(call -> {
                            String account = null;
                            if (call.sessionId() != null && AccessApi.accounts(ctx)) {
                                account = ctx.sessions.userName(call.sessionId()).orElse(null);
                            }
                            String viewer = account;
                            return support(() -> Support.get().status(viewer, System.currentTimeMillis()));
                        }),

                RpcMethod.named("supportPerks")
                        .returns("SupportYou")
                        .doc("The signed-in account's support code, supporter status and the perks it owns for good, and "
                                + "whether a thank-you waits. Only on servers with accounts.")
                        .handler(call -> {
                            AuthorizedUser account = MyAccountApi.account(ctx, call);
                            return support(() -> Support.get().you(account.getName(), System.currentTimeMillis()));
                        }),

                RpcMethod.named("supportNoticeSeen")
                        .returns("boolean")
                        .doc("The player saw the thank-you for their support; false when none was waiting.")
                        .handler(call -> {
                            AuthorizedUser account = MyAccountApi.account(ctx, call);
                            return support(() -> Support.get().noticeSeen(account.getName()));
                        }),

                RpcMethod.named("supportSetWall")
                        .params(of("show", BOOLEAN))
                        .returns("SupportYou")
                        .doc("Whether the signed-in account's name appears on the supporters wall (off until chosen).")
                        .handler(call -> {
                            AuthorizedUser account = MyAccountApi.account(ctx, call);
                            boolean show = call.bool(0);
                            return support(() -> {
                                Support.get().setWall(account.getName(), show);
                                return Support.get().you(account.getName(), System.currentTimeMillis());
                            });
                        }),

                RpcMethod.named("profileSetLook")
                        .params(object("look", "PlayerLook"))
                        .returns("PlayerLook")
                        .doc("The look the signed-in account plays with (sleeve, cloth, avatar, frame, title, flair). The server "
                                + "keeps what the account owns (free items, Career unlocks, supporter perks) and drops the rest; "
                                + "returns what it kept.")
                        .handler(call -> {
                            AuthorizedUser account = MyAccountApi.account(ctx, call);
                            PlayerLook look = call.object(0, PlayerLook.class);
                            return support(() -> Support.get().setLook(account.getName(), look, System.currentTimeMillis()));
                        }),

                RpcMethod.named("profileLooks")
                        .params(of("names", ARRAY).typed("string[]"))
                        .returns("PlayerLook[]")
                        .doc("The looks of these players (" + MAX_LOOKS + " at most), for a match's opponents or the lobby; "
                                + "players without one are left out.")
                        .handler(call -> {
                            ctx.sessions.userName(call.sessionId()).orElseThrow(() -> RpcException.notAuthorized("Sign in first"));
                            JsonElement raw = call.raw(0);
                            if (raw == null || !raw.isJsonArray()) {
                                throw RpcException.invalidParams("names is a list of player names");
                            }
                            List<String> names = new ArrayList<>();
                            for (JsonElement name : raw.getAsJsonArray()) {
                                if (name.isJsonPrimitive() && names.size() < MAX_LOOKS) {
                                    names.add(name.getAsString());
                                }
                            }
                            return support(() -> Support.get().looks(names));
                        }),

                RpcMethod.named("adminSupportLedger")
                        .params(optional("month", STRING), optional("unmatched", BOOLEAN), optional("provider", STRING))
                        .returns("SupportLedger")
                        .doc("Staff (admin): a month of support payments (2026-10; empty: this month) with its total against "
                                + "the goal, or every unmatched payment; provider kofi or github narrows it.")
                        .handler(call -> {
                            Staff.require(ctx, call, StaffRole.ADMIN);
                            String month = call.optString(0, "");
                            boolean unmatched = call.optBool(1, false);
                            String provider = call.optString(2, "");
                            return support(() -> Support.get().ledger(month, unmatched, provider, System.currentTimeMillis()));
                        }),

                RpcMethod.named("adminSupportLink")
                        .params(of("paymentId", INT), of("userName", STRING))
                        .returns("Payment")
                        .doc("Staff (admin): link a payment to an account; it unlocks the supporter perks as an automatic link would.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            long id = AccountApi.longValue(call, 0);
                            String userName = call.string(1);
                            Payment payment = support(() -> Support.get().link(id, userName, System.currentTimeMillis()));
                            Staff.audit(actor, "support.link", payment.account, "payment " + id, null);
                            return payment;
                        }),

                RpcMethod.named("adminSupportRevoke")
                        .params(of("paymentId", INT), of("reason", STRING), optional("removePerks", BOOLEAN))
                        .returns("Payment")
                        .doc("Staff (admin): revoke a refunded or charged-back payment; it stops counting. With removePerks the "
                                + "perks it granted go too, unless another payment of the account still stands for them.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            long id = AccountApi.longValue(call, 0);
                            String reason = call.string(1).trim();
                            if (reason.isEmpty()) {
                                throw RpcException.invalidParams("Say why the payment is revoked; it goes in the audit log");
                            }
                            boolean removePerks = call.optBool(2, false);
                            Payment payment = support(() -> Support.get().revoke(id, reason, removePerks, System.currentTimeMillis()));
                            Staff.audit(actor, "support.revoke", payment.account, "payment " + id + (removePerks ? ", perks removed" : ""), reason);
                            return payment;
                        }),

                RpcMethod.named("adminSupportGrant")
                        .params(of("userName", STRING), of("reason", STRING))
                        .returns("SupportYou")
                        .doc("Staff (admin): give an account the supporter perks by hand (support that came some other way).")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            String reason = call.string(1).trim();
                            if (reason.isEmpty()) {
                                throw RpcException.invalidParams("Say why; it goes in the audit log");
                            }
                            String name = call.string(0).trim();
                            if (name.isEmpty()) {
                                throw RpcException.invalidParams("Which account?");
                            }
                            Support.SupportYou you = support(() -> {
                                Support.get().grant(name, System.currentTimeMillis());
                                return Support.get().you(name, System.currentTimeMillis());
                            });
                            Staff.audit(actor, "support.grant", name, null, reason);
                            return you;
                        })
        );
    }

    interface SupportCall<T> {
        T run() throws SQLException;
    }

    /** Store errors as RPC errors: bad input is the caller's, anything else the server's. */
    static <T> T support(SupportCall<T> work) throws RpcException {
        try {
            return work.run();
        } catch (IllegalArgumentException e) {
            throw RpcException.invalidParams(e.getMessage());
        } catch (IllegalStateException | SQLException e) {
            throw new RpcException(RpcException.SERVER_ERROR, e.getMessage(), e);
        }
    }

    /** For the account's export. */
    static Object export(String account) {
        try {
            return Support.get().export(account);
        } catch (SQLException | IllegalStateException e) {
            return null;
        }
    }
}
