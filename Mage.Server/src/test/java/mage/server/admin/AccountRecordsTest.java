package mage.server.admin;

import mage.server.AuthorizedUserRepository;
import mage.server.decks.DeckStore;
import mage.server.moderation.ReportStore;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.util.UUID;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

/**
 * What the accounts page and an account deletion rely on: finding accounts, and every store forgetting one account
 * without touching anyone else's records.
 */
public class AccountRecordsTest {

    @TempDir
    Path dir;

    @Test
    void accountsAreFoundByNameOrEmailInAnyCase() {
        AuthorizedUserRepository accounts = new AuthorizedUserRepository("jdbc:h2:mem:list-" + UUID.randomUUID() + ";DB_CLOSE_DELAY=-1");
        accounts.add("Ravenna", "secret12", "raven@example.test");
        accounts.add("bob", "secret12", "bob@other.test");
        accounts.add("Carol_1", "secret12", "carol@example.test");

        assertThat(accounts.count("")).isEqualTo(3);
        assertThat(accounts.list("", 0, 50)).extracting(a -> a.getName()).containsExactly("bob", "Carol_1", "Ravenna");
        assertThat(accounts.list("RAV", 0, 50)).extracting(a -> a.getName()).containsExactly("Ravenna");
        assertThat(accounts.count("example.test")).isEqualTo(2);
        assertThat(accounts.list("", 1, 1)).extracting(a -> a.getName()).containsExactly("Carol_1");
        assertThat(accounts.count("_")).as("_ is a letter here, not a wildcard").isEqualTo(1);
        assertThat(accounts.count("%")).isZero();
    }

    @Test
    void storesForgetOneAccount() throws Exception {
        AdminStore store = new AdminStore("jdbc:sqlite:" + dir.resolve("admin.db"));
        store.allow("pat", "friend", "owner", 1);
        store.allow("sam", "friend", "owner", 1);
        store.grantStaff("Pat", StaffRole.MODERATOR, "owner", 1);
        store.linkGithub(7, "pat-gh", "Pat", 1);
        store.linkOidc("sub-pat", "Pat", "pat@example.test", "Pat", 1);
        store.linkOidc("sub-sam", "Sam", "sam@example.test", "sam", 1);
        store.addFeedback("Pat", "bug", "", "Cards overlap", "/game", 1);
        store.addFeedback("sam", "idea", "Dark mode", "More of it", "", 1);
        store.audit("owner", "user.mute", "Pat", null, null, 1);

        store.forgetAccount("Pat");

        assertThat(store.isListed("pat", null)).isFalse();
        assertThat(store.isListed("sam", null)).isTrue();
        assertThat(store.staffRole("pat")).isNull();
        assertThat(store.githubLinkOf("Pat")).isNull();
        assertThat(store.oidcLinkOf("Pat")).isNull();
        assertThat(store.oidcLink("sub-sam").account).isEqualTo("sam");
        assertThat(store.feedback(null, 10)).extracting(f -> f.userName).containsExactly("sam");
        assertThat(store.auditLog(null, null, "Pat", 0, 10)).as("the audit log stays").hasSize(1);

        DeckStore decks = new DeckStore("jdbc:sqlite:" + dir.resolve("decks.db"));
        decks.put("Pat", "d1", "Elves", 1, "{}");
        decks.put("sam", "d1", "Goblins", 1, "{}");
        assertThat(decks.export("Pat")).hasSize(1);
        decks.deleteAll("Pat");
        assertThat(decks.list("Pat", 2)).isEmpty();
        assertThat(decks.list("sam", 2)).hasSize(1);

        ReportStore reports = new ReportStore("jdbc:sqlite:" + dir.resolve("reports.db"));
        reports.file("sam", "Pat", "spam", "", null, 1);
        reports.file("Pat", "sam", "abuse", "", null, 1);
        reports.file("sam", "zed", "abuse", "", null, 1);
        assertThat(reports.involving("Pat", 10)).hasSize(2);
        reports.forget("Pat");
        assertThat(reports.list(false, 10)).extracting(r -> r.reported).containsExactly("zed");
    }

    @Test
    void feedbackIsWorkedThrough() throws Exception {
        AdminStore store = new AdminStore("jdbc:sqlite:" + dir.resolve("admin.db"));
        FeedbackEntry entry = store.addFeedback("pat", "weird", "", "The timer froze\nduring my turn", "/game 1.2", 5);

        assertThat(entry.type).isEqualTo("other");
        assertThat(entry.title).isEqualTo("The timer froze");
        assertThat(store.newFeedbackCount()).isEqualTo(1);

        store.markFeedback(entry.id, FeedbackEntry.DONE, "issue 12", "mia", 6);
        assertThat(store.feedback(FeedbackEntry.NEW, 10)).isEmpty();
        assertThat(store.feedback(FeedbackEntry.DONE, 10).get(0).note).isEqualTo("issue 12");
        assertThatThrownBy(() -> store.markFeedback(entry.id, "lost", null, "mia", 7)).isInstanceOf(IllegalArgumentException.class);
        assertThatThrownBy(() -> store.addFeedback("pat", "bug", "", "  ", "", 8)).isInstanceOf(IllegalArgumentException.class);
    }
}
