package mage.server.social;

import com.google.gson.Gson;
import mage.server.AuthorizedUser;
import mage.server.AuthorizedUserRepository;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.Arrays;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import java.util.stream.Collectors;

import static org.assertj.core.api.Assertions.assertThat;

/**
 * The player directory (#195): registered accounts by name with their level and whether they are online, searched by
 * name only, hidden players offline except to staff and themselves, and nothing but names and levels listed.
 */
public class PlayerDirectoryTest {

    @TempDir
    Path dir;
    private AuthorizedUserRepository accounts;
    private PresenceStore presence;
    private final Set<String> online = new HashSet<>();
    private final Map<String, Integer> levels = new HashMap<>();

    @BeforeEach
    void setUp() throws Exception {
        accounts = new AuthorizedUserRepository("jdbc:h2:mem:directory-" + UUID.randomUUID() + ";DB_CLOSE_DELAY=-1");
        accounts.add("Alice", "secret12", "alice@example.test");
        accounts.add("bob", "secret12", "bob@example.test");
        accounts.add("Carol", "secret12", "carol@mail.test");
        accounts.add("banned", "secret12", "banned@example.test");
        AuthorizedUser banned = accounts.getByName("banned");
        java.lang.reflect.Field active = AuthorizedUser.class.getDeclaredField("active");
        active.setAccessible(true);
        active.setBoolean(banned, false);
        accounts.update(banned);
        presence = new PresenceStore("jdbc:sqlite:" + dir.resolve("presence.db"));
        PresenceStore.use(presence);
        levels.put("alice", 12);
        levels.put("Carol", 3);
    }

    @AfterEach
    void tearDown() {
        PresenceStore.use(null);
    }

    private PlayerDirectory.DirectoryPage page(String query, String viewer, boolean staff) {
        return PlayerDirectory.page(accounts, query, 0, viewer, staff, name -> online.contains(name.toLowerCase()),
                names -> levels);
    }

    private static List<String> names(PlayerDirectory.DirectoryPage page) {
        return page.players.stream().map(player -> player.name).collect(Collectors.toList());
    }

    @Test
    void listsAccountsThatCanSignInByNameWithTheirLevels() {
        PlayerDirectory.DirectoryPage page = page("", "bob", false);
        assertThat(page.enabled).isTrue();
        assertThat(names(page)).containsExactly("Alice", "bob", "Carol");
        assertThat(page.total).isEqualTo(3);
        assertThat(page.players.get(0).level).as("matched without regard to case").isEqualTo(12);
        assertThat(page.players.get(1).level).isNull();
        assertThat(page.players.get(2).level).isEqualTo(3);

        String json = new Gson().toJson(page);
        assertThat(json).doesNotContain("@").doesNotContain("secret");
    }

    @Test
    void searchMatchesNamesNeverEmails() {
        assertThat(names(page("CAR", "bob", false))).containsExactly("Carol");
        assertThat(names(page("example.test", "bob", false))).as("no looking accounts up by email").isEmpty();
        assertThat(page("mail", "bob", false).total).isZero();
    }

    @Test
    void withoutCareerThereAreNoLevels() {
        PlayerDirectory.DirectoryPage page = PlayerDirectory.page(accounts, "", 0, "bob", false, name -> false, null);
        assertThat(page.players).allSatisfy(player -> assertThat(player.level).isNull());
    }

    @Test
    void hiddenPlayersShowOfflineExceptToStaffAndThemselves() throws Exception {
        online.addAll(Arrays.asList("alice", "carol"));
        presence.set("alice", Presence.HIDDEN, 1000);
        presence.set("carol", Presence.LOOKING, 1000);

        PlayerDirectory.DirectoryPage forBob = page("", "bob", false);
        assertThat(forBob.players.get(0).online).isFalse();
        assertThat(forBob.players.get(0).status).isNull();
        assertThat(forBob.players.get(1).online).isFalse();
        assertThat(forBob.players.get(2).online).isTrue();
        assertThat(forBob.players.get(2).status).isEqualTo(Presence.LOOKING);

        assertThat(page("", "alice", false).players.get(0).online).isTrue();
        PlayerDirectory.DirectoryPlayer forStaff = page("", "mia", true).players.get(0);
        assertThat(forStaff.online).isTrue();
        assertThat(forStaff.status).isEqualTo(Presence.HIDDEN);
    }

    @Test
    void pagesAndLongSearchesAreBounded() {
        PlayerDirectory.DirectoryPage second = PlayerDirectory.page(accounts, "", 2, "bob", false, name -> false, null);
        assertThat(names(second)).containsExactly("Carol");
        assertThat(second.offset).isEqualTo(2);
        assertThat(PlayerDirectory.page(accounts, "", -5, "bob", false, name -> false, null).offset).isZero();
        String longSearch = new String(new char[200]).replace('\0', 'a');
        assertThat(PlayerDirectory.page(accounts, longSearch, 0, "bob", false, name -> false, null).players).isEmpty();
    }
}
