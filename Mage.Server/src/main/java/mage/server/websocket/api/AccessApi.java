package mage.server.websocket.api;

import mage.server.AuthorizedUser;
import mage.server.AuthorizedUserRepository;
import mage.server.DisconnectReason;
import mage.server.Mailer;
import mage.server.Session;
import mage.server.User;
import mage.server.admin.AccessPolicy;
import mage.server.admin.StaffRole;
import mage.server.admin.AccessRequest;
import mage.server.admin.AdminStore;
import mage.server.admin.AllowEntry;
import mage.server.admin.AuditEntry;
import mage.server.admin.GithubAuth;
import mage.server.admin.GithubSignIn;
import mage.server.admin.Invite;
import mage.server.notify.DiscordNotifier;
import mage.server.websocket.rpc.RpcCall;
import mage.server.websocket.rpc.RpcException;
import mage.server.websocket.rpc.RpcMethod;
import org.apache.log4j.Logger;

import java.io.IOException;
import java.sql.SQLException;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Deque;
import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.function.BiPredicate;

import static mage.server.websocket.rpc.RpcParam.Type.BOOLEAN;
import static mage.server.websocket.rpc.RpcParam.Type.INT;
import static mage.server.websocket.rpc.RpcParam.Type.STRING;
import static mage.server.websocket.rpc.RpcParam.of;
import static mage.server.websocket.rpc.RpcParam.optional;

/**
 * Web client bridge: who may play here. The access mode (open or allowlist), the allowlist, invite links, access
 * requests, the server notice, and the audit log every staff action writes to.
 */
final class AccessApi {

    private static final Logger logger = Logger.getLogger(AccessApi.class);

    private static final long HOUR = 3_600_000L;
    private static final Map<String, Deque<Long>> recentRequests = new HashMap<>();

    private AccessApi() {
    }

    /**
     * Everything the Access page shows.
     */
    public static class AccessOverview {
        /** open or allowlist */
        public String mode;
        /** the server has accounts; an allowlist needs them */
        public boolean accounts;
        public boolean mail;
        public String notice;
        /** the account that is always allowed (xmage.owner), or empty */
        public String owner;
        public List<AllowEntry> allowlist;
        public List<Invite> invites;
        /** open ones first, then the latest decided */
        public List<AccessRequest> requests;
        /** players can sign in with GitHub */
        public boolean github;
        /** the repository whose people are let in (owner/name), or empty */
        public String githubRepo;
        public int githubPeople;
        public long githubSyncedAt;
        /** why the last fetch of the repository's people failed, or null */
        public String githubProblem;
    }

    /**
     * Whether an invite link still works, for /join before the player types anything.
     */
    public static class InviteCheck {
        public boolean valid;
        /** why it doesn't work, when it doesn't */
        public String reason;
        public long expiresAt;
    }

    static List<RpcMethod> methods(ApiContext ctx) {
        return Arrays.asList(
                RpcMethod.named("adminAudit")
                        .params(optional("actor", STRING), optional("action", STRING), optional("target", STRING), optional("before", INT))
                        .returns("AuditEntry[]")
                        .doc("Staff: the audit log, newest first, 100 at a time. action matches a family (\"access\" covers "
                                + "\"access.mode\"); before is the id to page back from.")
                        .handler(call -> {
                            Staff.require(ctx, call, StaffRole.MODERATOR);
                            long before = call.has(3) ? AccountApi.longValue(call, 3) : 0;
                            return store(() -> AccessPolicy.get().store().auditLog(call.optString(0, null), call.optString(1, null),
                                    call.optString(2, null), before, 100));
                        }),

                RpcMethod.named("adminAccess")
                        .returns("AccessOverview")
                        .doc("Staff: the access mode, notice, allowlist, invites and access requests.")
                        .handler(call -> {
                            Staff.require(ctx, call, StaffRole.ADMIN);
                            return overview(ctx);
                        }),

                RpcMethod.named("adminAccessPreview")
                        .params(of("mode", STRING))
                        .returns("string[]")
                        .doc("Staff: who is online now and would be signed out by switching to this mode (open or allowlist).")
                        .handler(call -> {
                            Staff.require(ctx, call, StaffRole.ADMIN);
                            return names(modeOutsiders(ctx, parseMode(call.string(0))));
                        }),

                RpcMethod.named("adminSetAccessMode")
                        .params(of("mode", STRING), of("reason", STRING))
                        .returns("string[]")
                        .doc("Staff: switch between open and allowlist. Switching to allowlist signs out whoever isn't on it; "
                                + "returns their names. Needs accounts on the server.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            String mode = parseMode(call.string(0));
                            if (AccessPolicy.ALLOWLIST.equals(mode) && !accounts(ctx)) {
                                throw RpcException.invalidParams("An allowlist needs accounts: without them anyone can sign in "
                                        + "with any name. Turn on authentication (XMAGE_AUTH=true) first.");
                            }
                            String reason = requireReason(call, 1);
                            List<User> outsiders = modeOutsiders(ctx, mode);
                            store(() -> {
                                AccessPolicy.get().setMode(mode, actor, reason, System.currentTimeMillis());
                                return null;
                            });
                            return signOut(ctx, outsiders, actor, reason);
                        }),

                RpcMethod.named("adminAllowAdd")
                        .params(of("nameOrEmail", STRING), optional("note", STRING))
                        .returns("AllowEntry")
                        .doc("Staff: allow an account name, or an email address (any account registered with it).")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            AllowEntry entry = store(() -> AccessPolicy.get().store().allow(call.string(0), call.optString(1, ""),
                                    actor, System.currentTimeMillis()));
                            Staff.audit(actor, "access.allow", who(entry), entry.note, null);
                            return entry;
                        }),

                RpcMethod.named("adminAllowRemovePreview")
                        .params(of("entryId", INT))
                        .returns("string[]")
                        .doc("Staff: who is online now and would be signed out by taking this entry off the allowlist.")
                        .handler(call -> {
                            Staff.require(ctx, call, StaffRole.ADMIN);
                            AllowEntry entry = entry(call);
                            return names(entryOutsiders(ctx, entry));
                        }),

                RpcMethod.named("adminAllowRemove")
                        .params(of("entryId", INT), of("reason", STRING))
                        .returns("string[]")
                        .doc("Staff: take an entry off the allowlist; whoever it let in is signed out. Returns their names.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            String reason = requireReason(call, 1);
                            AllowEntry entry = entry(call);
                            List<User> outsiders = entryOutsiders(ctx, entry);
                            store(() -> AccessPolicy.get().store().disallow(entry.id));
                            Staff.audit(actor, "access.disallow", who(entry), null, reason);
                            return signOut(ctx, outsiders, actor, reason);
                        }),

                RpcMethod.named("adminInviteCreate")
                        .params(optional("note", STRING), optional("maxUses", INT), optional("days", INT))
                        .returns("Invite")
                        .doc("Staff: a new invite link code (/join?invite=code). It registers an account and allowlists it; "
                                + "1 use and 7 days unless asked otherwise (at most " + AdminStore.MAX_INVITE_USES + " uses and "
                                + AdminStore.MAX_INVITE_DAYS + " days).")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            Invite invite = store(() -> AccessPolicy.get().store().createInvite(call.optString(0, ""),
                                    call.optInteger(1, 1), call.optInteger(2, 7), actor, System.currentTimeMillis()));
                            Staff.audit(actor, "access.invite.create", invite.code, invite.maxUses + " uses"
                                    + (invite.note == null || invite.note.isEmpty() ? "" : ", " + invite.note), null);
                            return invite;
                        }),

                RpcMethod.named("adminInviteRevoke")
                        .params(of("code", STRING))
                        .returns("boolean")
                        .doc("Staff: stop an invite link from working. False when it was revoked already or doesn't exist.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            boolean revoked = store(() -> AccessPolicy.get().store().revokeInvite(call.string(0)));
                            if (revoked) {
                                Staff.audit(actor, "access.invite.revoke", call.string(0), null, null);
                            }
                            return revoked;
                        }),

                RpcMethod.named("adminAccessDecide")
                        .params(of("requestId", INT), of("approve", BOOLEAN))
                        .returns("AccessRequest | null")
                        .doc("Staff: approve an access request (its email address goes on the allowlist, and the person is "
                                + "emailed when the server has mail) or decline it. Null when it was decided already.")
                        .handler(call -> decide(ctx, call)),

                RpcMethod.named("adminSetNotice")
                        .params(of("notice", STRING))
                        .returns("boolean")
                        .doc("Staff: the notice shown on sign-in and Home, e.g. \"builds from master, data may be reset\". "
                                + "Empty clears it. At most " + AccessPolicy.MAX_NOTICE + " characters.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            store(() -> {
                                AccessPolicy.get().setNotice(call.string(0), actor, System.currentTimeMillis());
                                return null;
                            });
                            return true;
                        }),

                RpcMethod.named("inviteCheck")
                        .publicAccess()
                        .params(of("code", STRING))
                        .returns("InviteCheck")
                        .doc("Whether an invite link still works. Callable before login.")
                        .handler(call -> inviteCheck(call.string(0))),

                RpcMethod.named("githubStart")
                        .publicAccess()
                        .returns("string")
                        .doc("Sign in with GitHub: the GitHub page to send the player to. GitHub sends them back to "
                                + GithubAuth.CALLBACK_PATH + "?code=...&state=..., for githubFinish. Fails when it isn't set up.")
                        .handler(call -> store(() -> GithubAuth.get().start())),

                RpcMethod.named("githubFinish")
                        .publicAccess()
                        .params(of("state", STRING), of("code", STRING), optional("userName", STRING))
                        .returns("GithubSignInResult")
                        .doc("Sign in with GitHub, back from GitHub: the account to sign in to with its rememberToken "
                                + "(connectUser's), or a suggestedName when a new account needs a name (call again with "
                                + "userName). On a contributors server, the repository's people and allowlisted emails get in.")
                        .handler(call -> {
                            String state = call.string(0);
                            String code = call.string(1);
                            String name = call.optString(2, null);
                            GithubSignIn signIn = new GithubSignIn(GithubAuth.get(), AccessPolicy.get(), AuthorizedUserRepository.getInstance());
                            return store(() -> signIn.finish(accounts(ctx), state, code, name, System.currentTimeMillis()));
                        }),

                RpcMethod.named("accessRequest")
                        .publicAccess()
                        .params(of("userName", STRING), of("email", STRING), optional("note", STRING), optional("turnstileToken", STRING))
                        .returns("boolean")
                        .doc("Ask to be let in to a contributors-only server, or for an account while registration is invite only: staff see it in /admin. "
                                + RegistrationLimiter.PER_HOUR + " an hour and " + RegistrationLimiter.PER_DAY + " a day from one address. "
                                + "When serverInfo has a turnstileSiteKey, turnstileToken is the Turnstile widget's token. Callable before login.")
                        .handler(call -> requestAccess(ctx, call))
        );
    }

    // --- overview ---------------------------------------------------------------------------------

    private static AccessOverview overview(ApiContext ctx) throws RpcException {
        AccessPolicy policy = AccessPolicy.get();
        AccessOverview result = new AccessOverview();
        result.mode = policy.mode();
        result.accounts = accounts(ctx);
        result.mail = mail(ctx);
        result.notice = policy.notice();
        result.owner = policy.owner();
        result.allowlist = store(() -> policy.store().allowlist());
        result.invites = store(() -> policy.store().invites(100));
        result.requests = store(() -> policy.store().accessRequests(50));
        GithubAuth github = GithubAuth.get();
        result.github = github.isEnabled();
        result.githubRepo = github.repo();
        result.githubPeople = github.peopleCount();
        result.githubSyncedAt = github.syncedAt();
        result.githubProblem = github.syncProblem();
        return result;
    }

    // --- who gets signed out ----------------------------------------------------------------------

    /** online players outside the allowlist once the mode is {@code mode} */
    private static List<User> modeOutsiders(ApiContext ctx, String mode) {
        if (!AccessPolicy.ALLOWLIST.equals(mode) || !accounts(ctx)) {
            return new ArrayList<>();
        }
        AccessPolicy policy = AccessPolicy.get();
        return online(ctx, (name, email) -> {
            if (policy.isOwner(name)) {
                return true;
            }
            try {
                return policy.store().isListed(name, email);
            } catch (SQLException e) {
                return true; // can't tell: nobody is signed out by mistake
            }
        });
    }

    /** online players this entry lets in, and nothing else does */
    private static List<User> entryOutsiders(ApiContext ctx, AllowEntry entry) {
        AccessPolicy policy = AccessPolicy.get();
        if (!policy.isAllowlist() || !accounts(ctx)) {
            return new ArrayList<>();
        }
        return online(ctx, (name, email) -> {
            if (!matches(entry, name, email) || policy.isOwner(name)) {
                return true;
            }
            try {
                return policy.store().listedCount(name, email) > 1;
            } catch (SQLException e) {
                return true;
            }
        });
    }

    static boolean matches(AllowEntry entry, String name, String email) {
        return (entry.name != null && name != null && entry.name.equalsIgnoreCase(name.trim()))
                || (entry.email != null && email != null && entry.email.equalsIgnoreCase(email.trim()));
    }

    /** online players (not staff consoles) for whom {@code allowed} says no */
    private static List<User> online(ApiContext ctx, BiPredicate<String, String> allowed) {
        List<User> result = new ArrayList<>();
        if (ctx.managers == null) {
            return result;
        }
        for (User user : ctx.managers.userManager().getUsers()) {
            if (!user.isOnlineUser() || isConsole(ctx, user)) {
                continue;
            }
            AuthorizedUser account = AuthorizedUserRepository.getInstance().getByName(user.getName());
            String email = account == null ? null : account.getEmail();
            if (!allowed.test(user.getName(), email)) {
                result.add(user);
            }
        }
        result.sort((a, b) -> a.getName().compareToIgnoreCase(b.getName()));
        return result;
    }

    private static boolean isConsole(ApiContext ctx, User user) {
        return ctx.managers.sessionManager().getSession(user.getSessionId()).map(Session::isAdmin).orElse(false);
    }

    private static List<String> signOut(ApiContext ctx, List<User> users, String actor, String reason) {
        List<String> names = names(users);
        for (User user : users) {
            Staff.audit(actor, "access.signout", user.getName(), null, reason);
            String sessionId = user.getSessionId();
            user.showUserMessage("Signed out", "This server is for contributors only now, and your account isn't on its list.");
            ctx.managers.threadExecutor().getCallExecutor().execute(() -> {
                try {
                    Thread.sleep(1000); // time for the message to arrive
                } catch (InterruptedException e) {
                    Thread.currentThread().interrupt();
                }
                ctx.managers.sessionManager().disconnect(sessionId, DisconnectReason.DisconnectedByAdmin, true);
            });
        }
        return names;
    }

    private static List<String> names(List<User> users) {
        List<String> names = new ArrayList<>();
        for (User user : users) {
            names.add(user.getName());
        }
        return names;
    }

    // --- requests and invites ---------------------------------------------------------------------

    private static Object decide(ApiContext ctx, RpcCall call) throws RpcException {
        String actor = Staff.require(ctx, call, StaffRole.ADMIN);
        long id = AccountApi.longValue(call, 0);
        boolean approve = call.bool(1);
        long now = System.currentTimeMillis();
        AdminStore store = AccessPolicy.get().store();
        AccessRequest request = store(() -> store.decide(id, approve, actor, now));
        if (request == null) {
            return null;
        }
        if (approve) {
            store(() -> store.allow(request.email, "asked as " + request.name, actor, now));
        }
        Staff.audit(actor, approve ? "access.request.approve" : "access.request.decline", request.name,
                request.email, null);
        if (approve && mail(ctx)) {
            String server = serverLabel(ctx);
            String link = System.getProperty(DiscordNotifier.PUBLIC_URL_PROP, "").trim();
            String text = "You can play on " + server + " now. Register with this email address (" + request.email + ")"
                    + (link.isEmpty() ? "." : " at " + link + " .");
            if (!Mailer.send(ctx.managers, request.email, "You're in: " + server, text)) {
                logger.warn("Access approval email failed for request " + request.id);
            }
        }
        return request;
    }

    static InviteCheck inviteCheck(String code) throws RpcException {
        InviteCheck check = new InviteCheck();
        Invite invite = store(() -> AccessPolicy.get().store().invite(code));
        long now = System.currentTimeMillis();
        if (invite == null) {
            check.reason = "This invite link doesn't exist. Check that it was copied whole.";
        } else if (invite.revoked) {
            check.reason = "This invite link was revoked.";
        } else if (invite.uses >= invite.maxUses) {
            check.reason = "This invite link was used up.";
        } else if (now >= invite.expiresAt) {
            check.reason = "This invite link has expired.";
        } else {
            check.valid = true;
            check.expiresAt = invite.expiresAt;
        }
        return check;
    }

    private static Object requestAccess(ApiContext ctx, RpcCall call) throws RpcException {
        // an approved request puts the name on the allowlist, which also lets it register while registration is invite only
        if (!AccessPolicy.get().isAllowlist() && mage.server.admin.RuntimeSettings.get().registrationOpen()) {
            throw RpcException.invalidParams("This server is open to everyone: just register.");
        }
        String address = call.getConnection() == null ? "" : call.getConnection().getRemoteHost();
        long now = System.currentTimeMillis();
        String problem = ctx.registrations.take("request:" + address);
        if (problem != null) {
            throw new RpcException(RpcException.RATE_LIMITED, "You've asked a few times already. Try again later.");
        }
        problem = ctx.turnstile.check(call.optString(3, null), address);
        if (problem != null) {
            ctx.registrations.refund("request:" + address);
            throw RpcException.invalidParams(problem);
        }
        AccessRequest request;
        try {
            request = store(() -> AccessPolicy.get().store().requestAccess(call.string(0), call.string(1),
                    call.optString(2, ""), now));
        } catch (RpcException e) {
            ctx.registrations.refund("request:" + address);
            throw e;
        }
        if (request.createdAt == now) {
            DiscordNotifier.staff().accessRequested(request.name, request.note);
        }
        return true;
    }

    /** At most perHour calls an hour for this key (an address, or "kind:name"). */
    static boolean takeSlot(String key, long now, int perHour) {
        String address = key;
        synchronized (recentRequests) {
            recentRequests.values().removeIf(times -> times.isEmpty() || now - times.peekLast() >= HOUR);
            Deque<Long> times = recentRequests.computeIfAbsent(address, a -> new ArrayDeque<>());
            while (!times.isEmpty() && now - times.peekFirst() >= HOUR) {
                times.pollFirst();
            }
            if (times.size() >= perHour) {
                return false;
            }
            times.addLast(now);
            return true;
        }
    }

    // --- helpers ----------------------------------------------------------------------------------

    private static AllowEntry entry(RpcCall call) throws RpcException {
        long id = AccountApi.longValue(call, 0);
        AllowEntry entry = store(() -> AccessPolicy.get().store().allowEntry(id));
        if (entry == null) {
            throw RpcException.invalidParams("That allowlist entry is gone already");
        }
        return entry;
    }

    static String who(AllowEntry entry) {
        return entry.name != null ? entry.name : entry.email;
    }

    static String parseMode(String mode) throws RpcException {
        String value = mode == null ? "" : mode.trim().toLowerCase(Locale.ROOT);
        if (!AccessPolicy.OPEN.equals(value) && !AccessPolicy.ALLOWLIST.equals(value)) {
            throw RpcException.invalidParams("Mode is open or allowlist");
        }
        return value;
    }

    static String requireReason(RpcCall call, int index) throws RpcException {
        String reason = call.optString(index, "").trim();
        if (reason.isEmpty()) {
            throw RpcException.invalidParams("Say why: the reason goes in the audit log");
        }
        return reason.length() > AdminStore.MAX_REASON ? reason.substring(0, AdminStore.MAX_REASON) : reason;
    }

    static boolean accounts(ApiContext ctx) {
        return ctx.managers != null && ctx.managers.configSettings().isAuthenticationActivated();
    }

    static boolean mail(ApiContext ctx) {
        return ctx.managers != null && Mailer.configured(ctx.managers.configSettings());
    }

    private static String serverLabel(ApiContext ctx) {
        String name = ctx.managers.configSettings().getServerName();
        return isBlank(name) ? "the server" : name.trim();
    }

    private static boolean isBlank(String text) {
        return text == null || text.trim().isEmpty();
    }

    @FunctionalInterface
    interface StoreCall<T> {
        T run() throws SQLException, RpcException, IOException;
    }

    /** Store errors become RPC errors: bad input is the caller's, the rest the server's. */
    static <T> T store(StoreCall<T> work) throws RpcException {
        try {
            return work.run();
        } catch (IllegalArgumentException e) {
            throw RpcException.invalidParams(e.getMessage());
        } catch (IllegalStateException e) {
            throw new RpcException(RpcException.SERVER_ERROR, e.getMessage());
        } catch (SQLException e) {
            logger.error("Admin database error", e);
            throw new RpcException(RpcException.SERVER_ERROR, "The admin database can't be reached right now");
        } catch (IOException e) {
            logger.warn("GitHub request failed: " + e.getMessage());
            throw new RpcException(RpcException.SERVER_ERROR, "GitHub can't be reached right now. Try again in a minute.");
        }
    }
}
