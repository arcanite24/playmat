package mage.server.notify;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import mage.game.Table;
import mage.players.PlayerType;
import mage.server.admin.AdminStore;
import mage.server.admin.RuntimeSettings;
import org.junit.jupiter.api.Test;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.UUID;
import java.util.concurrent.Executor;
import java.util.concurrent.atomic.AtomicLong;

import static org.assertj.core.api.Assertions.assertThat;

/**
 * Discord webhook posts: what is announced, what the messages say, and the rate limit. No HTTP: the sender records
 * what would have been posted, and the executor runs it on the spot.
 */
public class DiscordNotifierTest {

    private final List<String> posted = new ArrayList<>();
    private final AtomicLong now = new AtomicLong(1_000_000);
    private final Executor direct = Runnable::run;

    private DiscordNotifier notifier(String publicUrl) {
        return new DiscordNotifier(json -> {
            posted.add(json);
            return 204;
        }, publicUrl, direct, now::get);
    }

    private static Table table(String name, PlayerType... seats) {
        return new Table(UUID.randomUUID(), "Two Player Duel", name, "alice", null, Arrays.asList(seats), null,
                Collections.emptySet(), false) {
        };
    }

    private static JsonObject parse(String json) {
        return JsonParser.parseString(json).getAsJsonObject();
    }

    @Test
    public void onlyPublicTablesWithTwoOrMorePeopleAreAnnounced() {
        assertThat(DiscordNotifier.announced(2, null)).isTrue();
        assertThat(DiscordNotifier.announced(4, "")).isTrue();
        // practice against the AI, or the AI on its own
        assertThat(DiscordNotifier.announced(1, null)).isFalse();
        assertThat(DiscordNotifier.announced(0, null)).isFalse();
        // private tables
        assertThat(DiscordNotifier.announced(2, "secret")).isFalse();
    }

    @Test
    public void tablesAgainstTheAiAreNotPosted() {
        DiscordNotifier notifier = notifier("https://play.example.com");
        notifier.tableOpened(table("practice", PlayerType.HUMAN, PlayerType.COMPUTER_MAD), null);
        notifier.tableOpened(table("bots", PlayerType.COMPUTER_MAD, PlayerType.COMPUTER_MONTE_CARLO), null);
        notifier.tableOpened(table("private", PlayerType.HUMAN, PlayerType.HUMAN), "secret");
        assertThat(posted).isEmpty();

        notifier.tableOpened(table("open", PlayerType.HUMAN, PlayerType.HUMAN, PlayerType.COMPUTER_MAD), null);
        assertThat(posted).hasSize(1);
    }

    @Test
    public void aTableOpeningCarriesItsDetailsAndAJoinLink() {
        Table table = table("Friday night", PlayerType.HUMAN, PlayerType.HUMAN);
        notifier("https://play.example.com/").tableOpened(table, null);

        JsonObject json = parse(posted.get(0));
        String content = json.get("content").getAsString();
        assertThat(content).startsWith("alice opened a table: **Friday night** (<deck type missing>, 2 seats)");
        assertThat(content).contains("alice", "Friday night", "2 seats", "https://play.example.com/join/" + table.getId());
        JsonObject embed = json.getAsJsonArray("embeds").get(0).getAsJsonObject();
        assertThat(embed.get("title").getAsString()).isEqualTo("Table open: Friday night");
        assertThat(embed.get("url").getAsString()).isEqualTo("https://play.example.com/join/" + table.getId());
        assertThat(embed.getAsJsonArray("fields").toString()).contains("Two Player Duel", "\"Host\"", "alice");
        // nobody is pinged, whatever the names say
        assertThat(json.getAsJsonObject("allowed_mentions").getAsJsonArray("parse")).isEmpty();
    }

    @Test
    public void withoutAPublicAddressThereIsNoLink() {
        notifier("").tableOpened(table("no link", PlayerType.HUMAN, PlayerType.HUMAN), null);
        JsonObject json = parse(posted.get(0));
        assertThat(json.get("content").getAsString()).doesNotContain("Join:");
        assertThat(json.getAsJsonArray("embeds").get(0).getAsJsonObject().has("url")).isFalse();
    }

    @Test
    public void eventAndMatchMessages() {
        JsonObject event = parse(DiscordNotifier.eventStartedMessage("Cube night", "Booster Draft Elimination",
                "Limited", "alice", 8, "https://play.example.com/event/42"));
        assertThat(event.get("content").getAsString())
                .isEqualTo("Event starting: **Cube night** (Limited, 8 players) Follow it: https://play.example.com/event/42");

        JsonObject match = parse(DiscordNotifier.matchStartedMessage("Duel", "Two Player Duel", "Constructed - Modern",
                Arrays.asList("alice", "bob")));
        assertThat(match.get("content").getAsString()).isEqualTo("Match ready: alice vs bob (**Duel**, Constructed - Modern)");
        assertThat(match.getAsJsonArray("embeds").get(0).getAsJsonObject().get("description").getAsString()).isEqualTo("alice vs bob");
    }

    @Test
    public void playerTextCannotFormatOrMention() {
        assertThat(DiscordNotifier.escape("**big**\n<@123> _x_")).isEqualTo("\\*\\*big\\*\\* \\<@123\\> \\_x\\_");
        assertThat(DiscordNotifier.escape(null)).isEmpty();
    }

    @Test
    public void overTheRateLimitMessagesAreDropped() {
        DiscordNotifier notifier = notifier("");
        for (int i = 0; i < DiscordNotifier.RATE_LIMIT; i++) {
            assertThat(notifier.submit("{}")).isTrue();
        }
        assertThat(notifier.submit("{}")).isFalse();
        assertThat(posted).hasSize(DiscordNotifier.RATE_LIMIT);

        // the window slides: a minute after the first posts there is room again
        now.addAndGet(DiscordNotifier.RATE_WINDOW_MS);
        assertThat(notifier.submit("{}")).isTrue();
        assertThat(posted).hasSize(DiscordNotifier.RATE_LIMIT + 1);
    }

    @Test
    public void aFullQueueDropsInsteadOfWaiting() {
        DiscordNotifier notifier = new DiscordNotifier(json -> 204, "", command -> {
            throw new java.util.concurrent.RejectedExecutionException("full");
        }, now::get);
        assertThat(notifier.submit("{}")).isFalse();
    }

    @Test
    public void failedPostsAreSwallowed() {
        DiscordNotifier refused = new DiscordNotifier(json -> 429, "", direct, now::get);
        assertThat(refused.submit("{}")).isTrue();
        DiscordNotifier broken = new DiscordNotifier(json -> {
            throw new java.io.IOException("timeout");
        }, "", direct, now::get);
        assertThat(broken.submit("{}")).isTrue();
    }

    @Test
    public void withoutAWebhookNothingHappens() {
        DiscordNotifier off = new DiscordNotifier(null, "https://play.example.com", null, now::get);
        assertThat(off.isEnabled()).isFalse();
        off.tableOpened(table("open", PlayerType.HUMAN, PlayerType.HUMAN), null);
        assertThat(posted).isEmpty();
    }

    @Test
    public void aJoinIsPostedOnlyWhenStaffTurnedItOn(@org.junit.jupiter.api.io.TempDir java.nio.file.Path dir) throws Exception {
        RuntimeSettings settings = RuntimeSettings.create(new AdminStore("jdbc:sqlite:" + dir.resolve("admin.db")));
        RuntimeSettings.use(settings);
        try {
            DiscordNotifier notifier = notifier("https://play.example.com");
            Table table = table("Friday night", PlayerType.HUMAN, PlayerType.HUMAN, PlayerType.HUMAN);
            notifier.playerJoined(table, null, "bob");
            assertThat(posted).as("off by default").isEmpty();

            settings.set(RuntimeSettings.DISCORD_JOINS, "on", "ana", 1000);
            notifier.playerJoined(table, "secret", "bob");
            assertThat(posted).as("never password-protected tables").isEmpty();

            notifier.playerJoined(table, "", "bob");
            String content = parse(posted.get(0)).get("content").getAsString();
            assertThat(content).startsWith("bob joined alice's table **Friday night**")
                    .contains("0/3 seats", "https://play.example.com/join/" + table.getId());
        } finally {
            RuntimeSettings.use(null);
        }
    }

    @Test
    public void aJoinNamesTheHostAndDropsTheLinkOnceTheTableIsFull() {
        assertThat(DiscordNotifier.host("alice, bob")).isEqualTo("alice");
        assertThat(DiscordNotifier.host("alice")).isEqualTo("alice");
        String full = DiscordNotifier.playerJoinedMessage("b*b", "alice", "Duel", "Modern", 2, 2, "https://play.example.com/join/1");
        JsonObject json = parse(full);
        assertThat(json.get("content").getAsString()).isEqualTo("b\\*b joined alice's table **Duel** (Modern, 2/2 seats)");
        assertThat(json.getAsJsonArray("embeds").get(0).getAsJsonObject().has("url")).isFalse();
        assertThat(json.getAsJsonObject("allowed_mentions").getAsJsonArray("parse")).isEmpty();
    }
}
