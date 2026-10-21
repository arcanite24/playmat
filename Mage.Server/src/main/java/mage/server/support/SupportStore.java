package mage.server.support;

import org.apache.log4j.Logger;

import java.io.File;
import java.security.SecureRandom;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.sql.Types;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;

/**
 * Support for the server: the payment ledger, GitHub sponsorships, each account's support code, the supporter perks
 * it owns, the look it plays with, whether it appears on the supporters wall, and a pending thank-you. Kept in
 * db/support.db (SQLite) with a versioned schema like the Career's: each migration runs once, in order, in a
 * transaction. Account names are matched without regard to case.
 */
public final class SupportStore implements AutoCloseable {

    private static final Logger logger = Logger.getLogger(SupportStore.class);
    private static final String DEFAULT_URL = "jdbc:sqlite:./db/support.db";

    private static final String[][] MIGRATIONS = {
            {
                    "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
                    "CREATE TABLE payments (id INTEGER PRIMARY KEY AUTOINCREMENT, provider TEXT NOT NULL,"
                            + " provider_id TEXT NOT NULL UNIQUE, kind TEXT NOT NULL, amount_cents INTEGER NOT NULL,"
                            + " currency TEXT NOT NULL, tier TEXT, from_name TEXT, message TEXT, is_public INTEGER NOT NULL DEFAULT 0,"
                            + " email_hash TEXT, account TEXT COLLATE NOCASE, linked_by TEXT, received_at INTEGER NOT NULL,"
                            + " raw_payload TEXT, revoked INTEGER NOT NULL DEFAULT 0, revoked_reason TEXT,"
                            + " sponsor_id INTEGER NOT NULL DEFAULT 0)",
                    "CREATE INDEX payments_by_time ON payments (received_at)",
                    "CREATE INDEX payments_by_account ON payments (account)",
                    "CREATE INDEX payments_by_email ON payments (email_hash)",
                    "CREATE TABLE codes (account TEXT PRIMARY KEY COLLATE NOCASE, code TEXT NOT NULL UNIQUE, created_at INTEGER NOT NULL)",
                    "CREATE TABLE perks (account TEXT NOT NULL COLLATE NOCASE, kind TEXT NOT NULL, item TEXT NOT NULL,"
                            + " granted_at INTEGER NOT NULL, payment_id INTEGER, PRIMARY KEY (account, kind, item))",
                    "CREATE TABLE notices (account TEXT PRIMARY KEY COLLATE NOCASE, payment_id INTEGER, at INTEGER NOT NULL)",
                    "CREATE TABLE sponsorships (github_id INTEGER PRIMARY KEY, login TEXT, node_id TEXT, monthly_cents INTEGER NOT NULL,"
                            + " tier TEXT, is_public INTEGER NOT NULL DEFAULT 0, active INTEGER NOT NULL, since INTEGER NOT NULL,"
                            + " until INTEGER NOT NULL DEFAULT 0, account TEXT COLLATE NOCASE, updated_at INTEGER NOT NULL)",
                    "CREATE TABLE looks (account TEXT PRIMARY KEY COLLATE NOCASE, sleeve TEXT, cloth TEXT, avatar TEXT, frame TEXT,"
                            + " title TEXT, flair TEXT, updated_at INTEGER NOT NULL)",
                    "CREATE TABLE prefs (account TEXT PRIMARY KEY COLLATE NOCASE, wall INTEGER NOT NULL DEFAULT 0)"
            },
            {
                    // pets: a look column, and the party corgi for everyone who already has the thank-you set
                    "ALTER TABLE looks ADD COLUMN pet TEXT",
                    "INSERT OR IGNORE INTO perks (account, kind, item, granted_at, payment_id)"
                            + " SELECT account, 'pet', 'party-corgi', granted_at, payment_id FROM perks WHERE kind = 'sleeve' AND item = 'supporter'"
            }
    };

    private static final String PAYMENT_COLUMNS = "id, provider, provider_id, kind, amount_cents, currency, tier, from_name, message,"
            + " is_public, email_hash, account, linked_by, received_at, raw_payload, revoked, revoked_reason, sponsor_id";

    private static volatile SupportStore instance;

    private final Connection connection;

    public SupportStore(String jdbcUrl) throws SQLException {
        connection = DriverManager.getConnection(jdbcUrl);
        try (Statement statement = connection.createStatement()) {
            statement.execute("PRAGMA busy_timeout = 5000");
        }
        migrate();
    }

    public static SupportStore get() {
        SupportStore current = instance;
        if (current == null) {
            synchronized (SupportStore.class) {
                current = instance;
                if (current == null) {
                    new File("db").mkdirs();
                    try {
                        current = new SupportStore(DEFAULT_URL);
                    } catch (SQLException | LinkageError e) {
                        logger.error("Support database can't be opened", e);
                        throw new IllegalStateException("Support can't be stored on this server right now");
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    /** For tests: use this store instead of db/support.db (null goes back to it). */
    public static void use(SupportStore store) {
        instance = store;
    }

    public synchronized int schemaVersion() throws SQLException {
        try (Statement statement = connection.createStatement()) {
            statement.execute("CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL)");
            try (ResultSet rows = statement.executeQuery("SELECT version FROM schema_version")) {
                return rows.next() ? rows.getInt(1) : 0;
            }
        }
    }

    private synchronized void migrate() throws SQLException {
        int version = schemaVersion();
        for (int next = version; next < MIGRATIONS.length; next++) {
            final int target = next + 1;
            transaction(() -> {
                try (Statement statement = connection.createStatement()) {
                    for (String sql : MIGRATIONS[target - 1]) {
                        statement.execute(sql);
                    }
                    statement.execute("DELETE FROM schema_version");
                    statement.execute("INSERT INTO schema_version (version) VALUES (" + target + ")");
                }
                return null;
            });
            logger.info("Support database schema is now version " + target);
        }
    }

    interface Work<T> {
        T run() throws SQLException;
    }

    /** runs the work in one transaction; inside another transaction it joins that one */
    synchronized <T> T transaction(Work<T> work) throws SQLException {
        boolean auto = connection.getAutoCommit();
        if (!auto) {
            return work.run();
        }
        connection.setAutoCommit(false);
        try {
            T result = work.run();
            connection.commit();
            return result;
        } catch (SQLException | RuntimeException e) {
            connection.rollback();
            throw e;
        } finally {
            connection.setAutoCommit(auto);
        }
    }

    // ---- meta

    /** The secret mixed into every email hash, made once per server: a leaked ledger can't be matched to addresses. */
    public synchronized String pepper() throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT value FROM meta WHERE key = 'pepper'")) {
            try (ResultSet rows = select.executeQuery()) {
                if (rows.next()) {
                    return rows.getString(1);
                }
            }
        }
        byte[] bytes = new byte[32];
        new SecureRandom().nextBytes(bytes);
        String pepper = Base64.getEncoder().encodeToString(bytes);
        try (PreparedStatement insert = connection.prepareStatement("INSERT INTO meta (key, value) VALUES ('pepper', ?)")) {
            insert.setString(1, pepper);
            insert.executeUpdate();
        }
        return pepper;
    }

    // ---- payments

    /** Adds a payment; false when one with its provider id is in the ledger already (the provider sent it again). */
    public synchronized boolean addPayment(Payment payment) throws SQLException {
        try (PreparedStatement insert = connection.prepareStatement("INSERT OR IGNORE INTO payments (provider, provider_id, kind,"
                + " amount_cents, currency, tier, from_name, message, is_public, email_hash, account, linked_by, received_at,"
                + " raw_payload, revoked, revoked_reason, sponsor_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, NULL, ?)",
                Statement.RETURN_GENERATED_KEYS)) {
            insert.setString(1, payment.provider);
            insert.setString(2, payment.providerId);
            insert.setString(3, payment.kind);
            insert.setLong(4, payment.amountCents);
            insert.setString(5, payment.currency);
            insert.setString(6, payment.tier);
            insert.setString(7, payment.fromName);
            insert.setString(8, payment.message);
            insert.setInt(9, payment.isPublic ? 1 : 0);
            insert.setString(10, payment.emailHash);
            insert.setString(11, payment.account);
            insert.setString(12, payment.linkedBy);
            insert.setLong(13, payment.receivedAt);
            insert.setString(14, payment.rawPayload);
            insert.setLong(15, payment.sponsorId);
            if (insert.executeUpdate() != 1) {
                return false;
            }
            try (ResultSet keys = insert.getGeneratedKeys()) {
                if (keys.next()) {
                    payment.id = keys.getLong(1);
                }
            }
            return true;
        }
    }

    public synchronized Payment payment(long id) throws SQLException {
        List<Payment> found = payments("WHERE id = ?", id);
        return found.isEmpty() ? null : found.get(0);
    }

    public synchronized Payment paymentByProviderId(String providerId) throws SQLException {
        List<Payment> found = payments("WHERE provider_id = ?", providerId);
        return found.isEmpty() ? null : found.get(0);
    }

    /** Payments received in [from, to), newest first; revoked ones too (the ledger shows them). */
    public synchronized List<Payment> paymentsBetween(long from, long to) throws SQLException {
        return payments("WHERE received_at >= ? AND received_at < ? ORDER BY received_at DESC, id DESC", from, to);
    }

    /** Payments nobody is linked to yet (revoked ones aside), newest first. */
    public synchronized List<Payment> unmatched(int limit) throws SQLException {
        return payments("WHERE account IS NULL AND revoked = 0 ORDER BY received_at DESC, id DESC LIMIT ?", limit);
    }

    public synchronized long unmatchedCount() throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT COUNT(*) FROM payments WHERE account IS NULL AND revoked = 0")) {
            try (ResultSet rows = select.executeQuery()) {
                return rows.next() ? rows.getLong(1) : 0;
            }
        }
    }

    /** The account's payments, oldest first. */
    public synchronized List<Payment> paymentsOf(String account) throws SQLException {
        return payments("WHERE account = ? ORDER BY received_at, id", account);
    }

    /** The account the latest linked, unrevoked payment with this email hash went to, or null. */
    public synchronized String accountOfEmailHash(String emailHash) throws SQLException {
        if (emailHash == null) {
            return null;
        }
        try (PreparedStatement select = connection.prepareStatement("SELECT account FROM payments WHERE email_hash = ?"
                + " AND account IS NOT NULL AND revoked = 0 ORDER BY received_at DESC, id DESC LIMIT 1")) {
            select.setString(1, emailHash);
            try (ResultSet rows = select.executeQuery()) {
                return rows.next() ? rows.getString(1) : null;
            }
        }
    }

    public synchronized void linkPayment(long id, String account, String linkedBy) throws SQLException {
        try (PreparedStatement update = connection.prepareStatement("UPDATE payments SET account = ?, linked_by = ? WHERE id = ?")) {
            update.setString(1, account);
            update.setString(2, linkedBy);
            update.setLong(3, id);
            update.executeUpdate();
        }
    }

    /** A sponsor's months nobody was linked to go to the account they signed in to; returns the ids linked. */
    public synchronized List<Long> linkSponsorPayments(long sponsorId, String account) throws SQLException {
        List<Long> ids = new ArrayList<>();
        for (Payment payment : payments("WHERE sponsor_id = ? AND account IS NULL AND revoked = 0", sponsorId)) {
            ids.add(payment.id);
        }
        try (PreparedStatement update = connection.prepareStatement("UPDATE payments SET account = ?, linked_by = 'auto'"
                + " WHERE sponsor_id = ? AND account IS NULL AND revoked = 0")) {
            update.setString(1, account);
            update.setLong(2, sponsorId);
            update.executeUpdate();
        }
        return ids;
    }

    public synchronized void revokePayment(long id, String reason) throws SQLException {
        try (PreparedStatement update = connection.prepareStatement("UPDATE payments SET revoked = 1, revoked_reason = ? WHERE id = ?")) {
            update.setString(1, reason);
            update.setLong(2, id);
            update.executeUpdate();
        }
    }

    /** Drops the providers' raw payloads received before {@code before}; returns how many. */
    public synchronized int dropPayloadsBefore(long before) throws SQLException {
        try (PreparedStatement update = connection.prepareStatement("UPDATE payments SET raw_payload = NULL"
                + " WHERE raw_payload IS NOT NULL AND received_at < ?")) {
            update.setLong(1, before);
            return update.executeUpdate();
        }
    }

    public synchronized String rawPayload(long id) throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT raw_payload FROM payments WHERE id = ?")) {
            select.setLong(1, id);
            try (ResultSet rows = select.executeQuery()) {
                return rows.next() ? rows.getString(1) : null;
            }
        }
    }

    private List<Payment> payments(String where, Object... args) throws SQLException {
        List<Payment> payments = new ArrayList<>();
        try (PreparedStatement select = connection.prepareStatement("SELECT " + PAYMENT_COLUMNS + " FROM payments " + where)) {
            bind(select, args);
            try (ResultSet rows = select.executeQuery()) {
                while (rows.next()) {
                    Payment payment = new Payment();
                    payment.id = rows.getLong(1);
                    payment.provider = rows.getString(2);
                    payment.providerId = rows.getString(3);
                    payment.kind = rows.getString(4);
                    payment.amountCents = rows.getLong(5);
                    payment.currency = rows.getString(6);
                    payment.tier = rows.getString(7);
                    payment.fromName = rows.getString(8);
                    payment.message = rows.getString(9);
                    payment.isPublic = rows.getInt(10) != 0;
                    payment.emailHash = rows.getString(11);
                    payment.account = rows.getString(12);
                    payment.linkedBy = rows.getString(13);
                    payment.receivedAt = rows.getLong(14);
                    payment.rawPayload = rows.getString(15);
                    payment.revoked = rows.getInt(16) != 0;
                    payment.revokedReason = rows.getString(17);
                    payment.sponsorId = rows.getLong(18);
                    payments.add(payment);
                }
            }
        }
        return payments;
    }

    private static void bind(PreparedStatement statement, Object... args) throws SQLException {
        for (int i = 0; i < args.length; i++) {
            Object arg = args[i];
            if (arg == null) {
                statement.setNull(i + 1, Types.VARCHAR);
            } else if (arg instanceof Long || arg instanceof Integer) {
                statement.setLong(i + 1, ((Number) arg).longValue());
            } else {
                statement.setString(i + 1, arg.toString());
            }
        }
    }

    // ---- support codes

    public synchronized String codeOf(String account) throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT code FROM codes WHERE account = ?")) {
            select.setString(1, account);
            try (ResultSet rows = select.executeQuery()) {
                return rows.next() ? rows.getString(1) : null;
            }
        }
    }

    public synchronized String accountOfCode(String code) throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT account FROM codes WHERE code = ?")) {
            select.setString(1, code);
            try (ResultSet rows = select.executeQuery()) {
                return rows.next() ? rows.getString(1) : null;
            }
        }
    }

    /** False when the code belongs to another account already. */
    public synchronized boolean addCode(String account, String code, long now) throws SQLException {
        try (PreparedStatement insert = connection.prepareStatement("INSERT OR IGNORE INTO codes (account, code, created_at) VALUES (?, ?, ?)")) {
            insert.setString(1, account);
            insert.setString(2, code);
            insert.setLong(3, now);
            return insert.executeUpdate() == 1;
        }
    }

    // ---- perks

    /** One owned perk: kind, item, when it was granted, and the payment that granted it (0: staff's grant). */
    public static final class OwnedPerk {
        public final String kind;
        public final String item;
        public final long grantedAt;
        public final long paymentId;

        OwnedPerk(String kind, String item, long grantedAt, long paymentId) {
            this.kind = kind;
            this.item = item;
            this.grantedAt = grantedAt;
            this.paymentId = paymentId;
        }
    }

    public synchronized List<OwnedPerk> perks(String account) throws SQLException {
        List<OwnedPerk> perks = new ArrayList<>();
        try (PreparedStatement select = connection.prepareStatement("SELECT kind, item, granted_at, payment_id FROM perks"
                + " WHERE account = ? ORDER BY granted_at, kind, item")) {
            select.setString(1, account);
            try (ResultSet rows = select.executeQuery()) {
                while (rows.next()) {
                    perks.add(new OwnedPerk(rows.getString(1), rows.getString(2), rows.getLong(3), rows.getLong(4)));
                }
            }
        }
        return perks;
    }

    /** @return true when the account didn't own it yet */
    public synchronized boolean addPerk(String account, String kind, String item, long now, long paymentId) throws SQLException {
        try (PreparedStatement insert = connection.prepareStatement("INSERT OR IGNORE INTO perks (account, kind, item, granted_at, payment_id)"
                + " VALUES (?, ?, ?, ?, ?)")) {
            insert.setString(1, account);
            insert.setString(2, kind);
            insert.setString(3, item);
            insert.setLong(4, now);
            insert.setLong(5, paymentId);
            return insert.executeUpdate() == 1;
        }
    }

    /** Takes back the perks one payment granted; returns how many. */
    public synchronized int removePerksOf(long paymentId) throws SQLException {
        try (PreparedStatement delete = connection.prepareStatement("DELETE FROM perks WHERE payment_id = ?")) {
            delete.setLong(1, paymentId);
            return delete.executeUpdate();
        }
    }

    // ---- the thank-you shown on the next visit

    public synchronized void setNotice(String account, long paymentId, long now) throws SQLException {
        try (PreparedStatement upsert = connection.prepareStatement("INSERT OR REPLACE INTO notices (account, payment_id, at) VALUES (?, ?, ?)")) {
            upsert.setString(1, account);
            upsert.setLong(2, paymentId);
            upsert.setLong(3, now);
            upsert.executeUpdate();
        }
    }

    public synchronized boolean hasNotice(String account) throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT 1 FROM notices WHERE account = ?")) {
            select.setString(1, account);
            try (ResultSet rows = select.executeQuery()) {
                return rows.next();
            }
        }
    }

    /** @return whether there was one */
    public synchronized boolean clearNotice(String account) throws SQLException {
        try (PreparedStatement delete = connection.prepareStatement("DELETE FROM notices WHERE account = ?")) {
            delete.setString(1, account);
            return delete.executeUpdate() > 0;
        }
    }

    // ---- GitHub sponsorships

    public synchronized Sponsorship sponsorship(long githubId) throws SQLException {
        List<Sponsorship> found = sponsorships("WHERE github_id = ?", githubId);
        return found.isEmpty() ? null : found.get(0);
    }

    public synchronized List<Sponsorship> sponsorships() throws SQLException {
        return sponsorships("ORDER BY since");
    }

    public synchronized List<Sponsorship> sponsorshipsOf(String account) throws SQLException {
        return sponsorships("WHERE account = ? ORDER BY since", account);
    }

    public synchronized void putSponsorship(Sponsorship sponsorship, long now) throws SQLException {
        try (PreparedStatement upsert = connection.prepareStatement("INSERT OR REPLACE INTO sponsorships (github_id, login, node_id,"
                + " monthly_cents, tier, is_public, active, since, until, account, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")) {
            upsert.setLong(1, sponsorship.githubId);
            upsert.setString(2, sponsorship.login);
            upsert.setString(3, sponsorship.nodeId);
            upsert.setLong(4, sponsorship.monthlyCents);
            upsert.setString(5, sponsorship.tier);
            upsert.setInt(6, sponsorship.isPublic ? 1 : 0);
            upsert.setInt(7, sponsorship.active ? 1 : 0);
            upsert.setLong(8, sponsorship.since);
            upsert.setLong(9, sponsorship.until);
            upsert.setString(10, sponsorship.account);
            upsert.setLong(11, now);
            upsert.executeUpdate();
        }
    }

    private List<Sponsorship> sponsorships(String where, Object... args) throws SQLException {
        List<Sponsorship> list = new ArrayList<>();
        try (PreparedStatement select = connection.prepareStatement("SELECT github_id, login, node_id, monthly_cents, tier, is_public,"
                + " active, since, until, account FROM sponsorships " + where)) {
            bind(select, args);
            try (ResultSet rows = select.executeQuery()) {
                while (rows.next()) {
                    Sponsorship sponsorship = new Sponsorship();
                    sponsorship.githubId = rows.getLong(1);
                    sponsorship.login = rows.getString(2);
                    sponsorship.nodeId = rows.getString(3);
                    sponsorship.monthlyCents = rows.getLong(4);
                    sponsorship.tier = rows.getString(5);
                    sponsorship.isPublic = rows.getInt(6) != 0;
                    sponsorship.active = rows.getInt(7) != 0;
                    sponsorship.since = rows.getLong(8);
                    sponsorship.until = rows.getLong(9);
                    sponsorship.account = rows.getString(10);
                    list.add(sponsorship);
                }
            }
        }
        return list;
    }

    // ---- looks

    public synchronized PlayerLook look(String account) throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT account, sleeve, cloth, avatar, frame, title, flair, pet FROM looks WHERE account = ?")) {
            select.setString(1, account);
            try (ResultSet rows = select.executeQuery()) {
                if (!rows.next()) {
                    return null;
                }
                PlayerLook look = new PlayerLook();
                look.name = rows.getString(1);
                look.sleeve = rows.getString(2);
                look.cloth = rows.getString(3);
                look.avatar = rows.getString(4);
                look.frame = rows.getString(5);
                look.title = rows.getString(6);
                look.flair = rows.getString(7);
                look.pet = rows.getString(8);
                return look;
            }
        }
    }

    public synchronized void putLook(String account, PlayerLook look, long now) throws SQLException {
        try (PreparedStatement upsert = connection.prepareStatement("INSERT OR REPLACE INTO looks (account, sleeve, cloth, avatar, frame,"
                + " title, flair, pet, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")) {
            upsert.setString(1, account);
            upsert.setString(2, look.sleeve);
            upsert.setString(3, look.cloth);
            upsert.setString(4, look.avatar);
            upsert.setString(5, look.frame);
            upsert.setString(6, look.title);
            upsert.setString(7, look.flair);
            upsert.setString(8, look.pet);
            upsert.setLong(9, now);
            upsert.executeUpdate();
        }
    }

    // ---- the supporters wall

    public synchronized boolean onWall(String account) throws SQLException {
        try (PreparedStatement select = connection.prepareStatement("SELECT wall FROM prefs WHERE account = ?")) {
            select.setString(1, account);
            try (ResultSet rows = select.executeQuery()) {
                return rows.next() && rows.getInt(1) != 0;
            }
        }
    }

    public synchronized void setWall(String account, boolean wall) throws SQLException {
        try (PreparedStatement upsert = connection.prepareStatement("INSERT OR REPLACE INTO prefs (account, wall) VALUES (?, ?)")) {
            upsert.setString(1, account);
            upsert.setInt(2, wall ? 1 : 0);
            upsert.executeUpdate();
        }
    }

    // ---- deleting an account

    /**
     * Forgets the account: its payments stay in the ledger (the months' totals don't change) but lose everything that
     * names the payer; its code, perks, look, wall choice and pending thank-you go.
     */
    public synchronized void forget(String account) throws SQLException {
        transaction(() -> {
            try (PreparedStatement update = connection.prepareStatement("UPDATE payments SET account = NULL, linked_by = NULL,"
                    + " from_name = NULL, message = NULL, email_hash = NULL, raw_payload = NULL, is_public = 0 WHERE account = ?")) {
                update.setString(1, account);
                update.executeUpdate();
            }
            try (PreparedStatement update = connection.prepareStatement("UPDATE sponsorships SET account = NULL WHERE account = ?")) {
                update.setString(1, account);
                update.executeUpdate();
            }
            for (String table : new String[]{"codes", "perks", "notices", "looks", "prefs"}) {
                try (PreparedStatement delete = connection.prepareStatement("DELETE FROM " + table + " WHERE account = ?")) {
                    delete.setString(1, account);
                    delete.executeUpdate();
                }
            }
            return null;
        });
    }

    @Override
    public synchronized void close() throws SQLException {
        connection.close();
    }
}
