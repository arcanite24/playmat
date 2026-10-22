package mage.server.websocket.api;

import mage.server.Maintenance;
import mage.server.admin.AdminSetting;
import mage.server.admin.RuntimeSettings;
import mage.server.admin.StaffRole;
import mage.server.career.CareerService;
import mage.server.managers.ConfigSettings;
import mage.server.notify.DiscordNotifier;
import mage.server.websocket.rpc.RpcException;
import mage.server.websocket.rpc.RpcMethod;

import java.io.IOException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

import static mage.server.websocket.rpc.RpcParam.Type.BOOLEAN;
import static mage.server.websocket.rpc.RpcParam.Type.INT;
import static mage.server.websocket.rpc.RpcParam.Type.STRING;
import static mage.server.websocket.rpc.RpcParam.of;
import static mage.server.websocket.rpc.RpcParam.optional;

/**
 * Web client bridge: server settings staff change without a restart, planned maintenance, and the Discord channels.
 */
final class SettingsApi {

    /** One setting: what is in effect, and where it comes from. */
    public static class ServerSetting {
        public String name;
        /** in effect now */
        public String value;
        /** what the server's config says, which applies while nothing is stored */
        public String configValue;
        /** set from /admin (true), or following the config */
        public boolean changed;
        public String changedBy;
        public long changedAt;
        /** why it can't be changed here, or null */
        public String locked;
    }

    public static class ServerSettings {
        public List<ServerSetting> settings;
        /** lobby news has a webhook (the address itself stays on the server) */
        public boolean discordLobby;
        /** staff news has a webhook of its own (otherwise it goes to the lobby's) */
        public boolean discordStaff;
        /** planned maintenance, or null */
        public Maintenance.MaintenancePlan maintenance;
    }

    private static final long MAX_MAINTENANCE_MINUTES = 24 * 60;

    private SettingsApi() {
    }

    static List<RpcMethod> methods(ApiContext ctx) {
        return Arrays.asList(
                RpcMethod.named("adminSettings")
                        .returns("ServerSettings")
                        .doc("Staff (admin): settings that change without a restart (registration, Career, AI opponents, batch games, time to "
                                + "answer, Discord posts), each with where its value comes from; the Discord webhooks; planned maintenance.")
                        .handler(call -> {
                            Staff.require(ctx, call, StaffRole.ADMIN);
                            return AccessApi.store(() -> settings(ctx));
                        }),

                RpcMethod.named("adminSetSetting")
                        .params(of("name", STRING), of("value", STRING), optional("reason", STRING))
                        .returns("ServerSettings")
                        .doc("Staff (admin): change a setting; an empty value goes back to the server's config. The support.* "
                                + "settings (the goal, the Ko-fi page, what support pays for) are the owner's.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            String name = call.string(0);
                            String value = call.string(1);
                            if (name.startsWith("support.")) {
                                // where the money goes and what it says publicly: the owner's call
                                Staff.require(ctx, call, StaffRole.OWNER);
                            }
                            String reason = call.optString(2, null);
                            if (RuntimeSettings.CAREER.equals(name) && CareerService.offByEnvironment()) {
                                throw RpcException.invalidParams("Career is off in the server's environment (XMAGE_CAREER=off)");
                            }
                            String stored = AccessApi.store(() -> RuntimeSettings.get().set(name, value, actor, System.currentTimeMillis()));
                            Staff.audit(actor, "setting." + name, null, stored == null ? "config" : stored, reason);
                            return AccessApi.store(() -> settings(ctx));
                        }),

                RpcMethod.named("adminMaintenance")
                        .params(of("minutes", INT), optional("message", STRING), optional("restartWhenIdle", BOOLEAN))
                        .returns("ServerSettings")
                        .doc("Staff (owner): announce maintenance in so many minutes (0: now). New tables stop at once and players "
                                + "are warned; with restartWhenIdle the server exits once the time has come and no game is running, "
                                + "and its restart policy brings it back. A new announcement replaces the old one.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.OWNER);
                            long minutes = AccountApi.longValue(call, 0);
                            if (minutes < 0 || minutes > MAX_MAINTENANCE_MINUTES) {
                                throw RpcException.invalidParams("Maintenance starts within 0 to " + MAX_MAINTENANCE_MINUTES + " minutes");
                            }
                            String message = call.optString(1, "");
                            if (message.length() > 300) {
                                throw RpcException.invalidParams("The message is 300 characters at most");
                            }
                            boolean restart = call.optBool(2, false);
                            Maintenance.get().announce(ctx.managers, minutes, message, restart, actor);
                            Staff.audit(actor, "maintenance.announce", null, "in " + minutes + " min" + (restart ? ", restart when idle" : ""), message);
                            return AccessApi.store(() -> settings(ctx));
                        }),

                RpcMethod.named("adminMaintenanceCancel")
                        .returns("ServerSettings")
                        .doc("Staff (owner): call planned maintenance off; tables can start again.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.OWNER);
                            if (Maintenance.get().cancel(ctx.managers)) {
                                Staff.audit(actor, "maintenance.cancel", null, null, null);
                            }
                            return AccessApi.store(() -> settings(ctx));
                        }),

                RpcMethod.named("adminDiscordTest")
                        .params(of("channel", STRING))
                        .returns("string")
                        .doc("Staff (admin): post a test message to the lobby or staff channel now, and say what Discord answered.")
                        .handler(call -> {
                            String actor = Staff.require(ctx, call, StaffRole.ADMIN);
                            String channel = call.string(0);
                            DiscordNotifier notifier;
                            if ("lobby".equals(channel)) {
                                notifier = DiscordNotifier.instance();
                            } else if ("staff".equals(channel)) {
                                notifier = DiscordNotifier.staff();
                            } else {
                                throw RpcException.invalidParams("The channel is lobby or staff");
                            }
                            if (!notifier.isEnabled()) {
                                throw new RpcException(RpcException.SERVER_ERROR, "No webhook is set for the " + channel + " channel");
                            }
                            int status;
                            try {
                                status = notifier.postTest("Test from " + actor + " in /admin: this channel works.");
                            } catch (IOException e) {
                                throw new RpcException(RpcException.SERVER_ERROR, "Discord can't be reached: " + e.getMessage());
                            }
                            Staff.audit(actor, "discord.test", channel, "HTTP " + status, null);
                            return status / 100 == 2 ? "Posted: Discord answered " + status
                                    : "Discord refused it (HTTP " + status + "). Check the webhook address.";
                        })
        );
    }

    static ServerSettings settings(ApiContext ctx) throws java.sql.SQLException {
        ConfigSettings config = ctx.managers == null ? null : ctx.managers.configSettings();
        RuntimeSettings runtime = RuntimeSettings.get();
        ServerSettings result = new ServerSettings();
        result.settings = new ArrayList<>();
        result.settings.add(setting(runtime, RuntimeSettings.REGISTRATION, runtime.registrationOpen() ? "open" : "invite", "open",
                config != null && !config.isAuthenticationActivated() ? "This server has no accounts" : null));
        boolean envOff = CareerService.offByEnvironment();
        result.settings.add(setting(runtime, RuntimeSettings.CAREER, !envOff && runtime.careerOn() ? "on" : "off", envOff ? "off" : "on",
                envOff ? "Off in the server's environment (XMAGE_CAREER=off)" : null));
        String aiConfig = config == null ? null : config.getMaxAiOpponents();
        String ai = runtime.maxAiOpponents(aiConfig);
        result.settings.add(setting(runtime, RuntimeSettings.MAX_AI_OPPONENTS, ai == null ? "" : ai, aiConfig == null ? "" : aiConfig, null));
        int idleConfig = config == null ? 0 : config.getMaxSecondsIdle();
        result.settings.add(setting(runtime, RuntimeSettings.MAX_SECONDS_IDLE, String.valueOf(runtime.maxSecondsIdle(idleConfig)),
                String.valueOf(idleConfig), null));
        result.settings.add(setting(runtime, RuntimeSettings.BATCH_CONCURRENCY, String.valueOf(runtime.batchConcurrency()),
                String.valueOf(RuntimeSettings.BATCH_CONCURRENCY_DEFAULT), null));
        result.settings.add(setting(runtime, RuntimeSettings.BATCH_GAMES_PER_DAY, String.valueOf(runtime.batchGamesPerDay()),
                String.valueOf(RuntimeSettings.BATCH_GAMES_PER_DAY_DEFAULT), null));
        for (String kind : Arrays.asList(RuntimeSettings.DISCORD_TABLES, RuntimeSettings.DISCORD_EVENTS, RuntimeSettings.DISCORD_MATCHES,
                RuntimeSettings.DISCORD_STAFF)) {
            result.settings.add(setting(runtime, kind, runtime.discord(kind) ? "on" : "off", "on", null));
        }
        result.settings.add(setting(runtime, RuntimeSettings.SUPPORT_ENABLED, runtime.supportOn() ? "on" : "off", "on", null));
        result.settings.add(setting(runtime, RuntimeSettings.SUPPORT_GOAL_CENTS, String.valueOf(runtime.supportGoalCents()),
                String.valueOf(RuntimeSettings.DEFAULT_GOAL_CENTS), null));
        result.settings.add(setting(runtime, RuntimeSettings.SUPPORT_KOFI_URL, runtime.supportKofiUrl(), "", null));
        result.settings.add(setting(runtime, RuntimeSettings.SUPPORT_SPONSORS_URL, runtime.supportSponsorsUrl(), "", null));
        result.settings.add(setting(runtime, RuntimeSettings.SUPPORT_COSTS, runtime.supportCosts(), RuntimeSettings.DEFAULT_COSTS, null));
        result.settings.add(setting(runtime, RuntimeSettings.SUPPORT_DISCORD_AMOUNTS, runtime.supportDiscordAmounts() ? "on" : "off", "off", null));
        result.discordLobby = DiscordNotifier.instance().isEnabled();
        result.discordStaff = !System.getProperty(DiscordNotifier.STAFF_WEBHOOK_PROP, "").trim().isEmpty() && DiscordNotifier.staff().isEnabled();
        result.maintenance = Maintenance.get().plan();
        return result;
    }

    private static ServerSetting setting(RuntimeSettings runtime, String name, String value, String configValue, String locked)
            throws java.sql.SQLException {
        ServerSetting setting = new ServerSetting();
        setting.name = name;
        setting.value = value;
        setting.configValue = configValue;
        setting.changed = runtime.stored(name) != null;
        AdminSetting row = setting.changed ? runtime.row(name) : null;
        if (row != null) {
            setting.changedBy = row.changedBy;
            setting.changedAt = row.changedAt;
        }
        setting.locked = locked;
        return setting;
    }
}
