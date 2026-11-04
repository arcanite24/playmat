package mage.server;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.io.File;
import java.util.Arrays;
import java.util.Date;

import static org.assertj.core.api.Assertions.assertThat;

/**
 * The accounts move from H2 to SQLite once, and back with XMAGE_ACCOUNTS_DB=h2: every account, its password, locks
 * and remembered devices come along, names and emails still match in any case, and the copy that fails changes nothing.
 */
public class AccountsDbMoveTest {

    @TempDir
    File dir;

    @Test
    void movesToSqliteAndBack() {
        AuthorizedUserRepository h2 = AuthorizedUserRepository.open(dir, "h2");
        assertThat(h2.isSqlite()).isFalse();
        h2.add("Alice", "secret", "Alice@Example.test");
        h2.add("bob", "hunter2", "bob@example.test");
        AuthorizedUser alice = h2.getByName("alice");
        alice.lockedUntil = new Date(1_800_000_000_000L);
        h2.update(alice);
        String phone = h2.rememberLogin("Alice");
        h2.close();

        AuthorizedUserRepository sqlite = AuthorizedUserRepository.open(dir, null);
        assertThat(sqlite.isSqlite()).isTrue();
        assertThat(new File(dir, "authorized_user.db")).exists();
        assertThat(new File(dir, "authorized_user.db.part")).doesNotExist();
        assertThat(sqlite.count(null)).isEqualTo(2);
        AuthorizedUser moved = sqlite.getByName("ALICE");
        assertThat(moved.getName()).isEqualTo("Alice");
        assertThat(moved.doCredentialsMatch("Alice", "secret")).isTrue();
        assertThat(moved.getLockedUntil()).isEqualTo(new Date(1_800_000_000_000L));
        assertThat(sqlite.getByEmail("alice@example.TEST").getName()).isEqualTo("Alice");
        assertThat(sqlite.isRememberedLogin("Alice", phone)).isTrue();
        assertThat(sqlite.list("EXAMPLE", 0, 10)).extracting(AuthorizedUser::getName).containsExactly("Alice", "bob");

        // made on SQLite, then the server goes back to H2: nothing is lost
        sqlite.add("carol", "pw", null);
        sqlite.changePassword("bob", "new password");
        sqlite.close();
        AuthorizedUserRepository back = AuthorizedUserRepository.open(dir, "h2");
        assertThat(back.isSqlite()).isFalse();
        assertThat(back.count(null)).isEqualTo(3);
        assertThat(back.getByName("bob").doCredentialsMatch("bob", "new password")).isTrue();
        assertThat(back.isRememberedLogin("Alice", phone)).isTrue();
        assertThat(new File(dir, "authorized_user.db")).doesNotExist();
        assertThat(Arrays.stream(dir.list()).anyMatch(name -> name.startsWith("authorized_user.db.rolled-back-"))).isTrue();
        back.add("dave", "pw", null);
        back.close();

        // and forward again: copied afresh
        AuthorizedUserRepository again = AuthorizedUserRepository.open(dir, "");
        assertThat(again.isSqlite()).isTrue();
        assertThat(again.count(null)).isEqualTo(4);
        again.close();
    }

    @Test
    void aNewServerStartsOnSqlite() {
        AuthorizedUserRepository fresh = AuthorizedUserRepository.open(dir, null);
        assertThat(fresh.isSqlite()).isTrue();
        fresh.add("erin", "pw", "erin@example.test");
        assertThat(fresh.getByNameIgnoreCase("ERIN").getName()).isEqualTo("erin");
        fresh.close();
        assertThat(new File(dir, "authorized_user.h2.mv.db")).doesNotExist();
    }

    @Test
    void anH2ThatClosedAlreadyIsNoError() {
        // ORMLite wraps H2's "Database is already closed"
        Exception closed = new Exception("wrapped", new java.sql.SQLException("Database is already closed", "90121", 90121));
        assertThat(AuthorizedUserRepository.alreadyClosed(closed)).isTrue();
        assertThat(AuthorizedUserRepository.alreadyClosed(new java.sql.SQLException("other", "42000", 42000))).isFalse();
    }
}
