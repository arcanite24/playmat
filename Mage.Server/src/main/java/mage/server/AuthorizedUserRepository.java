package mage.server;

import com.j256.ormlite.dao.Dao;
import com.j256.ormlite.dao.DaoManager;
import com.j256.ormlite.jdbc.JdbcConnectionSource;
import com.j256.ormlite.stmt.DeleteBuilder;
import com.j256.ormlite.stmt.QueryBuilder;
import com.j256.ormlite.stmt.SelectArg;
import com.j256.ormlite.misc.TransactionManager;
import com.j256.ormlite.support.ConnectionSource;
import com.j256.ormlite.support.DatabaseConnection;
import com.j256.ormlite.table.TableUtils;
import mage.cards.repository.CardRepository;
import mage.cards.repository.DatabaseUtils;
import mage.cards.repository.RepositoryUtil;
import org.apache.log4j.Logger;
import org.apache.shiro.crypto.RandomNumberGenerator;
import org.apache.shiro.crypto.SecureRandomNumberGenerator;
import org.apache.shiro.crypto.hash.Hash;
import org.apache.shiro.crypto.hash.Sha256Hash;
import org.apache.shiro.crypto.hash.SimpleHash;

import java.io.File;
import java.sql.SQLException;
import java.util.Date;
import java.util.List;
import java.util.concurrent.TimeUnit;

public class AuthorizedUserRepository {

    private static final Logger LOGGER = Logger.getLogger(AuthorizedUserRepository.class);

    private static final String VERSION_ENTITY_NAME = "authorized_user";
    // raise this if db structure was changed
    private static final long DB_VERSION = 2;
    private static final RandomNumberGenerator rng = new SecureRandomNumberGenerator();

    /** the accounts since Nov 2026: SQLite, so the Rust platform service can read them too */
    public static final String SQLITE_NAME = "authorized_user.db";

    private static final String[] SQLITE_TABLES = {
            "CREATE TABLE IF NOT EXISTS authorized_user (name VARCHAR COLLATE NOCASE PRIMARY KEY, password VARCHAR,"
                    + " salt VARCHAR, hashAlgorithm VARCHAR, hashIterations INTEGER, email VARCHAR COLLATE NOCASE UNIQUE,"
                    + " active BOOLEAN, lockedUntil VARCHAR, chatLockedUntil VARCHAR, lastConnection VARCHAR)",
            "CREATE TABLE IF NOT EXISTS remembered_login (tokenHash VARCHAR PRIMARY KEY, name VARCHAR COLLATE NOCASE, lastUsed BIGINT)",
            "CREATE INDEX IF NOT EXISTS remembered_login_name_index ON remembered_login (name)"
    };

    private static volatile AuthorizedUserRepository instance;
    static {
        instance = open(new File("db"), System.getenv("XMAGE_ACCOUNTS_DB"));
    }

    // "keep me signed in": a device that hasn't signed in for this long has to type the password again
    private static final long REMEMBERED_LOGIN_TTL_MS = TimeUnit.DAYS.toMillis(90);
    private static final long REMEMBERED_LOGIN_TOUCH_MS = TimeUnit.DAYS.toMillis(1);
    private static final int REMEMBERED_LOGINS_PER_USER = 10;

    private Dao<AuthorizedUser, Object> usersDao;
    private Dao<RememberedLogin, String> rememberedDao;
    private final boolean sqlite;

    public AuthorizedUserRepository(String connectionString) {
        File file = new File("db");
        if (!file.exists()) {
            file.mkdirs();
        }
        sqlite = connectionString.startsWith("jdbc:sqlite:");
        try {
            ConnectionSource connectionSource = new JdbcConnectionSource(connectionString);
            if (sqlite) {
                // H2 opens the accounts with IGNORECASE=TRUE: names and emails match in any case, and SQLite does that
                // per column. ORMLite then finds the tables made.
                DatabaseConnection connection = connectionSource.getReadWriteConnection("authorized_user");
                try {
                    for (String sql : SQLITE_TABLES) {
                        connection.executeStatement(sql, DatabaseConnection.DEFAULT_RESULT_FLAGS);
                    }
                } finally {
                    connectionSource.releaseConnection(connection);
                }
            }
            TableUtils.createTableIfNotExists(connectionSource, AuthorizedUser.class);
            usersDao = DaoManager.createDao(connectionSource, AuthorizedUser.class);
            TableUtils.createTableIfNotExists(connectionSource, RememberedLogin.class);
            rememberedDao = DaoManager.createDao(connectionSource, RememberedLogin.class);
        } catch (SQLException ex) {
            LOGGER.error("Error creating / assigning authorized_user repository - ", ex);
        }
    }

    /**
     * The server's accounts in {@code dir}. By default db/authorized_user.db (SQLite); the first time it is made, it
     * is filled from db/authorized_user.h2, which stays as it was. {@code mode} h2 goes back to the H2 file: the
     * SQLite accounts are copied into it first and the SQLite file is put aside, so nothing made since is lost and the
     * next switch to SQLite copies again. A copy that fails leaves the accounts where they were.
     */
    static AuthorizedUserRepository open(File dir, String mode) {
        dir.mkdirs();
        File sqliteFile = new File(dir, SQLITE_NAME);
        String h2Url = DatabaseUtils.prepareH2Connection(DatabaseUtils.DB_NAME_USERS, false).replace("./db/", (dir.isAbsolute() ? dir.getPath() : "./" + dir.getPath()) + "/");
        boolean h2Exists = new File(dir, DatabaseUtils.DB_NAME_USERS + ".mv.db").exists();
        if ("h2".equalsIgnoreCase(mode == null ? "" : mode.trim())) {
            AuthorizedUserRepository h2 = new AuthorizedUserRepository(h2Url);
            if (sqliteFile.exists()) {
                AuthorizedUserRepository from = new AuthorizedUserRepository(sqliteUrl(sqliteFile));
                try {
                    long copied = copy(from, h2);
                    from.close();
                    File aside = new File(dir, SQLITE_NAME + ".rolled-back-" + System.currentTimeMillis());
                    if (!sqliteFile.renameTo(aside)) {
                        LOGGER.error("Accounts: copied back to H2, but " + sqliteFile + " couldn't be put aside");
                    }
                    LOGGER.info("Accounts: back on H2 with the " + copied + " accounts from SQLite; the SQLite file is now " + aside.getName());
                } catch (SQLException ex) {
                    from.close();
                    LOGGER.error("Accounts: the SQLite accounts couldn't be copied back to H2; staying on SQLite", ex);
                    h2.close();
                    return new AuthorizedUserRepository(sqliteUrl(sqliteFile));
                }
            }
            return h2;
        }
        if (!sqliteFile.exists() && h2Exists) {
            File part = new File(dir, SQLITE_NAME + ".part");
            part.delete();
            AuthorizedUserRepository h2 = new AuthorizedUserRepository(h2Url);
            AuthorizedUserRepository to = new AuthorizedUserRepository(sqliteUrl(part));
            try {
                if (!h2.checkAlterAndMigrateAuthorizedUser()) {
                    throw new SQLException("the H2 accounts couldn't be brought to version " + DB_VERSION);
                }
                long copied = copy(h2, to);
                to.close();
                if (!part.renameTo(sqliteFile)) {
                    throw new SQLException("couldn't rename " + part + " to " + sqliteFile);
                }
                h2.close();
                LOGGER.info("Accounts: " + copied + " accounts copied from H2 to " + sqliteFile + "; the H2 file stays as it was");
            } catch (SQLException ex) {
                to.close();
                part.delete();
                LOGGER.error("Accounts: the H2 accounts couldn't be copied to SQLite; staying on H2", ex);
                return h2;
            }
        }
        return new AuthorizedUserRepository(sqliteUrl(sqliteFile));
    }

    private static String sqliteUrl(File file) {
        return "jdbc:sqlite:" + file.getPath();
    }

    /** Replaces every account and remembered device in {@code to} with {@code from}'s, in one transaction; returns the accounts. */
    static long copy(AuthorizedUserRepository from, AuthorizedUserRepository to) throws SQLException {
        List<AuthorizedUser> users = from.usersDao.queryForAll();
        List<RememberedLogin> remembered = from.rememberedDao.queryForAll();
        TransactionManager.callInTransaction(to.usersDao.getConnectionSource(), () -> {
            to.usersDao.deleteBuilder().delete();
            to.rememberedDao.deleteBuilder().delete();
            for (AuthorizedUser user : users) {
                to.usersDao.create(user);
            }
            for (RememberedLogin login : remembered) {
                to.rememberedDao.create(login);
            }
            return null;
        });
        if (to.usersDao.countOf() != users.size()) {
            throw new SQLException("copied " + to.usersDao.countOf() + " of " + users.size() + " accounts");
        }
        return users.size();
    }

    public boolean isSqlite() {
        return sqlite;
    }

    /** Closes the accounts with every change written (unlike {@link #closeDB}, whose H2 shutdown drops the last second). */
    void close() {
        try {
            if (sqlite) {
                usersDao.getConnectionSource().close();
                return;
            }
            DatabaseConnection conn = usersDao.getConnectionSource().getReadWriteConnection(usersDao.getTableName());
            conn.executeStatement("SHUTDOWN", DatabaseConnection.DEFAULT_RESULT_FLAGS);
        } catch (Exception ex) {
            if (!alreadyClosed(ex)) {
                LOGGER.error("Error closing authorized_user repository - ", ex);
                return;
            }
        }
        // H2's SHUTDOWN closed the database and its connections: closing the source again only says so
        try {
            usersDao.getConnectionSource().close();
        } catch (Exception ignored) {
        }
    }

    /** H2's "Database is already closed" (90121): it has shut down already, with its changes written */
    static boolean alreadyClosed(Throwable error) {
        for (Throwable cause = error; cause != null; cause = cause.getCause()) {
            if (cause instanceof java.sql.SQLException && ((java.sql.SQLException) cause).getErrorCode() == 90121) {
                return true;
            }
        }
        return false;
    }

    public static AuthorizedUserRepository getInstance() {
        return instance;
    }

    /** For tests: the repository every server class sees. */
    public static void use(AuthorizedUserRepository repository) {
        instance = repository;
    }

    public void add(final String userName, final String password, final String email) {
        try {
            Hash hash = new SimpleHash(Sha256Hash.ALGORITHM_NAME, password, rng.nextBytes(), 1024);
            AuthorizedUser user = new AuthorizedUser(userName, hash, email);
            usersDao.create(user);
        } catch (SQLException ex) {
            LOGGER.error("Error adding a user to DB - ", ex);
        }
    }

    /**
     * Gives an existing user a new password, keeping everything else (email, locks, active flag).
     */
    public boolean changePassword(final String userName, final String password) {
        AuthorizedUser user = getByName(userName);
        if (user == null) {
            return false;
        }
        Hash hash = new SimpleHash(Sha256Hash.ALGORITHM_NAME, password, rng.nextBytes(), 1024);
        user.setPasswordHash(hash);
        update(user);
        // a new password signs every remembered device out
        forgetLogins(userName);
        return true;
    }

    /**
     * Gives an existing user another email address, or none (null). A verified address stops counting as verified:
     * see {@link EmailVerification}.
     */
    public boolean changeEmail(final String userName, final String email) {
        AuthorizedUser user = getByName(userName);
        if (user == null) {
            return false;
        }
        user.email = email == null || email.trim().isEmpty() ? null : email.trim();
        update(user);
        return true;
    }

    /**
     * Remembers a device for the user ("keep me signed in").
     *
     * @return the token the device signs in with instead of the password, or null when it couldn't be stored
     */
    public String rememberLogin(final String userName) {
        String token = rng.nextBytes(32).toHex();
        try {
            rememberedDao.create(new RememberedLogin(tokenHash(token), userName, new Date()));
            // keep the newest few devices per user
            QueryBuilder<RememberedLogin, String> qb = rememberedDao.queryBuilder();
            qb.where().eq("name", new SelectArg(userName));
            qb.orderBy("lastUsed", false);
            List<RememberedLogin> logins = rememberedDao.query(qb.prepare());
            for (RememberedLogin old : logins.subList(Math.min(REMEMBERED_LOGINS_PER_USER, logins.size()), logins.size())) {
                rememberedDao.delete(old);
            }
            return token;
        } catch (SQLException ex) {
            LOGGER.error("Error remembering a login - ", ex);
            return null;
        }
    }

    /**
     * @return whether the token is a remembered device of this user that was used recently enough; using it keeps it
     */
    public boolean isRememberedLogin(final String userName, final String token) {
        if (token == null || token.isEmpty()) {
            return false;
        }
        try {
            RememberedLogin login = rememberedDao.queryForId(tokenHash(token));
            if (login == null || !login.name.equals(userName)) {
                return false;
            }
            long now = System.currentTimeMillis();
            if (login.lastUsed == null || now - login.lastUsed.getTime() > REMEMBERED_LOGIN_TTL_MS) {
                rememberedDao.delete(login);
                return false;
            }
            if (now - login.lastUsed.getTime() > REMEMBERED_LOGIN_TOUCH_MS) {
                login.lastUsed = new Date(now);
                rememberedDao.update(login);
            }
            return true;
        } catch (SQLException ex) {
            LOGGER.error("Error checking a remembered login - ", ex);
            return false;
        }
    }

    /**
     * Forgets one remembered device (signing out on it).
     */
    public void forgetLogin(final String userName, final String token) {
        if (token == null || token.isEmpty()) {
            return;
        }
        try {
            DeleteBuilder<RememberedLogin, String> db = rememberedDao.deleteBuilder();
            db.where().eq("tokenHash", new SelectArg(tokenHash(token))).and().eq("name", new SelectArg(userName));
            db.delete();
        } catch (SQLException ex) {
            LOGGER.error("Error forgetting a remembered login - ", ex);
        }
    }

    public void forgetLogins(final String userName) {
        try {
            DeleteBuilder<RememberedLogin, String> db = rememberedDao.deleteBuilder();
            db.where().eq("name", new SelectArg(userName));
            db.delete();
        } catch (SQLException ex) {
            LOGGER.error("Error forgetting remembered logins - ", ex);
        }
    }

    private static String tokenHash(String token) {
        return new Sha256Hash(token).toHex();
    }

    public void remove(final String userName) {
        try {
            DeleteBuilder<AuthorizedUser, Object> db = usersDao.deleteBuilder();
            db.where().eq("name", new SelectArg(userName));
            db.delete();
        } catch (SQLException ex) {
            LOGGER.error("Error removing a user from DB - ", ex);
        }
        forgetLogins(userName);
    }

    /**
     * Accounts whose name or email contains the text (any case), by name, a page at a time.
     */
    public List<AuthorizedUser> list(String text, long offset, long limit) {
        try {
            QueryBuilder<AuthorizedUser, Object> qb = usersDao.queryBuilder().orderByRaw("LOWER(name)").offset(offset).limit(limit);
            matching(qb, text);
            return usersDao.query(qb.prepare());
        } catch (SQLException ex) {
            LOGGER.error("Error listing authorized_user - ", ex);
            return java.util.Collections.emptyList();
        }
    }

    public long count(String text) {
        try {
            QueryBuilder<AuthorizedUser, Object> qb = usersDao.queryBuilder().setCountOf(true);
            matching(qb, text);
            return usersDao.countOf(qb.prepare());
        } catch (SQLException ex) {
            LOGGER.error("Error counting authorized_user - ", ex);
            return 0;
        }
    }

    /**
     * Names of the accounts that can sign in whose name (never the email) contains the text, any case, by name, a page
     * at a time: the player directory's list, which must not let anyone look accounts up by email.
     */
    public List<String> listActiveNames(String text, long offset, long limit) {
        try {
            QueryBuilder<AuthorizedUser, Object> qb = usersDao.queryBuilder().selectColumns("name").orderByRaw("LOWER(name)").offset(offset).limit(limit);
            activeNamed(qb, text);
            List<String> names = new java.util.ArrayList<>();
            for (AuthorizedUser user : usersDao.query(qb.prepare())) {
                names.add(user.getName());
            }
            return names;
        } catch (SQLException ex) {
            LOGGER.error("Error listing authorized_user names - ", ex);
            return java.util.Collections.emptyList();
        }
    }

    public long countActiveNames(String text) {
        try {
            QueryBuilder<AuthorizedUser, Object> qb = usersDao.queryBuilder().setCountOf(true);
            activeNamed(qb, text);
            return usersDao.countOf(qb.prepare());
        } catch (SQLException ex) {
            LOGGER.error("Error counting authorized_user names - ", ex);
            return 0;
        }
    }

    private static void activeNamed(QueryBuilder<AuthorizedUser, Object> qb, String text) throws SQLException {
        com.j256.ormlite.stmt.Where<AuthorizedUser, Object> where = qb.where().eq("active", true);
        if (text != null && !text.trim().isEmpty()) {
            String like = "%" + text.trim().toLowerCase(java.util.Locale.ROOT).replace("\\", "\\\\").replace("%", "\\%").replace("_", "\\_") + "%";
            where.and().raw("LOWER(name) LIKE ? ESCAPE '\\'", new SelectArg(com.j256.ormlite.field.SqlType.STRING, like));
        }
    }

    private static void matching(QueryBuilder<AuthorizedUser, Object> qb, String text) throws SQLException {
        if (text == null || text.trim().isEmpty()) {
            return;
        }
        String like = "%" + text.trim().toLowerCase(java.util.Locale.ROOT).replace("\\", "\\\\").replace("%", "\\%").replace("_", "\\_") + "%";
        qb.where().raw("(LOWER(name) LIKE ? ESCAPE '\\' OR LOWER(email) LIKE ? ESCAPE '\\')",
                new SelectArg(com.j256.ormlite.field.SqlType.STRING, like), new SelectArg(com.j256.ormlite.field.SqlType.STRING, like));
    }

    public AuthorizedUser getByName(String userName) {
        try {
            QueryBuilder<AuthorizedUser, Object> qb = usersDao.queryBuilder();
            qb.where().eq("name", new SelectArg(userName));
            List<AuthorizedUser> results = usersDao.query(qb.prepare());
            if (results.size() == 1) {
                return results.get(0);
            }
            return null;
        } catch (SQLException ex) {
            LOGGER.error("Error getting a authorized_user - ", ex);
        }
        return null;
    }

    /**
     * The account with this name in any case, or null when there is none (or, from before names were checked that
     * way, more than one).
     */
    public AuthorizedUser getByNameIgnoreCase(String userName) {
        if (userName == null) {
            return null;
        }
        try {
            QueryBuilder<AuthorizedUser, Object> qb = usersDao.queryBuilder();
            qb.where().raw("LOWER(name) = ?", new SelectArg(com.j256.ormlite.field.SqlType.STRING,
                    userName.toLowerCase(java.util.Locale.ROOT)));
            List<AuthorizedUser> results = usersDao.query(qb.prepare());
            if (results.size() == 1) {
                return results.get(0);
            }
            return null;
        } catch (SQLException ex) {
            LOGGER.error("Error getting a authorized_user - ", ex);
        }
        return null;
    }

    public void update(AuthorizedUser authorizedUser) {
        try {
            usersDao.update(authorizedUser);
        } catch (SQLException ex) {
            LOGGER.error("Error updating authorized_user", ex);
        }
    }

    public AuthorizedUser getByEmail(String userName) {
        try {
            QueryBuilder<AuthorizedUser, Object> qb = usersDao.queryBuilder();
            qb.where().eq("email", new SelectArg(userName));
            List<AuthorizedUser> results = usersDao.query(qb.prepare());
            if (results.size() == 1) {
                return results.get(0);
            }
            return null;
        } catch (SQLException ex) {
            LOGGER.error("Error getting a authorized_user - ", ex);
        }
        return null;
    }

    public void closeDB() {
        try {
            if (sqlite && usersDao != null) {
                usersDao.getConnectionSource().close();
                return;
            }
            if (usersDao != null && usersDao.getConnectionSource() != null) {
                DatabaseConnection conn = usersDao.getConnectionSource().getReadWriteConnection(usersDao.getTableName());
                conn.executeStatement("SHUTDOWN IMMEDIATELY", DatabaseConnection.DEFAULT_RESULT_FLAGS);
                usersDao.getConnectionSource().releaseConnection(conn);
            }
        } catch (Exception ex) {
            LOGGER.error("Error closing authorized_user repository - ", ex);
        }
    }

    public long getDBVersionFromDB() {
        try {
            ConnectionSource connectionSource = new JdbcConnectionSource(DatabaseUtils.prepareH2Connection(DatabaseUtils.DB_NAME_USERS, false));
            return RepositoryUtil.getDatabaseVersion(connectionSource, VERSION_ENTITY_NAME);
        } catch (SQLException ex) {
            LOGGER.error("Error getting DB version from DB - ", ex);
        }
        return 0;
    }

    public boolean checkAlterAndMigrateAuthorizedUser() {
        if (sqlite) {
            // made at version 2
            return true;
        }
        long currentDBVersion = getDBVersionFromDB();
        if (currentDBVersion == 1 && DB_VERSION == 2) {
            return migrateFrom1To2();
        }
        return true;
    }

    private boolean migrateFrom1To2() {
        try {
            LOGGER.info("Starting " + VERSION_ENTITY_NAME + " DB migration from version 1 to version 2");
            usersDao.executeRaw("ALTER TABLE authorized_user ADD COLUMN active BOOLEAN DEFAULT true;");
            usersDao.executeRaw("ALTER TABLE authorized_user ADD COLUMN lockedUntil DATETIME;");
            usersDao.executeRaw("ALTER TABLE authorized_user ADD COLUMN chatLockedUntil DATETIME;");
            usersDao.executeRaw("ALTER TABLE authorized_user ADD COLUMN lastConnection DATETIME;");
            RepositoryUtil.updateVersion(usersDao.getConnectionSource(), VERSION_ENTITY_NAME, DB_VERSION);
            LOGGER.info("Migration finished.");
            return true;
        } catch (SQLException ex) {
            LOGGER.error("Error while migrating from version 1 to version 2 - ", ex);
            return false;
        }
    }
}
