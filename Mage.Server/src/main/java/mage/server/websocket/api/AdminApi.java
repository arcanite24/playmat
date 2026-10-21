package mage.server.websocket.api;

import mage.game.Game;
import mage.game.Table;
import mage.game.match.Match;
import mage.server.Main;
import mage.server.Moderation;
import mage.server.Session;
import mage.server.UpdateCheck;
import mage.server.UpdateRunner;
import mage.server.notify.DiscordNotifier;
import mage.server.admin.AccessPolicy;
import mage.server.admin.AdminStore;
import mage.server.admin.ClientErrorGroup;
import mage.server.admin.StaffRole;
import mage.server.moderation.ReportStore;
import mage.server.websocket.BridgeMetrics;
import mage.server.websocket.rpc.RpcCall;
import mage.server.websocket.rpc.RpcException;
import mage.server.websocket.rpc.RpcMethod;
import org.apache.log4j.Logger;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Optional;
import java.util.UUID;

import static mage.server.websocket.rpc.RpcParam.Type.BOOLEAN;
import static mage.server.websocket.rpc.RpcParam.Type.INT;
import static mage.server.websocket.rpc.RpcParam.Type.STRING;
import static mage.server.websocket.rpc.RpcParam.of;
import static mage.server.websocket.rpc.RpcParam.optional;

/**
 * Web client bridge: server administration (admin sessions only) and test-mode helpers for e2e tests.
 */
final class AdminApi {

    private static final Logger logger = Logger.getLogger(AdminApi.class);

    private AdminApi() {
    }

    static List<RpcMethod> methods(ApiContext ctx) {
        List<RpcMethod> methods = new ArrayList<>(Arrays.asList(
                RpcMethod.named("adminGetUsers")
                        .session(0)
                        .params(of("sessionId", STRING))
                        .returns("UserView[]")
                        .doc("Staff (moderator): all connected users.")
                        .handler(call -> {
                            Staff.require(ctx, call, StaffRole.MODERATOR);
                            return moderation(ctx).users();
                        }),

                RpcMethod.named("adminDisconnectUser")
                        .session(0)
                        .params(of("sessionId", STRING), of("userSessionId", STRING), optional("reason", STRING))
                        .doc("Staff (moderator): sign a player's session out.")
                        .handler(call -> disconnect(ctx, call, "user.disconnect")),

                RpcMethod.named("adminEndUserSession")
                        .session(0)
                        .params(of("sessionId", STRING), of("userSessionId", STRING), optional("reason", STRING))
                        .doc("Staff (moderator): end a player's session; they leave their tables.")
                        .handler(call -> disconnect(ctx, call, "user.endsession")),

                RpcMethod.named("adminMuteUser")
                        .session(0)
                        .params(of("sessionId", STRING), of("userName", STRING), of("durationMinutes", INT), optional("reason", STRING))
                        .doc("Staff (moderator): mute a player in chats, online or not; 0 minutes lifts the mute.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.MODERATOR);
                            int minutes = minutes(call, 2);
                            AccessApi.store(() -> moderation(ctx).mute(call.string(1), minutes));
                            Staff.audit(actor, minutes == 0 ? "user.unmute" : "user.mute", call.string(1), minutes == 0 ? null : minutes + " min", reason(call, 3));
                            return true;
                        }),

                RpcMethod.named("adminLockUser")
                        .session(0)
                        .params(of("sessionId", STRING), of("userName", STRING), of("durationMinutes", INT), optional("reason", STRING))
                        .doc("Staff (admin): lock an account out, online or not; 0 minutes unlocks it.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            int minutes = minutes(call, 2);
                            refuseStaffTarget(ctx, actor, call.string(1));
                            AccessApi.store(() -> moderation(ctx).lock(call.string(1), minutes));
                            Staff.audit(actor, minutes == 0 ? "user.unlock" : "user.lock", call.string(1), minutes == 0 ? null : minutes + " min", reason(call, 3));
                            return true;
                        }),

                RpcMethod.named("adminActivateUser")
                        .session(0)
                        .params(of("sessionId", STRING), of("userName", STRING), of("active", BOOLEAN), optional("reason", STRING))
                        .doc("Staff (admin): activate or deactivate an account.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            if (!call.bool(2)) {
                                refuseStaffTarget(ctx, actor, call.string(1));
                            }
                            AccessApi.store(() -> {
                                moderation(ctx).setActive(call.string(1), call.bool(2));
                                return null;
                            });
                            Staff.audit(actor, "user.activate", call.string(1), call.bool(2) ? "active" : "inactive", reason(call, 3));
                            return true;
                        }),

                RpcMethod.named("adminToggleActivateUser")
                        .session(0)
                        .params(of("sessionId", STRING), of("userName", STRING))
                        .doc("Staff (admin): toggle an online player's active state.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            String name = call.string(1);
                            boolean active = !ctx.managers.userManager().getUserByName(name).map(mage.server.User::isActive)
                                    .orElseThrow(() -> RpcException.invalidParams("Nobody named " + name + " is online"));
                            if (!active) {
                                refuseStaffTarget(ctx, actor, call.string(1));
                            }
                            moderation(ctx).setActive(call.string(1), active);
                            Staff.audit(actor, "user.activate", call.string(1), active ? "active" : "inactive", null);
                            return true;
                        }),

                RpcMethod.named("adminTableRemove")
                        .session(0)
                        .params(of("sessionId", STRING), of("tableId", mage.server.websocket.rpc.RpcParam.Type.UUID), optional("reason", STRING))
                        .doc("Staff (admin): remove any table. False when it was gone already.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            boolean removed = moderation(ctx).removeTable(call.uuid(1));
                            if (removed) {
                                Staff.audit(actor, "table.remove", call.string(1), null, reason(call, 2));
                            }
                            return removed;
                        }),

                RpcMethod.named("adminTableWatch")
                        .session(0)
                        .params(of("sessionId", STRING), of("tableId", mage.server.websocket.rpc.RpcParam.Type.UUID))
                        .returns("string")
                        .doc("Staff (moderator): watch any running game from your own player session, whatever its spectator "
                                + "settings; the game opens through the usual watch callback. Returns the game id, or null when there "
                                + "is no running game or you play in it.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.MODERATOR);
                            String sessionId = call.string(0);
                            UUID tableId = call.uuid(1);
                            UUID gameId = AccessApi.store(() -> moderation(ctx).watchTable(sessionId, tableId));
                            if (gameId != null) {
                                Staff.audit(actor, "table.watch", tableId.toString(), "game " + gameId, null);
                            }
                            return gameId == null ? null : gameId.toString();
                        }),

                RpcMethod.named("adminSendBroadcastMessage")
                        .session(0)
                        .params(of("sessionId", STRING), of("message", STRING))
                        .doc("Staff (admin): message every connected player; a message starting with \"warn\" shows in red.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            String message = call.string(1).trim();
                            if (message.isEmpty() || message.length() > 500) {
                                throw RpcException.invalidParams("A broadcast is 1 to 500 characters");
                            }
                            moderation(ctx).broadcast(message);
                            Staff.audit(actor, "broadcast", null, message, null);
                            return true;
                        }),

                RpcMethod.named("adminServerStats")
                        .returns("ServerStats")
                        .doc("Staff (moderator): load and traffic: users, tables, games, memory, CPU, and the size of messages sent to web clients "
                                + "(with the heaviest callbacks and every one over " + BridgeMetrics.LARGE_CHARS / 1024 + " KB).")
                        .handler(call -> {
                            Staff.require(ctx, call, StaffRole.MODERATOR);
                            return ServerStats.collect(ctx.managers);
                        }),

                RpcMethod.named("adminUpdateStatus")
                        .params(optional("refresh", BOOLEAN))
                        .returns("UpdateStatus")
                        .doc("Staff (admin): whether a newer published image is out than the one this server runs, the commits "
                                + "it brings, and the image names to follow. The answer is kept for an hour; refresh looks again "
                                + "(at most once a minute). XMAGE_UPDATE_CHECK=off turns the lookups off.")
                        .handler(call -> {
                            Staff.require(ctx, call, StaffRole.ADMIN);
                            return UpdateCheck.get().status(call.optBool(0, false)).withUpdater(UpdateRunner.get().info());
                        }),

                RpcMethod.named("adminUpdateApply")
                        .params(of("version", STRING))
                        .returns("UpdateStatus")
                        .doc("Staff (owner): ask the updater (the compose service behind the Update button) to update to this "
                                + "published version (vX.Y.Z or sha-<hex>). It backs up, warns players and waits for games to end, "
                                + "restarts on the new images and goes back if they aren't healthy; updater.job follows it.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.OWNER);
                            String version = call.string(0);
                            update(UpdateRunner.UPDATE, version, actor);
                            Staff.audit(actor, "server.update", version, null, null);
                            DiscordNotifier.staff().staffNews("Update to " + version + " started by " + actor + ".");
                            return UpdateCheck.get().status(false).withUpdater(UpdateRunner.get().info());
                        }),

                RpcMethod.named("adminUpdateRollback")
                        .returns("UpdateStatus")
                        .doc("Staff (owner): ask the updater to go back to the version that ran before its last update.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.OWNER);
                            update(UpdateRunner.ROLLBACK, null, actor);
                            Staff.audit(actor, "server.rollback", null, null, null);
                            DiscordNotifier.staff().staffNews("Rollback started by " + actor + ".");
                            return UpdateCheck.get().status(false).withUpdater(UpdateRunner.get().info());
                        }),

                RpcMethod.named("adminGetReports")
                        .params(optional("openOnly", BOOLEAN))
                        .returns("PlayerReport[]")
                        .doc("Staff (moderator): player reports, newest first; only the open ones unless openOnly is false.")
                        .handler(call -> {
                            Staff.require(ctx, call, StaffRole.MODERATOR);
                            boolean openOnly = call.optBool(0, true);
                            return AccountApi.decks(() -> ReportStore.get().list(openOnly, 200));
                        }),

                RpcMethod.named("adminCloseReport")
                        .params(of("reportId", INT), of("resolution", STRING))
                        .returns("boolean")
                        .doc("Staff (moderator): close a report with a note about what was done. False when it was closed already.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.MODERATOR);
                            long id = AccountApi.longValue(call, 0);
                            boolean closed = AccountApi.decks(() -> ReportStore.get().close(id, call.string(1), System.currentTimeMillis()));
                            if (closed) {
                                Staff.audit(actor, "report.close", "report " + id, call.string(1), null);
                            }
                            return closed;
                        }),

                RpcMethod.named("adminClientErrors")
                        .params(optional("includeResolved", BOOLEAN))
                        .returns("ClientErrorGroup[]")
                        .doc("Staff (moderator): crash reports from web clients, one row per kind of error with how often it "
                                + "happened, seen most recently first; resolved ones only when asked for.")
                        .handler(call -> {
                            Staff.require(ctx, call, StaffRole.MODERATOR);
                            boolean includeResolved = call.optBool(0, false);
                            return AccessApi.store(() -> AdminStore.get().clientErrors(includeResolved, 200));
                        }),

                RpcMethod.named("adminClientErrorResolve")
                        .params(of("id", INT), optional("resolved", BOOLEAN))
                        .returns("ClientErrorGroup")
                        .doc("Staff (moderator): mark a kind of client error fixed (or, with resolved false, open again). "
                                + "It opens again by itself when it happens again.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.MODERATOR);
                            long id = AccountApi.longValue(call, 0);
                            boolean resolved = call.optBool(1, true);
                            ClientErrorGroup group = AccessApi.store(() -> AdminStore.get().resolveClientError(id, resolved, actor, System.currentTimeMillis()));
                            Staff.audit(actor, resolved ? "clientError.resolve" : "clientError.reopen", String.valueOf(id), group.message, null);
                            return group;
                        }),

                RpcMethod.named("testEndGame")
                        .session(0)
                        .params(of("sessionId", STRING), of("tableId", mage.server.websocket.rpc.RpcParam.Type.UUID))
                        .doc("Test mode only: end the running game of a table you own.")
                        .handler(call -> {
                            UUID tableId = ownedTestTable(ctx, call);
                            ctx.managers.tableManager().endGameOnGameThread(activeChildOrSelf(ctx, tableId));
                            return true;
                        }),

                RpcMethod.named("testConcedeMatch")
                        .session(0)
                        .params(of("sessionId", STRING), of("tableId", mage.server.websocket.rpc.RpcParam.Type.UUID), optional("losingPlayerIndex", INT))
                        .doc("Test mode only: make one player of a table you own concede the match.")
                        .handler(call -> testConcedeMatch(ctx, call))
        ));
        return methods;
    }


    /** a year, so a typo can't lock someone out for a century */
    static final int MAX_MINUTES = 525_600;

    private static Moderation moderation(ApiContext ctx) {
        return new Moderation(ctx.managers);
    }

    private static int minutes(RpcCall call, int index) throws RpcException {
        int minutes = call.integer(index);
        if (minutes < 0 || minutes > MAX_MINUTES) {
            throw RpcException.invalidParams("0 to " + MAX_MINUTES + " minutes");
        }
        return minutes;
    }

    private static String reason(RpcCall call, int index) throws RpcException {
        String reason = call.optString(index, "").trim();
        return reason.isEmpty() ? null : reason;
    }

    /** staff can't lock or deactivate the owner, or anyone above their own role */
    static void refuseStaffTarget(ApiContext ctx, String actor, String userName) throws RpcException {
        StaffRole target = AccessPolicy.get().roleOf(userName);
        if (target == null || Staff.CONSOLE.equals(actor)) {
            return;
        }
        StaffRole own = AccessPolicy.get().roleOf(actor);
        if (target == StaffRole.OWNER || own == null || !own.atLeast(target)) {
            throw RpcException.notAuthorized(userName + " is " + target.id() + "; that needs someone above them");
        }
    }

    private static Object disconnect(ApiContext ctx, RpcCall call, String action) throws RpcException {
        String actor = Staff.require(ctx, call, StaffRole.MODERATOR);
        String target = sessionUser(ctx, call.string(1));
        if (!moderation(ctx).disconnect(call.string(1))) {
            return false;
        }
        Staff.audit(actor, action, target, null, reason(call, 2));
        return true;
    }

    /** the name behind a session, for the audit log */
    private static String sessionUser(ApiContext ctx, String sessionId) {
        return ctx.sessions.userName(sessionId).orElse("session " + sessionId);
    }

    private static UUID ownedTestTable(ApiContext ctx, RpcCall call) throws RpcException {
        UUID tableId = call.uuid(1);
        if (!Main.isTestMode()) {
            throw RpcException.notAuthorized(call.getMethod().getName() + " is only available in server test mode");
        }
        Optional<Session> session = ctx.managers.sessionManager().getSession(call.sessionId());
        if (!session.isPresent() || !ctx.managers.tableManager().isTableOwner(tableId, session.get().getUserId())) {
            throw RpcException.notAuthorized("Only the table owner can do that");
        }
        return tableId;
    }

    private static Object testConcedeMatch(ApiContext ctx, RpcCall call) throws RpcException {
        UUID tableId = activeChildOrSelf(ctx, ownedTestTable(ctx, call));
        Match match = ctx.managers.tableManager().getMatch(tableId).orElse(null);
        Game game = match == null ? null : match.getGame();
        if (match == null || match.getPlayers().isEmpty() || game == null) {
            throw new RpcException(RpcException.SERVER_ERROR, "Table has no running game: " + tableId);
        }
        int index = Math.max(0, Math.min(call.optInteger(2, 0), match.getPlayers().size() - 1));
        UUID losingPlayerId = match.getPlayers().get(index).getPlayer().getId();
        ctx.managers.threadExecutor().getGameExecutor().execute(() -> {
            match.quitMatch(losingPlayerId);
            game.concede(losingPlayerId);
            if (game.checkIfGameIsOver() || game.hasEnded()) {
                ctx.managers.tableManager().endGame(tableId);
            } else {
                logger.warn("testConcedeMatch did not end the game of table " + tableId);
            }
        });
        return true;
    }

    /**
     * Tournament tables run their games on child tables.
     */
    private static UUID activeChildOrSelf(ApiContext ctx, UUID tableId) {
        return ctx.managers.tableManager().getTables().stream()
                .filter(table -> tableId.equals(table.getParentTableId()))
                .filter(table -> table.getMatch() != null && table.getMatch().getGame() != null)
                .findFirst()
                .map(Table::getId)
                .orElse(tableId);
    }

    /** asks the updater; what it can't take now comes back to staff as the reason */
    private static void update(String action, String version, String actor) throws RpcException {
        try {
            UpdateRunner.get().request(action, version, actor);
        } catch (IllegalArgumentException | IllegalStateException e) {
            throw RpcException.invalidParams(e.getMessage());
        }
    }
}
