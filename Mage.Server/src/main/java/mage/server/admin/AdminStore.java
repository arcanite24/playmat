package mage.server.admin;

import com.j256.ormlite.dao.Dao;
import com.j256.ormlite.dao.DaoManager;
import com.j256.ormlite.jdbc.JdbcConnectionSource;
import com.j256.ormlite.stmt.DeleteBuilder;
import com.j256.ormlite.stmt.QueryBuilder;
import com.j256.ormlite.stmt.SelectArg;
import com.j256.ormlite.stmt.Where;
import com.j256.ormlite.support.ConnectionSource;
import com.j256.ormlite.table.TableUtils;
import org.apache.log4j.Logger;

import java.io.File;
import java.security.SecureRandom;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;
import java.util.Locale;
import java.util.concurrent.TimeUnit;

/**
 * What /admin keeps across restarts: the audit log, settings, the allowlist, invites and access requests, and
 * which accounts verified their email. Stored in db/web_admin.db (SQLite).
 */
public final class AdminStore {

    private static final Logger logger = Logger.getLogger(AdminStore.class);

    private static final String DEFAULT_URL = "jdbc:sqlite:./db/web_admin.db";
    public static final int MAX_NOTE = 300;
    public static final int MAX_REASON = 500;
    public static final int MAX_INVITE_USES = 100;
    public static final int MAX_INVITE_DAYS = 90;
    /** open access requests kept at a time, so the queue can't be flooded */
    public static final int MAX_OPEN_REQUESTS = 200;

    private static volatile AdminStore instance;
    private static final SecureRandom random = new SecureRandom();

    private final Dao<AuditEntry, Long> audit;
    private final Dao<AdminSetting, String> settings;
    private final Dao<AllowEntry, Long> allowlist;
    private final Dao<Invite, String> invites;
    private final Dao<AccessRequest, Long> requests;
    private final Dao<GithubLink, Long> githubLinks;
    private final Dao<StaffMember, String> staff;
    private final Dao<FeedbackEntry, Long> feedback;
    private final Dao<ClientErrorGroup, Long> clientErrors;
    private final Dao<AccountEmail, String> accountEmails;

    public AdminStore(String jdbcUrl) throws SQLException {
        ConnectionSource source = new JdbcConnectionSource(jdbcUrl);
        TableUtils.createTableIfNotExists(source, AuditEntry.class);
        TableUtils.createTableIfNotExists(source, AdminSetting.class);
        TableUtils.createTableIfNotExists(source, AllowEntry.class);
        TableUtils.createTableIfNotExists(source, Invite.class);
        TableUtils.createTableIfNotExists(source, AccessRequest.class);
        TableUtils.createTableIfNotExists(source, GithubLink.class);
        TableUtils.createTableIfNotExists(source, StaffMember.class);
        TableUtils.createTableIfNotExists(source, FeedbackEntry.class);
        TableUtils.createTableIfNotExists(source, ClientErrorGroup.class);
        TableUtils.createTableIfNotExists(source, AccountEmail.class);
        audit = DaoManager.createDao(source, AuditEntry.class);
        settings = DaoManager.createDao(source, AdminSetting.class);
        allowlist = DaoManager.createDao(source, AllowEntry.class);
        invites = DaoManager.createDao(source, Invite.class);
        requests = DaoManager.createDao(source, AccessRequest.class);
        githubLinks = DaoManager.createDao(source, GithubLink.class);
        staff = DaoManager.createDao(source, StaffMember.class);
        feedback = DaoManager.createDao(source, FeedbackEntry.class);
        clientErrors = DaoManager.createDao(source, ClientErrorGroup.class);
        accountEmails = DaoManager.createDao(source, AccountEmail.class);
    }

    public static AdminStore get() {
        AdminStore current = instance;
        if (current == null) {
            synchronized (AdminStore.class) {
                current = instance;
                if (current == null) {
                    new File("db").mkdirs();
                    try {
                        current = new AdminStore(DEFAULT_URL);
                    } catch (SQLException | LinkageError e) {
                        logger.error("Admin database can't be opened", e);
                        throw new IllegalStateException("The admin database can't be opened");
                    }
                    instance = current;
                }
            }
        }
        return current;
    }

    // --- audit ------------------------------------------------------------------------------------

    public synchronized AuditEntry audit(String actor, String action, String target, String detail, String reason, long now) {
        AuditEntry entry = new AuditEntry();
        entry.at = now;
        entry.actor = actor == null ? "" : actor;
        entry.action = action;
        entry.target = target == null ? "" : target;
        entry.detail = cut(detail, 1000);
        entry.reason = cut(reason, MAX_REASON);
        try {
            audit.create(entry);
        } catch (SQLException e) {
            // the action already happened; the log line keeps the trail
            logger.error("Audit entry not stored: " + entry.actor + ' ' + entry.action + ' ' + entry.target, e);
        }
        logger.info("Audit: " + entry.actor + ' ' + entry.action + (entry.target.isEmpty() ? "" : ' ' + entry.target)
                + (entry.reason == null || entry.reason.isEmpty() ? "" : " (" + entry.reason + ')'));
        return entry;
    }

    /**
     * Newest first, optionally only one actor's, one action family's ("access" matches "access.mode") or about one target.
     */
    public synchronized List<AuditEntry> auditLog(String actor, String action, String target, long before, int limit) throws SQLException {
        QueryBuilder<AuditEntry, Long> query = audit.queryBuilder().orderBy("id", false).limit((long) clamp(limit, 1, 500));
        Where<AuditEntry, Long> where = query.where().gt("id", 0L);
        if (!isBlank(actor)) {
            where.and().eq("actor", new SelectArg(actor.trim()));
        }
        if (!isBlank(action)) {
            where.and().like("action", new SelectArg(action.trim() + "%"));
        }
        if (!isBlank(target)) {
            where.and().eq("target", new SelectArg(target.trim()));
        }
        if (before > 0) {
            where.and().lt("id", before);
        }
        return query.query();
    }

    // --- staff ------------------------------------------------------------------------------------

    public synchronized List<StaffMember> staff() throws SQLException {
        return staff.queryBuilder().orderBy("name", true).query();
    }

    /** the account's role, or null when it isn't staff */
    public synchronized StaffRole staffRole(String userName) throws SQLException {
        if (isBlank(userName)) {
            return null;
        }
        StaffMember member = staff.queryForId(userName.trim().toLowerCase(Locale.ROOT));
        return member == null ? null : StaffRole.parse(member.role);
    }

    public synchronized StaffMember grantStaff(String userName, StaffRole role, String by, long now) throws SQLException {
        StaffMember member = new StaffMember();
        member.key = userName.trim().toLowerCase(Locale.ROOT);
        member.name = userName.trim();
        member.role = role.id();
        member.grantedBy = by;
        member.grantedAt = now;
        staff.createOrUpdate(member);
        return member;
    }

    /** false when the account wasn't staff */
    public synchronized boolean revokeStaff(String userName) throws SQLException {
        return !isBlank(userName) && staff.deleteById(userName.trim().toLowerCase(Locale.ROOT)) > 0;
    }

    // --- client errors ----------------------------------------------------------------------------

    /** kinds of error kept at a time; the ones seen longest ago go first */
    public static final int MAX_CLIENT_ERRORS = 500;

    /**
     * Counts one report into its group, which takes the report's details as the latest; a resolved group opens again.
     *
     * @return the group, and whether it is new (count 1) or came back (regressed)
     */
    public synchronized ClientErrorGroup recordClientError(ClientErrorGroup report, long now) throws SQLException {
        ClientErrorGroup group = clientErrors.queryBuilder().where().eq("fingerprint", new SelectArg(report.fingerprint)).queryForFirst();
        if (group == null) {
            report.count = 1;
            report.firstSeen = now;
            report.lastSeen = now;
            clientErrors.create(report);
            if (clientErrors.countOf() > MAX_CLIENT_ERRORS) {
                ClientErrorGroup oldest = clientErrors.queryBuilder().orderBy("lastSeen", true).limit(1L).queryForFirst();
                if (oldest != null && oldest.id != report.id) {
                    clientErrors.delete(oldest);
                }
            }
            return report;
        }
        group.count++;
        group.lastSeen = now;
        group.message = report.message;
        group.path = report.path;
        group.stack = report.stack;
        group.appVersion = report.appVersion;
        group.userAgent = report.userAgent;
        group.context = report.context;
        group.lastUser = report.lastUser;
        if (group.resolved) {
            group.resolved = false;
            group.regressed = true;
        }
        clientErrors.update(group);
        return group;
    }

    /** Seen most recently first. */
    public synchronized List<ClientErrorGroup> clientErrors(boolean includeResolved, int limit) throws SQLException {
        QueryBuilder<ClientErrorGroup, Long> query = clientErrors.queryBuilder().orderBy("lastSeen", false).limit((long) clamp(limit, 1, 500));
        if (!includeResolved) {
            query.where().eq("resolved", false);
        }
        return query.query();
    }

    /**
     * Marks a kind of error fixed (or, with {@code resolved} false, open again).
     */
    public synchronized ClientErrorGroup resolveClientError(long id, boolean resolved, String by, long now) throws SQLException {
        ClientErrorGroup group = clientErrors.queryForId(id);
        if (group == null) {
            throw new IllegalArgumentException("There is no client error " + id);
        }
        group.resolved = resolved;
        group.resolvedBy = resolved ? by : null;
        group.resolvedAt = resolved ? now : 0;
        if (resolved) {
            group.regressed = false;
        }
        clientErrors.update(group);
        return group;
    }

    // --- feedback ---------------------------------------------------------------------------------

    public static final int MAX_FEEDBACK_MESSAGE = 4000;
    /** kept at a time; the oldest handled ones go first */
    public static final int MAX_FEEDBACK = 2000;

    public synchronized FeedbackEntry addFeedback(String userName, String type, String title, String message, String context, long now) throws SQLException {
        if (isBlank(message)) {
            throw new IllegalArgumentException("Write something first.");
        }
        FeedbackEntry entry = new FeedbackEntry();
        entry.createdAt = now;
        entry.userName = cut(userName, 40);
        entry.type = "bug".equals(type) || "idea".equals(type) ? type : "other";
        entry.title = cut(isBlank(title) ? firstLine(message) : title.trim(), 200);
        entry.message = cut(message.trim(), MAX_FEEDBACK_MESSAGE);
        entry.context = cut(context, 300);
        entry.status = FeedbackEntry.NEW;
        feedback.create(entry);
        if (feedback.countOf() > MAX_FEEDBACK) {
            QueryBuilder<FeedbackEntry, Long> oldest = feedback.queryBuilder().orderBy("id", true).limit(1L);
            oldest.where().ne("status", FeedbackEntry.NEW);
            FeedbackEntry drop = oldest.queryForFirst();
            if (drop != null) {
                feedback.delete(drop);
            }
        }
        return entry;
    }

    /** Newest first; status null for all. */
    public synchronized List<FeedbackEntry> feedback(String status, int limit) throws SQLException {
        QueryBuilder<FeedbackEntry, Long> query = feedback.queryBuilder().orderBy("id", false).limit((long) clamp(limit, 1, 500));
        if (!isBlank(status)) {
            query.where().eq("status", new SelectArg(status.trim()));
        }
        return query.query();
    }

    public synchronized long newFeedbackCount() throws SQLException {
        return feedback.queryBuilder().where().eq("status", FeedbackEntry.NEW).countOf();
    }

    public synchronized FeedbackEntry markFeedback(long id, String status, String note, String by, long now) throws SQLException {
        if (!FeedbackEntry.NEW.equals(status) && !FeedbackEntry.SEEN.equals(status) && !FeedbackEntry.DONE.equals(status)) {
            throw new IllegalArgumentException("The status is new, seen or done");
        }
        FeedbackEntry entry = feedback.queryForId(id);
        if (entry == null) {
            throw new IllegalArgumentException("There is no feedback " + id);
        }
        entry.status = status;
        if (note != null) {
            entry.note = cut(note.trim(), MAX_NOTE);
        }
        entry.handledBy = by;
        entry.handledAt = now;
        feedback.update(entry);
        return entry;
    }

    public synchronized List<FeedbackEntry> feedbackFrom(String userName, int limit) throws SQLException {
        return feedback.queryBuilder().orderBy("id", false).limit((long) clamp(limit, 1, 500))
                .where().eq("userName", new SelectArg(userName)).query();
    }

    private static String firstLine(String text) {
        String line = text.trim().split("\\R", 2)[0];
        return line.length() > 80 ? line.substring(0, 80) + "…" : line;
    }

    // --- an account's records ---------------------------------------------------------------------

    /**
     * Everything this store keeps about an account, gone: its allowlist entries by name, staff role, GitHub link,
     * feedback and verified email. The audit log stays: it is the record of what staff did.
     */
    public synchronized void forgetAccount(String userName) throws SQLException {
        String key = userName.trim().toLowerCase(Locale.ROOT);
        DeleteBuilder<AllowEntry, Long> allowed = allowlist.deleteBuilder();
        allowed.where().eq("name", new SelectArg(key));
        allowed.delete();
        staff.deleteById(key);
        DeleteBuilder<GithubLink, Long> links = githubLinks.deleteBuilder();
        links.where().eq("account", new SelectArg(userName.trim()));
        links.delete();
        DeleteBuilder<FeedbackEntry, Long> sent = feedback.deleteBuilder();
        sent.where().eq("userName", new SelectArg(userName.trim()));
        sent.delete();
        accountEmails.deleteById(key);
    }

    // --- email verification ----------------------------------------------------------------------

    /** What is known about the account's email, or null. */
    public synchronized AccountEmail accountEmail(String userName) throws SQLException {
        return isBlank(userName) ? null : accountEmails.queryForId(userName.trim().toLowerCase(Locale.ROOT));
    }

    /** The player proved they read this address. */
    public synchronized AccountEmail emailVerified(String userName, String email, long now) throws SQLException {
        AccountEmail row = accountEmailRow(userName);
        row.verifiedEmail = email == null ? null : email.trim();
        row.verifiedAt = row.verifiedEmail == null ? 0 : now;
        accountEmails.createOrUpdate(row);
        return row;
    }

    /** Forgets the verified address, as when the account's email changes. */
    public synchronized void emailUnverified(String userName) throws SQLException {
        AccountEmail row = accountEmail(userName);
        if (row != null && row.verifiedEmail != null) {
            row.verifiedEmail = null;
            row.verifiedAt = 0;
            accountEmails.update(row);
        }
    }

    public synchronized void accountRegistered(String userName, long now) throws SQLException {
        AccountEmail row = accountEmailRow(userName);
        row.registeredAt = now;
        accountEmails.createOrUpdate(row);
    }

    private AccountEmail accountEmailRow(String userName) throws SQLException {
        AccountEmail row = accountEmail(userName);
        if (row == null) {
            row = new AccountEmail();
            row.key = userName.trim().toLowerCase(Locale.ROOT);
        }
        row.name = userName.trim();
        return row;
    }

    // --- settings ---------------------------------------------------------------------------------

    public synchronized String setting(String name) throws SQLException {
        AdminSetting setting = settings.queryForId(name);
        return setting == null ? null : setting.value;
    }

    public synchronized void putSetting(String name, String value, String by, long now) throws SQLException {
        AdminSetting setting = new AdminSetting();
        setting.name = name;
        setting.value = value;
        setting.changedBy = by;
        setting.changedAt = now;
        settings.createOrUpdate(setting);
    }

    public synchronized AdminSetting settingRow(String name) throws SQLException {
        return settings.queryForId(name);
    }

    // --- allowlist --------------------------------------------------------------------------------

    public synchronized List<AllowEntry> allowlist() throws SQLException {
        return allowlist.queryBuilder().orderBy("addedAt", false).query();
    }

    /**
     * Adds an account name, or an email when the text has an @; returns the existing entry when it is listed already.
     */
    public synchronized AllowEntry allow(String nameOrEmail, String note, String by, long now) throws SQLException {
        String who = nameOrEmail == null ? "" : nameOrEmail.trim();
        if (who.isEmpty()) {
            throw new IllegalArgumentException("Type an account name or an email address");
        }
        if (who.length() > 254) {
            throw new IllegalArgumentException("That is too long for a name or an email address");
        }
        boolean email = who.contains("@");
        String key = who.toLowerCase(Locale.ROOT);
        List<AllowEntry> existing = allowlist.queryBuilder().where().eq(email ? "email" : "name", new SelectArg(key)).query();
        if (!existing.isEmpty()) {
            return existing.get(0);
        }
        AllowEntry entry = new AllowEntry();
        if (email) {
            entry.email = key;
        } else {
            entry.name = key;
        }
        entry.note = cut(note, MAX_NOTE);
        entry.addedBy = by;
        entry.addedAt = now;
        allowlist.create(entry);
        return entry;
    }

    public synchronized AllowEntry allowEntry(long id) throws SQLException {
        return allowlist.queryForId(id);
    }

    public synchronized AllowEntry disallow(long id) throws SQLException {
        AllowEntry entry = allowlist.queryForId(id);
        if (entry != null) {
            allowlist.deleteById(id);
        }
        return entry;
    }

    /**
     * Whether an entry names this account or its email.
     */
    public synchronized boolean isListed(String name, String email) throws SQLException {
        return listedCount(name, email) > 0;
    }

    /**
     * How many entries name this account or its email.
     */
    public synchronized long listedCount(String name, String email) throws SQLException {
        Where<AllowEntry, Long> where = allowlist.queryBuilder().where();
        int clauses = 0;
        if (!isBlank(name)) {
            where.eq("name", new SelectArg(name.trim().toLowerCase(Locale.ROOT)));
            clauses++;
        }
        if (!isBlank(email)) {
            where.eq("email", new SelectArg(email.trim().toLowerCase(Locale.ROOT)));
            clauses++;
        }
        if (clauses == 0) {
            return 0;
        }
        if (clauses == 2) {
            where.or(2);
        }
        return where.countOf();
    }

    // --- invites ----------------------------------------------------------------------------------

    public synchronized Invite createInvite(String note, int maxUses, int days, String by, long now) throws SQLException {
        Invite invite = new Invite();
        byte[] bytes = new byte[12];
        random.nextBytes(bytes);
        invite.code = Base64.getUrlEncoder().withoutPadding().encodeToString(bytes);
        invite.note = cut(note, MAX_NOTE);
        invite.maxUses = clamp(maxUses, 1, MAX_INVITE_USES);
        invite.createdBy = by;
        invite.createdAt = now;
        invite.expiresAt = now + TimeUnit.DAYS.toMillis(clamp(days, 1, MAX_INVITE_DAYS));
        invite.usedBy = "";
        invites.create(invite);
        return invite;
    }

    public synchronized List<Invite> invites(int limit) throws SQLException {
        return invites.queryBuilder().orderBy("createdAt", false).limit((long) clamp(limit, 1, 500)).query();
    }

    public synchronized Invite invite(String code) throws SQLException {
        return isBlank(code) ? null : invites.queryForId(code.trim());
    }

    public synchronized boolean revokeInvite(String code) throws SQLException {
        Invite invite = invite(code);
        if (invite == null || invite.revoked) {
            return false;
        }
        invite.revoked = true;
        invites.update(invite);
        return true;
    }

    /**
     * Uses the invite up once for this account; false when it can't be used (any more).
     */
    public synchronized boolean redeemInvite(String code, String userName, long now) throws SQLException {
        Invite invite = invite(code);
        if (invite == null || !invite.usable(now)) {
            return false;
        }
        invite.uses++;
        invite.usedBy = cut(isBlank(invite.usedBy) ? userName : invite.usedBy + ", " + userName, 2000);
        invites.update(invite);
        return true;
    }

    // --- access requests --------------------------------------------------------------------------

    public synchronized AccessRequest requestAccess(String name, String email, String note, long now) throws SQLException {
        String who = name == null ? "" : name.trim();
        String address = email == null ? "" : email.trim().toLowerCase(Locale.ROOT);
        if (who.isEmpty() || who.length() > 64) {
            throw new IllegalArgumentException("Type the account name you'd like");
        }
        if (address.length() > 254 || !address.matches("[^@\\s]+@[^@\\s]+\\.[^@\\s]+")) {
            throw new IllegalArgumentException("Type an email address the server's staff can reach you at");
        }
        List<AccessRequest> open = requests.queryBuilder().where().eq("status", AccessRequest.OPEN).and()
                .eq("email", new SelectArg(address)).query();
        if (!open.isEmpty()) {
            return open.get(0); // asked already: one request per address waits at a time
        }
        if (requests.queryBuilder().where().eq("status", AccessRequest.OPEN).countOf() >= MAX_OPEN_REQUESTS) {
            throw new IllegalStateException("Too many requests are waiting already. Try again in a few days.");
        }
        AccessRequest request = new AccessRequest();
        request.name = who;
        request.email = address;
        request.note = cut(note, MAX_REASON);
        request.createdAt = now;
        requests.create(request);
        return request;
    }

    /**
     * Open requests first, then the latest decided ones.
     */
    public synchronized List<AccessRequest> accessRequests(int limit) throws SQLException {
        List<AccessRequest> result = new ArrayList<>(requests.queryBuilder().orderBy("createdAt", false)
                .where().eq("status", AccessRequest.OPEN).query());
        result.addAll(requests.queryBuilder().orderBy("decidedAt", false).limit((long) clamp(limit, 1, 500))
                .where().ne("status", AccessRequest.OPEN).query());
        return result;
    }

    public synchronized AccessRequest decide(long id, boolean approve, String by, long now) throws SQLException {
        AccessRequest request = requests.queryForId(id);
        if (request == null || !AccessRequest.OPEN.equals(request.status)) {
            return null;
        }
        request.status = approve ? AccessRequest.APPROVED : AccessRequest.DECLINED;
        request.decidedBy = by;
        request.decidedAt = now;
        requests.update(request);
        return request;
    }

    // --- GitHub sign-in ---------------------------------------------------------------------------

    public synchronized GithubLink githubLink(long githubId) throws SQLException {
        return githubLinks.queryForId(githubId);
    }

    public synchronized GithubLink githubLinkOf(String account) throws SQLException {
        if (isBlank(account)) {
            return null;
        }
        List<GithubLink> links = githubLinks.queryBuilder().where().eq("account", new SelectArg(account.trim())).query();
        return links.isEmpty() ? null : links.get(0);
    }

    public synchronized GithubLink linkGithub(long githubId, String login, String account, long now) throws SQLException {
        GithubLink link = new GithubLink();
        link.githubId = githubId;
        link.login = login;
        link.account = account;
        link.linkedAt = now;
        githubLinks.createOrUpdate(link);
        return link;
    }

    // --- helpers ----------------------------------------------------------------------------------

    static String cut(String text, int max) {
        if (text == null) {
            return null;
        }
        String trimmed = text.trim();
        return trimmed.length() > max ? trimmed.substring(0, max) : trimmed;
    }

    static int clamp(int value, int min, int max) {
        return Math.max(min, Math.min(max, value));
    }

    static boolean isBlank(String text) {
        return text == null || text.trim().isEmpty();
    }
}
