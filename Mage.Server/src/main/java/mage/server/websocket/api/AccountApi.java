package mage.server.websocket.api;

import com.google.gson.JsonElement;
import mage.server.Main;
import mage.server.admin.AccessPolicy;
import mage.server.admin.RuntimeSettings;
import mage.server.decks.DeckStore;
import mage.server.decks.DeckSyncEntry;
import mage.server.managers.ConfigSettings;
import mage.server.notify.DiscordNotifier;
import mage.server.moderation.ClientErrors;
import mage.server.moderation.PlayerReport;
import mage.server.moderation.ReportStore;
import mage.server.websocket.rpc.RpcCall;
import mage.server.websocket.rpc.RpcException;
import mage.server.websocket.rpc.RpcMethod;

import java.sql.SQLException;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.ThreadPoolExecutor;

import static mage.server.websocket.rpc.RpcParam.Type.INT;
import static mage.server.websocket.rpc.RpcParam.Type.STRING;
import static mage.server.websocket.rpc.RpcParam.object;
import static mage.server.websocket.rpc.RpcParam.of;
import static mage.server.websocket.rpc.RpcParam.optional;

/**
 * Web client bridge: what the server offers before login, decks kept per account, player reports and crash reports.
 */
final class AccountApi {

    private AccountApi() {
    }

    /**
     * What the sign-in screen needs to know about the server.
     */
    public static class ServerInfo {
        /** players sign in with a registered name and a password */
        public boolean accounts;
        /** the server can send email (welcome mails, password reset codes) */
        public boolean mail;
        public boolean testMode;
        public String serverName;
        public int minUserNameLength;
        public int maxUserNameLength;
        public int minPasswordLength;
        public int maxPasswordLength;
        /** decks are kept on the server for each account */
        public boolean deckSync;
        /** the optional single-player Career is offered (accounts on, and not switched off) */
        public boolean career;
        /** open, or allowlist: a contributors server, where only listed or invited accounts get in */
        public String access;
        /** what staff want everyone to know, shown on sign-in and Home; empty when there is nothing */
        public String notice;
        /** players can sign in with GitHub (githubStart) */
        public boolean github;
        /**
         * single sign-on: the account name the sign-on proxy in front named for this connection; connectUser with it and
         * no password signs in. Absent when the server has no single sign-on or the connection didn't come through it
         */
        public String signOn;
        /** the server signs in only through its single sign-on: no passwords, registration, reset codes or GitHub */
        public boolean signOnOnly;
        /** open, or invite: new accounts need an invite link (or a place on the allowlist) */
        public String registration;
        /** when planned maintenance starts (epoch ms), or 0; no new tables can start meanwhile */
        public long maintenanceAt;
        public String maintenanceMessage;
        /** games running now, and how many can run at once (maxGameThreads); further games wait for a free slot */
        public int gamesRunning;
        public int maxGames;
        /** games waiting for a free slot */
        public int gamesWaiting;
        /** Cloudflare Turnstile's site key: registration and access requests need the widget's token. Absent when off */
        public String turnstileSiteKey;
        /** where players write about their data and the server (XMAGE_CONTACT_EMAIL); absent when not set */
        public String contactEmail;
        /** players can support the server here: a support page is set, or payments arrive (supportStatus says more) */
        public boolean support;
    }

    static List<RpcMethod> methods(ApiContext ctx) {
        return Arrays.asList(
                RpcMethod.named("serverInfo")
                        .publicAccess()
                        .returns("ServerInfo")
                        .doc("Whether the server has accounts and mail, and its name and password rules. Callable before login.")
                        .handler(call -> serverInfo(ctx, call.getConnection())),

                RpcMethod.named("deckSyncList")
                        .returns("DeckSyncEntry[]")
                        .doc("Every deck kept on the server for the signed-in account, without the deck data, deleted ones included "
                                + "(as tombstones, for " + 90 + " days). Only on servers with accounts.")
                        .handler(call -> {
                            String owner = account(ctx, call);
                            return decks(() -> DeckStore.get().list(owner, System.currentTimeMillis()));
                        }),

                RpcMethod.named("deckSyncGet")
                        .params(of("deckId", STRING))
                        .returns("DeckSyncEntry | null")
                        .doc("One deck of the signed-in account with its data, or null when there is none.")
                        .handler(call -> {
                            String owner = account(ctx, call);
                            return decks(() -> DeckStore.get().get(owner, call.string(0)));
                        }),

                RpcMethod.named("deckSyncPut")
                        .params(of("deckId", STRING), of("name", STRING), of("updatedAt", INT), of("data", STRING))
                        .returns("DeckPutResult")
                        .doc("Store a deck for the signed-in account. The newer change wins: when the server's copy is newer, "
                                + "nothing is stored and its updatedAt comes back. Up to " + DeckStore.MAX_DECKS + " decks of "
                                + DeckStore.MAX_DECK_CHARS / 1024 + " KB each.")
                        .handler(call -> {
                            String owner = account(ctx, call);
                            long updatedAt = longValue(call, 2);
                            return decks(() -> DeckStore.get().put(owner, call.string(0), call.string(1), updatedAt, call.string(3)));
                        }),

                RpcMethod.named("deckSyncDelete")
                        .params(of("deckId", STRING), of("deletedAt", INT))
                        .returns("DeckPutResult")
                        .doc("Delete a deck of the signed-in account, unless the server has a change newer than the deletion.")
                        .handler(call -> {
                            String owner = account(ctx, call);
                            long deletedAt = longValue(call, 1);
                            return decks(() -> DeckStore.get().delete(owner, call.string(0), deletedAt));
                        }),

                RpcMethod.named("reportPlayer")
                        .params(of("userName", STRING), of("reason", STRING), of("details", STRING), optional("gameId", STRING))
                        .returns("number")
                        .doc("Report a player to the server's moderators. Reasons: " + String.join(", ", ReportStore.REASONS)
                                + ". A player can have " + ReportStore.MAX_OPEN_PER_REPORTER + " open reports at a time. Returns the report id.")
                        .handler(call -> {
                            String reporter = ctx.sessions.userName(call.sessionId())
                                    .orElseThrow(() -> RpcException.notAuthorized("Sign in to report a player"));
                            PlayerReport report = decks(() -> ReportStore.get().file(reporter, call.string(0), call.string(1),
                                    call.string(2), call.optString(3, null), System.currentTimeMillis()));
                            DiscordNotifier.staff().reportFiled(reporter, call.string(0), call.string(1));
                            return report.id;
                        }),

                RpcMethod.named("clientReportError")
                        .publicAccess()
                        .params(object("report", "ClientError"))
                        .doc("A crash report from the web client: kept in the server log and the admin console, never sent elsewhere. "
                                + "Limited per connection; extra reports are dropped.")
                        .handler(call -> {
                            String reporter = call.getConnection() == null ? "" : call.getConnection().getRemoteHost();
                            String userName = call.sessionId() == null ? null : ctx.sessions.userName(call.sessionId()).orElse(null);
                            ClientErrors.ClientError report = call.object(0, ClientErrors.ClientError.class);
                            return ClientErrors.get().report(reporter, userName, report, System.currentTimeMillis());
                        })
        );
    }

    static ServerInfo serverInfo(ApiContext ctx) {
        return serverInfo(ctx, null);
    }

    static ServerInfo serverInfo(ApiContext ctx, mage.server.websocket.rpc.RpcConnection connection) {
        ServerInfo info = new ServerInfo();
        ConfigSettings config = ctx.managers == null ? null : ctx.managers.configSettings();
        info.testMode = Main.isTestMode();
        if (config != null) {
            info.accounts = config.isAuthenticationActivated();
            info.mail = mage.server.Mailer.configured(config);
            info.serverName = config.getServerName();
            info.minUserNameLength = config.getMinUserNameLength();
            info.maxUserNameLength = config.getMaxUserNameLength();
            info.minPasswordLength = config.getMinPasswordLength();
            info.maxPasswordLength = config.getMaxPasswordLength();
        }
        info.deckSync = info.accounts;
        mage.server.TrustedHeaderAuth signOn = mage.server.TrustedHeaderAuth.get();
        if (info.accounts && signOn.isEnabled()) {
            info.signOn = connection == null ? null : connection.getSignedOnUser();
            info.signOnOnly = !signOn.passwordSignIn();
        }
        info.turnstileSiteKey = ctx.turnstile.siteKey();
        info.contactEmail = contactEmail();
        if (ctx.managers != null && ctx.managers.threadExecutor() != null
                && ctx.managers.threadExecutor().getGameExecutor() instanceof ThreadPoolExecutor) {
            ThreadPoolExecutor games = (ThreadPoolExecutor) ctx.managers.threadExecutor().getGameExecutor();
            info.gamesRunning = games.getActiveCount();
            info.maxGames = games.getMaximumPoolSize();
            info.gamesWaiting = games.getQueue().size();
        }
        mage.server.Maintenance.MaintenancePlan maintenance = mage.server.Maintenance.get().plan();
        if (maintenance != null) {
            info.maintenanceAt = maintenance.startsAt;
            info.maintenanceMessage = maintenance.message;
        }
        info.career = mage.server.career.CareerService.enabled(info.accounts);
        try {
            AccessPolicy access = AccessPolicy.get();
            info.access = info.accounts ? access.mode() : AccessPolicy.OPEN;
            info.notice = access.notice();
            info.github = info.accounts && !info.signOnOnly && mage.server.admin.GithubAuth.get().isEnabled();
            info.registration = RuntimeSettings.get().registrationOpen() ? "open" : "invite";
            RuntimeSettings runtime = RuntimeSettings.get();
            info.support = runtime.supportOn() && (!runtime.supportKofiUrl().isEmpty() || !runtime.supportSponsorsUrl().isEmpty()
                    || mage.server.support.SupportHooks.listening());
        } catch (IllegalStateException e) {
            info.registration = "open";
            info.access = AccessPolicy.OPEN;
            info.notice = "";
        }
        return info;
    }

    public static final String CONTACT_EMAIL_PROP = "xmage.contactEmail";
    public static final String CONTACT_EMAIL_ENV = "XMAGE_CONTACT_EMAIL";

    static String contactEmail() {
        String value = System.getProperty(CONTACT_EMAIL_PROP);
        if (value == null || value.trim().isEmpty()) {
            value = System.getenv(CONTACT_EMAIL_ENV);
        }
        return value == null || value.trim().isEmpty() ? null : value.trim();
    }

    /**
     * The signed-in account's name; decks are only kept for real accounts, since anyone can take an anonymous name.
     */
    private static String account(ApiContext ctx, RpcCall call) throws RpcException {
        String userName = ctx.sessions.userName(call.sessionId())
                .orElseThrow(() -> RpcException.notAuthorized("Sign in first"));
        if (ctx.managers == null || !ctx.managers.configSettings().isAuthenticationActivated()) {
            throw RpcException.notAuthorized("This server has no accounts, so decks stay in your browser");
        }
        return userName;
    }

    static long longValue(RpcCall call, int index) throws RpcException {
        JsonElement value = call.raw(index);
        try {
            return value.getAsLong();
        } catch (RuntimeException e) {
            throw RpcException.invalidParams("Parameter #" + index + " must be a number");
        }
    }

    interface StoreCall<T> {
        T run() throws SQLException, RpcException;
    }

    /** Store errors as RPC errors: bad input is the caller's, anything else the server's. */
    static <T> T decks(StoreCall<T> work) throws RpcException {
        try {
            return work.run();
        } catch (IllegalArgumentException e) {
            throw RpcException.invalidParams(e.getMessage());
        } catch (IllegalStateException | SQLException e) {
            throw new RpcException(RpcException.SERVER_ERROR, e.getMessage(), e);
        }
    }
}
