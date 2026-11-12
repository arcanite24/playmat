package mage.server.social;

import com.j256.ormlite.dao.Dao;
import com.j256.ormlite.dao.DaoManager;
import com.j256.ormlite.jdbc.JdbcConnectionSource;
import com.j256.ormlite.support.ConnectionSource;
import com.j256.ormlite.table.TableUtils;
import org.apache.log4j.Logger;

import java.io.File;
import java.sql.SQLException;
import java.util.Locale;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

/**
 * The status each account chose (#229): hidden, looking to play, or online (no row). It lasts across sessions until
 * the player changes it. Stored in db/web_presence.db (SQLite), next to the other server databases and in the same
 * backups; every row is held in memory too, since the lobby reads the status of every online player every 2 seconds.
 */
public final class PresenceStore {

    private static final Logger logger = Logger.getLogger(PresenceStore.class);
    private static final String DEFAULT_URL = "jdbc:sqlite:./db/web_presence.db";

    private static volatile PresenceStore instance;
    private static volatile boolean broken;

    private final Dao<StoredPresence, String> dao;
    /** account (lower case) to status; online accounts are absent */
    private final Map<String, String> cache = new ConcurrentHashMap<>();

    public PresenceStore(String jdbcUrl) throws SQLException {
        ConnectionSource source = new JdbcConnectionSource(jdbcUrl);
        TableUtils.createTableIfNotExists(source, StoredPresence.class);
        dao = DaoManager.createDao(source, StoredPresence.class);
        for (StoredPresence row : dao.queryForAll()) {
            if (row.account != null && !Presence.ONLINE.equals(Presence.normalize(row.status))) {
                cache.put(row.account, Presence.normalize(row.status));
            }
        }
    }

    /** The server's store, or null when its database can't be opened (everyone is then simply online). */
    public static PresenceStore get() {
        PresenceStore current = instance;
        if (current == null && !broken) {
            synchronized (PresenceStore.class) {
                current = instance;
                if (current == null && !broken) {
                    new File("db").mkdirs();
                    try {
                        current = new PresenceStore(DEFAULT_URL);
                        instance = current;
                    } catch (SQLException | LinkageError e) {
                        // asked every 2 seconds by the lobby: say it once
                        broken = true;
                        logger.error("Presence database can't be opened; every player shows as online", e);
                    }
                }
            }
        }
        return current;
    }

    /** For tests: use this store instead of db/web_presence.db (null goes back to it). */
    public static void use(PresenceStore store) {
        instance = store;
        broken = false;
    }

    /** The account's status: {@link Presence#ONLINE} unless it chose another. */
    public String status(String account) {
        if (account == null) {
            return Presence.ONLINE;
        }
        return cache.getOrDefault(key(account), Presence.ONLINE);
    }

    public synchronized void set(String account, String status, long now) throws SQLException {
        String normalized = Presence.normalize(status);
        String key = key(account);
        if (Presence.ONLINE.equals(normalized)) {
            dao.deleteById(key);
            cache.remove(key);
            return;
        }
        StoredPresence row = new StoredPresence();
        row.account = key;
        row.status = normalized;
        row.updatedAt = now;
        dao.createOrUpdate(row);
        cache.put(key, normalized);
    }

    /** For the account's deletion. */
    public synchronized void forget(String account) throws SQLException {
        String key = key(account);
        dao.deleteById(key);
        cache.remove(key);
    }

    private static String key(String account) {
        return account.trim().toLowerCase(Locale.ROOT);
    }
}
