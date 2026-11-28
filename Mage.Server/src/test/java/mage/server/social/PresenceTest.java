package mage.server.social;

import mage.view.RoomUsersView;
import mage.view.UsersView;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.UUID;
import java.util.stream.Collectors;

import static org.assertj.core.api.Assertions.assertThat;

/**
 * Appear offline (#229): the status lasts across sessions, hidden players are left out of the online list except for
 * themselves and staff, and whispers reach them only from players they whispered first.
 */
public class PresenceTest {

    @TempDir
    Path dir;
    private PresenceStore store;
    private final UUID alice = UUID.randomUUID();

    @BeforeEach
    void open() throws Exception {
        store = new PresenceStore("jdbc:sqlite:" + dir.resolve("presence.db"));
        PresenceStore.use(store);
    }

    @AfterEach
    void close() {
        PresenceStore.use(null);
        Presence.forget(alice);
    }

    private static UsersView user(String name) {
        return new UsersView("world.png", name, "", 0, "", 0, "", "", 0, 0, 0).withStatus(Presence.statusOf(name));
    }

    private static List<String> names(List<UsersView> users) {
        return users.stream().map(UsersView::getUserName).collect(Collectors.toList());
    }

    @Test
    void theStatusLastsUntilChangedAndSurvivesARestart() throws Exception {
        assertThat(Presence.statusOf("Alice")).isEqualTo(Presence.ONLINE);
        store.set("Alice", "hidden", 1000);
        assertThat(Presence.isHidden("alice")).as("any case").isTrue();

        PresenceStore reopened = new PresenceStore("jdbc:sqlite:" + dir.resolve("presence.db"));
        assertThat(reopened.status("ALICE")).isEqualTo(Presence.HIDDEN);

        store.set("alice", "looking", 2000);
        assertThat(Presence.statusOf("Alice")).isEqualTo(Presence.LOOKING);
        store.set("alice", "online", 3000);
        assertThat(new PresenceStore("jdbc:sqlite:" + dir.resolve("presence.db")).status("alice")).isEqualTo(Presence.ONLINE);

        assertThat(Presence.known("Hidden")).isTrue();
        assertThat(Presence.known("away")).isFalse();
        assertThat(Presence.normalize("away")).isEqualTo(Presence.ONLINE);
    }

    @Test
    void hiddenPlayersAreLeftOutOfTheOnlineListExceptForThemselvesAndStaff() throws Exception {
        store.set("alice", Presence.HIDDEN, 1000);
        store.set("carol", Presence.LOOKING, 1000);
        List<UsersView> online = Arrays.asList(user("alice"), user("bob"), user("carol"));

        assertThat(names(Presence.visible(online, "bob", false))).containsExactly("bob", "carol");
        assertThat(names(Presence.visible(online, null, false))).as("no session: the old clients' call").containsExactly("bob", "carol");
        assertThat(names(Presence.visible(online, "Alice", false))).as("she still sees herself").containsExactly("alice", "bob", "carol");
        List<UsersView> forStaff = Presence.visible(online, "mia", true);
        assertThat(names(forStaff)).containsExactly("alice", "bob", "carol");
        assertThat(forStaff.get(0).getStatus()).isEqualTo(Presence.HIDDEN);
        assertThat(forStaff.get(2).getStatus()).as("looking to play shows to everyone").isEqualTo(Presence.LOOKING);

        List<RoomUsersView> rooms = Collections.singletonList(new RoomUsersView(online, 3, 4, 5));
        RoomUsersView filtered = Presence.visibleRooms(rooms, "bob", false).get(0);
        assertThat(names(filtered.getUsersView())).containsExactly("bob", "carol");
        assertThat(filtered.getNumberActiveGames()).isEqualTo(3);
        assertThat(filtered.getNumberMaxGames()).isEqualTo(5);
    }

    @Test
    void whispersReachAHiddenPlayerOnlyFromPlayersSheWhisperedFirst() throws Exception {
        assertThat(Presence.whisperReaches("bob", alice, "alice")).as("online: anyone").isTrue();

        store.set("alice", Presence.HIDDEN, 1000);
        assertThat(Presence.whisperReaches("bob", alice, "alice")).as("refused like an offline player").isFalse();

        Presence.whispered(alice, "alice", "Bob");
        assertThat(Presence.whisperReaches("bob", alice, "alice")).as("bob may answer her").isTrue();
        assertThat(Presence.whisperReaches("carol", alice, "alice")).isFalse();

        Presence.forget(alice);
        assertThat(Presence.whisperReaches("bob", alice, "alice")).as("a new session starts over").isFalse();
    }
}
