package mage.server;

import com.j256.ormlite.dao.Dao;
import com.j256.ormlite.dao.DaoManager;
import com.j256.ormlite.jdbc.JdbcConnectionSource;
import com.j256.ormlite.stmt.DeleteBuilder;
import com.j256.ormlite.stmt.QueryBuilder;
import com.j256.ormlite.stmt.SelectArg;
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

    private static volatile AuthorizedUserRepository instance;
    static {
        instance = new AuthorizedUserRepository(DatabaseUtils.prepareH2Connection(DatabaseUtils.DB_NAME_USERS, false));
    }

    // "keep me signed in": a device that hasn't signed in for this long has to type the password again
    private static final long REMEMBERED_LOGIN_TTL_MS = TimeUnit.DAYS.toMillis(90);
    private static final long REMEMBERED_LOGIN_TOUCH_MS = TimeUnit.DAYS.toMillis(1);
    private static final int REMEMBERED_LOGINS_PER_USER = 10;

    private Dao<AuthorizedUser, Object> usersDao;
    private Dao<RememberedLogin, String> rememberedDao;

    public AuthorizedUserRepository(String connectionString) {
        File file = new File("db");
        if (!file.exists()) {
            file.mkdirs();
        }
        try {
            ConnectionSource connectionSource = new JdbcConnectionSource(connectionString);
            TableUtils.createTableIfNotExists(connectionSource, AuthorizedUser.class);
            usersDao = DaoManager.createDao(connectionSource, AuthorizedUser.class);
            TableUtils.createTableIfNotExists(connectionSource, RememberedLogin.class);
            rememberedDao = DaoManager.createDao(connectionSource, RememberedLogin.class);
        } catch (SQLException ex) {
            LOGGER.error("Error creating / assigning authorized_user repository - ", ex);
        }
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

    private static void matching(QueryBuilder<AuthorizedUser, Object> qb, String text) throws SQLException {
        if (text == null || text.trim().isEmpty()) {
            return;
        }
        String like = "%" + text.trim().toLowerCase(java.util.Locale.ROOT).replace("\\", "\\\\").replace("%", "\\%").replace("_", "\\_") + "%";
        qb.where().raw("(LOWER(name) LIKE ? OR LOWER(email) LIKE ?)",
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
            if (usersDao != null && usersDao.getConnectionSource() != null) {
                DatabaseConnection conn = usersDao.getConnectionSource().getReadWriteConnection(usersDao.getTableName());
                conn.executeStatement("SHUTDOWN IMMEDIATELY", DatabaseConnection.DEFAULT_RESULT_FLAGS);
                usersDao.getConnectionSource().releaseConnection(conn);
            }
        } catch (SQLException ex) {
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
