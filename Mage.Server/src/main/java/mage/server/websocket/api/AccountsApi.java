package mage.server.websocket.api;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import mage.server.AccountService;
import mage.server.AuthorizedUser;
import mage.server.AuthorizedUserRepository;
import mage.server.Moderation;
import mage.server.User;
import mage.server.admin.AccessPolicy;
import mage.server.admin.AdminStore;
import mage.server.admin.AuditEntry;
import mage.server.admin.FeedbackEntry;
import mage.server.admin.GithubLink;
import mage.server.admin.StaffRole;
import mage.server.career.CareerService;
import mage.server.career.CareerStore;
import mage.server.decks.DeckStore;
import mage.server.decks.DeckSyncEntry;
import mage.server.moderation.PlayerReport;
import mage.server.moderation.ReportStore;
import mage.server.record.UserStats;
import mage.server.record.UserStatsRepository;
import mage.server.websocket.rpc.RpcCall;
import mage.server.websocket.rpc.RpcException;
import mage.server.websocket.rpc.RpcMethod;
import mage.view.UserView;
import org.apache.log4j.Logger;

import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;

import static mage.server.websocket.rpc.RpcParam.Type.INT;
import static mage.server.websocket.rpc.RpcParam.Type.STRING;
import static mage.server.websocket.rpc.RpcParam.of;
import static mage.server.websocket.rpc.RpcParam.optional;

/**
 * Web client bridge: every registered account for staff (find, look into, reset a password, export, delete), and
 * the feedback players send from the app. Email and IP addresses are shown to admins and owners only.
 */
final class AccountsApi {

    private static final Logger logger = Logger.getLogger(AccountsApi.class);

    static final int PAGE = 50;

    /** One account as the accounts list shows it. */
    public static class AccountRow {
        public String name;
        /** admins and owners only */
        public String email;
        /** the player confirmed the email with a mailed code (or GitHub vouched for it) */
        public boolean emailVerified;
        public boolean active;
        /** 0 when not locked */
        public long lockedUntil;
        /** 0 when not muted */
        public long mutedUntil;
        public long lastConnection;
        public boolean online;
        /** on the allowlist by name or email (a contributors server lets them in) */
        public boolean allowlisted;
        /** moderator, admin or owner; null for players */
        public String staffRole;
        public boolean career;
    }

    public static class AccountsPage {
        public long total;
        public long offset;
        public List<AccountRow> accounts;
    }

    /** A signed-in session of the account. */
    public static class AccountSession {
        public String sessionId;
        /** admins and owners only */
        public String host;
        public long connectedAt;
        public long lastActivity;
        public String state;
        public String playing;
    }

    /** Everything staff need to judge a report about this account. */
    public static class AccountDetail {
        public AccountRow account;
        public List<AccountSession> sessions;
        public int matches;
        public int matchesQuit;
        public int tourneys;
        /** the general rating, 0 when there is none yet */
        public int rating;
        public int decks;
        public String githubLogin;
        public List<PlayerReport> reports;
        public List<FeedbackEntry> feedback;
        /** what staff did to or about this account, newest first */
        public List<AuditEntry> audit;
    }

    private AccountsApi() {
    }

    static List<RpcMethod> methods(ApiContext ctx) {
        return Arrays.asList(
                RpcMethod.named("adminAccounts")
                        .params(optional("search", STRING), optional("offset", INT))
                        .returns("AccountsPage")
                        .doc("Staff (moderator): registered accounts whose name or email contains search, by name, "
                                + PAGE + " at a time from offset.")
                        .handler(call -> {
                            Staff.require(ctx, call, StaffRole.MODERATOR);
                            boolean admin = isAdmin(ctx, call);
                            String search = call.optString(0, "");
                            long offset = Math.max(0, call.has(1) ? AccountApi.longValue(call, 1) : 0);
                            AuthorizedUserRepository repository = AuthorizedUserRepository.getInstance();
                            AccountsPage page = new AccountsPage();
                            page.offset = offset;
                            page.total = repository.count(search);
                            page.accounts = new ArrayList<>();
                            for (AuthorizedUser account : repository.list(search, offset, PAGE)) {
                                page.accounts.add(row(ctx, account, admin));
                            }
                            return page;
                        }),

                RpcMethod.named("adminAccount")
                        .params(of("userName", STRING))
                        .returns("AccountDetail")
                        .doc("Staff (moderator): one account with its sessions, stats, reports by and about it, its feedback "
                                + "and its audit history.")
                        .handler(call -> {
                            Staff.require(ctx, call, StaffRole.MODERATOR);
                            return detail(ctx, account(call.string(0)), isAdmin(ctx, call));
                        }),

                RpcMethod.named("adminResetCode")
                        .params(of("userName", STRING), optional("reason", STRING))
                        .returns("StaffResetCode")
                        .doc("Staff (admin): a one-time password reset code for the account, shown once; it is emailed too "
                                + "when the server can send mail. The player enters it under Forgot password with the account's email.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            String name = account(call.string(0)).getName();
                            AdminApi.refuseStaffTarget(ctx, actor, name);
                            AccountService.StaffResetCode code = AccessApi.store(() -> new AccountService(ctx.managers).staffResetCode(name));
                            Staff.audit(actor, "user.resetcode", name, code.emailed ? "emailed" : "shown", reason(call, 1));
                            return code;
                        }),

                RpcMethod.named("adminAccountExport")
                        .params(of("userName", STRING))
                        .returns("string")
                        .doc("Staff (admin): everything the server keeps about the account as JSON, for the player or "
                                + "before a deletion. Passwords and sign-in tokens are left out.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            AuthorizedUser account = account(call.string(0));
                            String json = AccessApi.store(() -> export(account));
                            Staff.audit(actor, "user.export", account.getName(), null, null);
                            return json;
                        }),

                RpcMethod.named("adminDeleteAccount")
                        .params(of("userName", STRING), of("reason", STRING))
                        .returns("boolean")
                        .doc("Staff (admin): delete the account with its decks, Career, reports, stats, feedback, allowlist "
                                + "entry, staff role and supporter perks (its payments stay in the ledger, anonymized), signing its player out. Can't be undone: export first. The audit log keeps "
                                + "the record of the deletion.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            AuthorizedUser account = account(call.string(0));
                            String name = account.getName();
                            String reason = call.string(1).trim();
                            if (reason.isEmpty()) {
                                throw RpcException.invalidParams("Say why the account is deleted; it goes in the audit log");
                            }
                            if (AccessPolicy.get().isOwner(name)) {
                                throw RpcException.invalidParams(name + " is the server's owner (XMAGE_OWNER)");
                            }
                            AdminApi.refuseStaffTarget(ctx, actor, name);
                            AccessApi.store(() -> {
                                delete(ctx, name, "This account was deleted by the server's staff.");
                                return null;
                            });
                            Staff.audit(actor, "user.delete", name, null, reason);
                            return true;
                        }),

                RpcMethod.named("adminFeedback")
                        .params(optional("status", STRING))
                        .returns("FeedbackEntry[]")
                        .doc("Staff (moderator): feedback players sent, newest first, 200 at most; status new, seen or done "
                                + "narrows it.")
                        .handler(call -> {
                            Staff.require(ctx, call, StaffRole.MODERATOR);
                            return AccessApi.store(() -> AccessPolicy.get().store().feedback(call.optString(0, null), 200));
                        }),

                RpcMethod.named("adminFeedbackMark")
                        .params(of("id", INT), of("status", STRING), optional("note", STRING))
                        .returns("FeedbackEntry")
                        .doc("Staff (moderator): mark feedback new, seen or done, with an optional note (such as the issue it became).")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.MODERATOR);
                            long id = AccountApi.longValue(call, 0);
                            FeedbackEntry entry = AccessApi.store(() -> AccessPolicy.get().store().markFeedback(id, call.string(1),
                                    call.has(2) ? call.string(2) : null, actor, System.currentTimeMillis()));
                            Staff.audit(actor, "feedback." + entry.status, "feedback " + id, entry.note, null);
                            return entry;
                        })
        );
    }

    private static boolean isAdmin(ApiContext ctx, RpcCall call) {
        Staff.Member member = Staff.member(ctx, call.sessionId());
        return member != null && member.role.atLeast(StaffRole.ADMIN);
    }

    private static AuthorizedUser account(String userName) throws RpcException {
        AuthorizedUser account = AuthorizedUserRepository.getInstance().getByName(userName.trim());
        if (account == null) {
            throw RpcException.invalidParams("There is no account named " + userName.trim());
        }
        return account;
    }

    private static AccountRow row(ApiContext ctx, AuthorizedUser account, boolean admin) {
        long now = System.currentTimeMillis();
        AccountRow row = new AccountRow();
        row.name = account.getName();
        row.email = admin ? account.getEmail() : null;
        row.emailVerified = mage.server.EmailVerification.isVerified(account.getName(), account.getEmail());
        row.active = account.isActive();
        row.lockedUntil = future(account.getLockedUntil(), now);
        row.mutedUntil = future(account.getChatLockedUntil(), now);
        row.lastConnection = account.getLastConnection() == null ? 0 : account.getLastConnection().getTime();
        Optional<User> online = ctx.managers == null ? Optional.empty() : ctx.managers.userManager().getUserByName(row.name);
        row.online = online.isPresent() && online.get().isConnected();
        AccessPolicy access = AccessPolicy.get();
        try {
            row.allowlisted = access.store().isListed(row.name, account.getEmail());
        } catch (SQLException e) {
            logger.warn("Allowlist can't be read for " + row.name);
        }
        StaffRole role = access.roleOf(row.name);
        row.staffRole = role == null ? null : role.id();
        try {
            row.career = CareerStore.get().profile(row.name) != null;
        } catch (SQLException | RuntimeException e) {
            row.career = false; // no Career store on this server
        }
        return row;
    }

    private static long future(java.util.Date date, long now) {
        return date == null || date.getTime() <= now ? 0 : date.getTime();
    }

    private static AccountDetail detail(ApiContext ctx, AuthorizedUser account, boolean admin) throws RpcException {
        AccountDetail detail = new AccountDetail();
        detail.account = row(ctx, account, admin);
        String name = account.getName();
        detail.sessions = new ArrayList<>();
        for (UserView view : new Moderation(ctx.managers).users()) {
            if (name.equalsIgnoreCase(view.getUserName())) {
                AccountSession session = new AccountSession();
                session.sessionId = view.getSessionId();
                session.host = admin ? view.getHost() : null;
                session.connectedAt = view.getTimeConnected() == null ? 0 : view.getTimeConnected().getTime();
                session.lastActivity = view.getLastActivity() == null ? 0 : view.getLastActivity().getTime();
                session.state = view.getUserState();
                session.playing = view.getGameInfo();
                detail.sessions.add(session);
            }
        }
        UserStats stats = UserStatsRepository.instance.getUser(name);
        if (stats != null && stats.getProto() != null) {
            detail.matches = stats.getProto().getMatches();
            detail.matchesQuit = stats.getProto().getMatchesQuit();
            detail.tourneys = stats.getProto().getTourneys();
            if (stats.getProto().hasGeneralGlickoRating()) {
                detail.rating = (int) Math.round(stats.getProto().getGeneralGlickoRating().getRating());
            }
        }
        AdminStore store = AccessPolicy.get().store();
        return AccessApi.store(() -> {
            detail.decks = DeckStore.get().export(name).size();
            GithubLink link = store.githubLinkOf(name);
            detail.githubLogin = link == null ? null : link.login;
            detail.reports = ReportStore.get().involving(name, 50);
            detail.feedback = store.feedbackFrom(name, 50);
            detail.audit = store.auditLog(null, null, name, 0, 50);
            return detail;
        });
    }

    /** Everything kept about the account as JSON, passwords and sign-in tokens left out: for staff, and for the player. */
    static String export(AuthorizedUser account) throws SQLException {
        String name = account.getName();
        Map<String, Object> out = new LinkedHashMap<>();
        out.put("exportedAt", System.currentTimeMillis());
        Map<String, Object> fields = new LinkedHashMap<>();
        fields.put("name", name);
        fields.put("email", account.getEmail());
        fields.put("active", account.isActive());
        fields.put("lockedUntil", account.getLockedUntil());
        fields.put("mutedUntil", account.getChatLockedUntil());
        fields.put("lastConnection", account.getLastConnection());
        fields.put("emailVerified", mage.server.EmailVerification.isVerified(name, account.getEmail()));
        out.put("account", fields);
        List<DeckSyncEntry> decks = DeckStore.get().export(name);
        out.put("decks", decks);
        String career = null;
        try {
            career = CareerService.get().export(name);
        } catch (CareerService.CareerException | RuntimeException e) {
            // no Career
        }
        out.put("career", career);
        out.put("reports", ReportStore.get().involving(name, 500));
        out.put("feedback", AccessPolicy.get().store().feedbackFrom(name, 500));
        UserStats stats = UserStatsRepository.instance.getUser(name);
        out.put("stats", stats == null || stats.getProto() == null ? null : stats.getProto().toString());
        GithubLink link = AccessPolicy.get().store().githubLinkOf(name);
        out.put("githubLogin", link == null ? null : link.login);
        // support for the server: the code, payments (without the providers' payloads), perks and the look
        out.put("support", SupportApi.export(name));
        Gson gson = new GsonBuilder().setPrettyPrinting().serializeNulls().create();
        return gson.toJson(out);
    }

    /**
     * Deletes the account with everything kept about it, then signs its player out with {@code message}, after a
     * moment so the caller's answer (and the message) arrive first. The audit log keeps its record.
     */
    static void delete(ApiContext ctx, String name, String message) throws SQLException {
        DeckStore.get().deleteAll(name);
        try {
            CareerService.get().delete(name);
        } catch (RuntimeException e) {
            logger.warn("No Career store to delete " + name + " from");
        }
        try {
            mage.server.solo.SoloStore.get().deleteUser(name);
        } catch (java.sql.SQLException | RuntimeException e) {
            logger.warn("Saved games and scores of " + name + " couldn't be deleted", e);
        }
        ReportStore.get().forget(name);
        try {
            // the ledger keeps the amounts, for the months' totals, without anything that names the payer
            mage.server.support.Support.get().forget(name);
        } catch (IllegalStateException e) {
            logger.warn("No support store to forget " + name + " in");
        }
        UserStatsRepository.instance.remove(name);
        AccessPolicy.get().store().forgetAccount(name);
        AuthorizedUserRepository.getInstance().remove(name);
        logger.info("Account deleted: " + name);
        if (ctx.managers != null) {
            ctx.managers.userManager().getUserByName(name).ifPresent(user -> {
                user.showUserMessage("Account deleted", message);
                String sessionId = user.getSessionId();
                ctx.managers.threadExecutor().getCallExecutor().execute(() -> {
                    try {
                        Thread.sleep(1000);
                    } catch (InterruptedException e) {
                        Thread.currentThread().interrupt();
                    }
                    ctx.managers.sessionManager().disconnect(sessionId, mage.server.DisconnectReason.DisconnectedByAdmin, true);
                });
            });
        }
    }

    private static String reason(RpcCall call, int index) throws RpcException {
        String reason = call.optString(index, "").trim();
        return reason.isEmpty() ? null : reason;
    }
}
