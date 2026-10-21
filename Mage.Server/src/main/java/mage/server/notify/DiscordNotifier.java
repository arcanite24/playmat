package mage.server.notify;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import mage.game.Seat;
import mage.game.Table;
import mage.game.match.MatchPlayer;
import mage.game.tournament.Tournament;
import mage.players.PlayerType;
import mage.server.admin.RuntimeSettings;
import org.apache.log4j.Logger;

import java.io.IOException;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Collection;
import java.util.Deque;
import java.util.List;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.Executor;
import java.util.concurrent.RejectedExecutionException;
import java.util.concurrent.ThreadPoolExecutor;
import java.util.concurrent.TimeUnit;
import java.util.function.LongSupplier;

/**
 * Posts lobby news to a Discord channel through a webhook: a public table opening, an event (tournament) starting,
 * and a match starting. Off unless {@code xmage.discordWebhook} is set; {@code xmage.publicUrl} (the web client's
 * address, e.g. https://play.example.com) adds join links.
 * <p>
 * What is announced:
 * <ul>
 * <li>only tables with at least two human seats: AI-only tables and one player against the AI are practice, not
 * news</li>
 * <li>never password-protected tables</li>
 * <li>table openings from the main lobby room; match starts only for standalone tables (an event's matches are
 * covered by its start post)</li>
 * </ul>
 * Callers are game and lobby threads, so nothing here blocks: messages are built on the caller (cheap), then go to a
 * bounded queue drained by one daemon thread. At most {@link #RATE_LIMIT} posts a minute (Discord allows 30 per
 * webhook); a message over the limit or over the queue is dropped and logged. Posts never mention anyone. The webhook
 * address is a secret and is never logged.
 */
public final class DiscordNotifier {

    private static final Logger logger = Logger.getLogger(DiscordNotifier.class);

    public static final String WEBHOOK_PROP = "xmage.discordWebhook";
    public static final String PUBLIC_URL_PROP = "xmage.publicUrl";
    /** staff news (access requests, reports, staff changes) goes here when set, to keep it out of a public channel */
    public static final String STAFF_WEBHOOK_PROP = "xmage.discordStaffWebhook";

    static final int RATE_LIMIT = 30;
    static final long RATE_WINDOW_MS = 60_000;
    static final int QUEUE_CAPACITY = 20;
    static final int TIMEOUT_MS = 5_000;
    /** the mat's amber (Mage.Web.Client tokens.css --decision) */
    private static final int EMBED_COLOR = 0xF0B13D;

    /** Posts one JSON body to the webhook; returns the HTTP status. */
    @FunctionalInterface
    interface Sender {
        int post(String json) throws IOException;
    }

    private static volatile DiscordNotifier instance;
    private static volatile DiscordNotifier staffInstance;

    private final Sender sender;
    private final String publicUrl;
    private final Executor executor;
    private final LongSupplier clock;
    private final Deque<Long> recentPosts = new ArrayDeque<>();

    DiscordNotifier(Sender sender, String publicUrl, Executor executor, LongSupplier clock) {
        this.sender = sender;
        this.publicUrl = trimSlash(publicUrl);
        this.executor = executor;
        this.clock = clock;
    }

    /** The server's notifier, from system properties; does nothing when no webhook is configured. */
    public static DiscordNotifier instance() {
        DiscordNotifier result = instance;
        if (result == null) {
            synchronized (DiscordNotifier.class) {
                result = instance;
                if (result == null) {
                    result = fromSystemProperties(WEBHOOK_PROP);
                    instance = result;
                }
            }
        }
        return result;
    }

    /**
     * The staff channel's notifier: {@code xmage.discordStaffWebhook}, or the lobby's webhook when there is no staff
     * one (which was where access requests went before staff had their own channel).
     */
    public static DiscordNotifier staff() {
        DiscordNotifier result = staffInstance;
        if (result == null) {
            synchronized (DiscordNotifier.class) {
                result = staffInstance;
                if (result == null) {
                    result = System.getProperty(STAFF_WEBHOOK_PROP, "").trim().isEmpty() ? instance() : fromSystemProperties(STAFF_WEBHOOK_PROP);
                    staffInstance = result;
                }
            }
        }
        return result;
    }

    private static DiscordNotifier fromSystemProperties(String prop) {
        String webhook = System.getProperty(prop, "").trim();
        String publicUrl = System.getProperty(PUBLIC_URL_PROP, "").trim();
        if (webhook.isEmpty()) {
            return new DiscordNotifier(null, publicUrl, null, System::currentTimeMillis);
        }
        if (!webhook.startsWith("https://") && !webhook.startsWith("http://")) {
            logger.warn("Discord notifications off: " + prop + " is not an http(s) address");
            return new DiscordNotifier(null, publicUrl, null, System::currentTimeMillis);
        }
        ThreadPoolExecutor executor = new ThreadPoolExecutor(1, 1, 0, TimeUnit.MILLISECONDS,
                new ArrayBlockingQueue<>(QUEUE_CAPACITY), runnable -> {
            Thread thread = new Thread(runnable, "XMAGE discord webhook " + prop);
            thread.setDaemon(true);
            return thread;
        });
        logger.info("Discord notifications on (" + prop + ")" + (publicUrl.isEmpty() ? " (no " + PUBLIC_URL_PROP + ", so no join links)" : ""));
        return new DiscordNotifier(json -> httpPost(webhook, json), publicUrl, executor, System::currentTimeMillis);
    }

    public boolean isEnabled() {
        return sender != null;
    }

    // --- events -----------------------------------------------------------------------------------

    /** A table was created in the main lobby room (seats are still empty; their types are known). */
    public void tableOpened(Table table, String password) {
        if (!isEnabled() || !on(RuntimeSettings.DISCORD_TABLES)) {
            return;
        }
        int humans = humanSeats(table.getSeats());
        if (!announced(humans, password)) {
            return;
        }
        String link = table.isTournament() ? link("/events") : link("/join/" + table.getId());
        submit(tableOpenedMessage(table.isTournament(), table.getName(), table.getGameType(), table.getDeckType(),
                table.getControllerName(), table.getNumberOfSeats(), link));
    }

    /** An event (tournament) started with {@code humanPlayers} people in it. */
    public void eventStarted(Table table, Tournament tournament, String password, int humanPlayers) {
        if (!isEnabled() || !on(RuntimeSettings.DISCORD_EVENTS) || !announced(humanPlayers, password)) {
            return;
        }
        submit(eventStartedMessage(table.getName(), table.getGameType(), table.getDeckType(), table.getControllerName(),
                humanPlayers, link("/event/" + tournament.getId())));
    }

    /** A standalone match started (its first game): players seated, cards dealt. */
    public void matchStarted(Table table, String password, int humanPlayers, Collection<MatchPlayer> players) {
        if (!isEnabled() || !on(RuntimeSettings.DISCORD_MATCHES) || table.isTournamentSubTable() || !announced(humanPlayers, password)) {
            return;
        }
        List<String> names = new ArrayList<>();
        for (MatchPlayer player : players) {
            names.add(player.getName());
        }
        submit(matchStartedMessage(table.getName(), table.getGameType(), table.getDeckType(), names));
    }

    /** Someone asked to be let in to a contributors-only server; staff decide in /admin. */
    public void accessRequested(String name, String note) {
        if (!isEnabled() || !on(RuntimeSettings.DISCORD_STAFF)) {
            return;
        }
        submit(accessRequestedMessage(name, note, link("/admin")));
    }

    /** A player reported another; staff look at it in /admin. */
    public void reportFiled(String reporter, String reported, String reason) {
        if (!isEnabled() || !on(RuntimeSettings.DISCORD_STAFF)) {
            return;
        }
        submit(staffMessage("Report: " + escape(reporter) + " reported " + escape(reported), reason, link("/admin")));
    }

    /** A change only the owner makes: staff roles. */
    public void staffChanged(String actor, String change) {
        if (!isEnabled() || !on(RuntimeSettings.DISCORD_STAFF)) {
            return;
        }
        submit(staffMessage("Staff: " + escape(actor) + " " + escape(change), null, link("/admin")));
    }

    /** News for staff that has no event of its own (maintenance plans, backup trouble). */
    public void staffNews(String text) {
        if (!isEnabled() || !on(RuntimeSettings.DISCORD_STAFF)) {
            return;
        }
        submit(staffMessage(escape(text), null, link("/admin")));
    }

    /**
     * Someone supported the server and may be thanked in public (their provider's public name, or a player who chose
     * to appear on the supporters wall). The amount only when the owner turned amounts on.
     */
    public void supported(String name, String amount) {
        if (!isEnabled()) {
            return;
        }
        submit(supportedMessage(name, amount, link("/support")));
    }

    /**
     * Posts a test message now, past the switches and the queue, so staff see at once whether the webhook works.
     *
     * @return the HTTP status Discord answered with
     * @throws IOException when Discord can't be reached
     * @throws IllegalStateException when no webhook is set
     */
    public int postTest(String text) throws IOException {
        if (!isEnabled()) {
            throw new IllegalStateException("No webhook is set for this channel");
        }
        return sender.post(staffMessage(escape(text), null, link("/admin")));
    }

    /** a kind of post staff can turn off in /admin; on when the admin database can't say */
    private static boolean on(String kind) {
        return RuntimeSettings.get().discord(kind);
    }

    // --- rules ------------------------------------------------------------------------------------

    /** The announcement rule: at least two people, and no password. */
    static boolean announced(int humanPlayers, String password) {
        return humanPlayers >= 2 && (password == null || password.isEmpty());
    }

    static int humanSeats(Seat[] seats) {
        int humans = 0;
        for (Seat seat : seats) {
            if (seat != null && seat.getPlayerType() == PlayerType.HUMAN) {
                humans++;
            }
        }
        return humans;
    }

    // --- messages ---------------------------------------------------------------------------------

    static String tableOpenedMessage(boolean event, String name, String gameType, String format, String host, int seats, String link) {
        String what = event ? "an event" : "a table";
        StringBuilder content = new StringBuilder()
                .append(escape(host)).append(" opened ").append(what).append(": **").append(escape(name)).append("** (")
                .append(format).append(", ").append(seats).append(" seats)");
        if (link != null) {
            content.append(event ? " Sign up: " : " Join: ").append(link);
        }
        JsonObject embed = embed((event ? "Event open: " : "Table open: ") + escape(name), link);
        JsonArray fields = new JsonArray();
        fields.add(field("Format", format));
        fields.add(field(event ? "Event" : "Game", gameType));
        fields.add(field("Host", escape(host)));
        fields.add(field("Seats", String.valueOf(seats)));
        embed.add("fields", fields);
        return payload(content.toString(), embed);
    }

    static String eventStartedMessage(String name, String type, String format, String host, int players, String link) {
        StringBuilder content = new StringBuilder()
                .append("Event starting: **").append(escape(name)).append("** (").append(format).append(", ")
                .append(players).append(" players)");
        if (link != null) {
            content.append(" Follow it: ").append(link);
        }
        JsonObject embed = embed("Event starting: " + escape(name), link);
        JsonArray fields = new JsonArray();
        fields.add(field("Format", format));
        fields.add(field("Event", type));
        fields.add(field("Host", escape(host)));
        fields.add(field("Players", String.valueOf(players)));
        embed.add("fields", fields);
        return payload(content.toString(), embed);
    }

    static String matchStartedMessage(String name, String gameType, String format, List<String> players) {
        List<String> escaped = new ArrayList<>();
        for (String player : players) {
            escaped.add(escape(player));
        }
        String versus = String.join(" vs ", escaped);
        String content = "Match ready: " + versus + " (**" + escape(name) + "**, " + format + ")";
        JsonObject embed = embed("Match ready: " + escape(name), null);
        embed.addProperty("description", limit(versus, 4096));
        JsonArray fields = new JsonArray();
        fields.add(field("Format", format));
        fields.add(field("Game", gameType));
        embed.add("fields", fields);
        return payload(content, embed);
    }

    static String accessRequestedMessage(String name, String note, String link) {
        String content = "Access request from **" + escape(name) + "**" + (link == null ? "" : " Decide: " + link);
        JsonObject embed = embed("Access request: " + escape(name), link);
        if (note != null && !note.trim().isEmpty()) {
            embed.addProperty("description", limit(escape(note), 4096));
        }
        return payload(content, embed);
    }

    static String supportedMessage(String name, String amount, String link) {
        String text = "\uD83D\uDC9A " + escape(name) + " supported the server" + (amount == null ? "" : " (" + escape(amount) + ")")
                + " \u2014 thank you!";
        return payload(text + (link == null ? "" : " " + link), embed(text, link));
    }

    static String staffMessage(String title, String text, String link) {
        JsonObject embed = embed(title, link);
        if (text != null && !text.trim().isEmpty()) {
            embed.addProperty("description", limit(escape(text), 4096));
        }
        return payload(title + (link == null ? "" : " " + link), embed);
    }

    private static JsonObject embed(String title, String link) {
        JsonObject embed = new JsonObject();
        embed.addProperty("title", limit(title, 256));
        if (link != null) {
            embed.addProperty("url", link);
        }
        embed.addProperty("color", EMBED_COLOR);
        return embed;
    }

    private static JsonObject field(String name, String value) {
        JsonObject field = new JsonObject();
        field.addProperty("name", name);
        field.addProperty("value", value == null || value.trim().isEmpty() ? "-" : limit(value, 1024));
        field.addProperty("inline", true);
        return field;
    }

    /** The message with its embed; mentions are switched off, so names like "@everyone" stay text. */
    private static String payload(String content, JsonObject embed) {
        JsonObject json = new JsonObject();
        json.addProperty("content", limit(content, 2000));
        JsonArray embeds = new JsonArray();
        embeds.add(embed);
        json.add("embeds", embeds);
        JsonObject mentions = new JsonObject();
        mentions.add("parse", new JsonArray());
        json.add("allowed_mentions", mentions);
        return json.toString();
    }

    /** Player-chosen text (table and player names): one line, Discord markdown shown as typed. */
    static String escape(String text) {
        if (text == null) {
            return "";
        }
        return text.replaceAll("[\\r\\n]+", " ").replaceAll("([\\\\*_~`|>\\[\\]()#<])", "\\\\$1");
    }

    private static String limit(String text, int max) {
        return text.length() <= max ? text : text.substring(0, max - 1) + "…";
    }

    private String link(String path) {
        return publicUrl.isEmpty() ? null : publicUrl + path;
    }

    private static String trimSlash(String url) {
        String result = url == null ? "" : url.trim();
        while (result.endsWith("/")) {
            result = result.substring(0, result.length() - 1);
        }
        return result;
    }

    // --- delivery ---------------------------------------------------------------------------------

    /** Queues one message; false when it was dropped (over the rate limit, or the queue is full). */
    boolean submit(String json) {
        if (!takeRateSlot()) {
            logger.warn("Discord notification dropped: more than " + RATE_LIMIT + " a minute");
            return false;
        }
        try {
            executor.execute(() -> deliver(json));
            return true;
        } catch (RejectedExecutionException e) {
            logger.warn("Discord notification dropped: " + QUEUE_CAPACITY + " already waiting");
            return false;
        }
    }

    private synchronized boolean takeRateSlot() {
        long now = clock.getAsLong();
        while (!recentPosts.isEmpty() && now - recentPosts.peekFirst() >= RATE_WINDOW_MS) {
            recentPosts.pollFirst();
        }
        if (recentPosts.size() >= RATE_LIMIT) {
            return false;
        }
        recentPosts.addLast(now);
        return true;
    }

    private void deliver(String json) {
        try {
            int status = sender.post(json);
            if (status < 200 || status >= 300) {
                // 429 included: Discord's own limit; the message is dropped rather than retried
                logger.warn("Discord notification refused: HTTP " + status);
            }
        } catch (IOException | RuntimeException e) {
            logger.warn("Discord notification failed: " + e.getClass().getSimpleName());
        }
    }

    private static int httpPost(String webhook, String json) throws IOException {
        HttpURLConnection connection = (HttpURLConnection) new URL(webhook).openConnection();
        try {
            connection.setConnectTimeout(TIMEOUT_MS);
            connection.setReadTimeout(TIMEOUT_MS);
            connection.setInstanceFollowRedirects(false);
            connection.setRequestMethod("POST");
            connection.setDoOutput(true);
            connection.setRequestProperty("Content-Type", "application/json");
            connection.setRequestProperty("User-Agent", "XMage server (https://github.com/magefree/mage)");
            byte[] body = json.getBytes(StandardCharsets.UTF_8);
            connection.setFixedLengthStreamingMode(body.length);
            try (OutputStream out = connection.getOutputStream()) {
                out.write(body);
            }
            return connection.getResponseCode();
        } finally {
            connection.disconnect();
        }
    }
}
