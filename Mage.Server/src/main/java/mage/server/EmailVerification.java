package mage.server;

import mage.server.admin.AccessPolicy;
import mage.server.admin.AccountEmail;
import mage.server.admin.AdminStore;
import mage.server.managers.ManagerFactory;
import org.apache.log4j.Logger;

import java.sql.SQLException;
import java.util.ArrayDeque;
import java.util.Date;
import java.util.Deque;
import java.util.HashMap;
import java.util.Locale;
import java.util.Map;
import java.util.concurrent.TimeUnit;

/**
 * Whether a player reads their account's email: a six-digit code sent there and typed back (or a password reset by a
 * mailed code, or GitHub's verified address). Kept in web_admin.db with the address it proved, so an account counts
 * as verified only while its email is still that address. Unverified accounts play everything; verification is what
 * account recovery and supporter perks rely on.
 * <p>
 * Codes work for 30 minutes and 5 tries, like password reset codes. One account gets a code at most once a minute and
 * {@link #SENDS_PER_HOUR} times an hour. Codes and that count are in memory: a restart cancels them.
 */
public final class EmailVerification {

    private static final Logger logger = Logger.getLogger(EmailVerification.class);

    public static final long RESEND_MILLIS = TimeUnit.MINUTES.toMillis(1);
    public static final int SENDS_PER_HOUR = 5;
    private static final long HOUR = TimeUnit.HOURS.toMillis(1);
    /** an unverified address is held this long, by an account that hasn't signed in for as long, before another may take it */
    public static final long SQUAT_MILLIS = TimeUnit.HOURS.toMillis(48);

    private static final EmailVerification INSTANCE = new EmailVerification();

    private final ResetTokens codes = new ResetTokens();
    private final Map<String, Deque<Long>> sends = new HashMap<>();

    EmailVerification() {
    }

    public static EmailVerification get() {
        return INSTANCE;
    }

    // --- what other code asks ---------------------------------------------------------------------

    /**
     * Whether the account proved it reads this address. False for any other address, so an account whose email
     * changed is unverified until it proves the new one.
     */
    public static boolean isVerified(String userName, String email) {
        if (isBlank(userName) || isBlank(email)) {
            return false;
        }
        try {
            AccountEmail row = store().accountEmail(userName);
            return row != null && row.verifiedEmail != null && row.verifiedEmail.equalsIgnoreCase(email.trim());
        } catch (SQLException | IllegalStateException e) {
            logger.warn("Email verification can't be read for " + userName + ": " + e.getMessage());
            return false;
        }
    }

    /** Whether the account's current email is verified. */
    public static boolean isVerified(String userName) {
        AuthorizedUser account = isBlank(userName) ? null : AuthorizedUserRepository.getInstance().getByName(userName.trim());
        return account != null && isVerified(account.getName(), account.getEmail());
    }

    /** The player proved they read this address (a mailed code, or GitHub vouching for it). */
    public static void markVerified(String userName, String email, long now) {
        if (isBlank(userName) || isBlank(email)) {
            return;
        }
        try {
            store().emailVerified(userName, email, now);
            logger.info("Email verified for " + userName);
        } catch (SQLException | IllegalStateException e) {
            logger.error("Email verification can't be stored for " + userName, e);
        }
    }

    /** A new account: when it was made decides how long an unverified address stays its own. */
    public static void registered(String userName, long now) {
        try {
            store().accountRegistered(userName, now);
        } catch (SQLException | IllegalStateException e) {
            logger.warn("Registration time can't be stored for " + userName + ": " + e.getMessage());
        }
    }

    // --- codes ------------------------------------------------------------------------------------

    /**
     * A new code for the account's address, replacing any earlier one.
     *
     * @throws IllegalStateException when the account asked too often
     */
    synchronized String issue(String userName, String email, long now) {
        String account = userName.trim().toLowerCase(Locale.ROOT);
        sends.values().removeIf(times -> times.isEmpty() || now - times.peekLast() >= HOUR);
        Deque<Long> times = sends.computeIfAbsent(account, a -> new ArrayDeque<>());
        while (!times.isEmpty() && now - times.peekFirst() >= HOUR) {
            times.pollFirst();
        }
        if (!times.isEmpty() && now - times.peekLast() < RESEND_MILLIS) {
            throw new IllegalStateException("A code is on its way. Wait a minute before asking for another.");
        }
        if (times.size() >= SENDS_PER_HOUR) {
            throw new IllegalStateException("That's " + SENDS_PER_HOUR + " codes this hour. Use the latest one, or try again later.");
        }
        times.addLast(now);
        return codes.issue(codeKey(userName, email), now);
    }

    /**
     * Checks a code for the account's current address, using up one try; a right one verifies the address.
     */
    public boolean verify(String userName, String code, long now) {
        AuthorizedUser account = isBlank(userName) ? null : AuthorizedUserRepository.getInstance().getByName(userName.trim());
        if (account == null || isBlank(account.getEmail())) {
            return false;
        }
        // the code was made for the address it went to: if the email changed since, it no longer matches
        if (!codes.redeem(codeKey(account.getName(), account.getEmail()), code, now)) {
            logger.info("Wrong or expired email code for " + account.getName());
            return false;
        }
        markVerified(account.getName(), account.getEmail(), now);
        return true;
    }

    /**
     * Mails a code to the account's address.
     *
     * @return null when it went out, or why not
     */
    public String sendCode(ManagerFactory managers, String userName, long now) {
        AuthorizedUser account = isBlank(userName) ? null : AuthorizedUserRepository.getInstance().getByName(userName.trim());
        if (account == null) {
            return "Only accounts have an email address to confirm.";
        }
        if (isBlank(account.getEmail())) {
            return "Your account has no email address. Ask the server's staff to add one.";
        }
        if (isVerified(account.getName(), account.getEmail())) {
            return "Your email address is confirmed already.";
        }
        if (!Mailer.configured(managers.configSettings())) {
            return "This server can't send email, so addresses can't be confirmed here.";
        }
        String code;
        try {
            code = issue(account.getName(), account.getEmail(), now);
        } catch (IllegalStateException e) {
            return e.getMessage();
        }
        if (!Mailer.send(managers, account.getEmail(), "Confirm your email" + serverSuffix(managers), codeText(account.getName(), code))) {
            if (Main.isTestMode()) {
                logger.info("Test mode, email failed: the email code for " + account.getName() + " is " + code);
                return null;
            }
            return "The server couldn't send the email. Try again later.";
        }
        return null;
    }

    /**
     * The welcome mail's lines with a first code, or empty when the account can't get one now.
     */
    public String welcomeLines(String userName, String email, long now) {
        try {
            return "\n\n" + codeText(userName, issue(userName, email, now));
        } catch (IllegalStateException e) {
            return "";
        }
    }

    static String codeText(String userName, String code) {
        return "To confirm this is " + userName + "'s email address, enter this code under Settings, Account: " + code + '\n'
                + "It works for " + (ResetTokens.VALID_MILLIS / 60000) + " minutes and " + ResetTokens.ATTEMPTS + " tries. "
                + "A confirmed address lets you recover your account and receive supporter perks.";
    }

    private static String serverSuffix(ManagerFactory managers) {
        String name = managers.configSettings().getServerName();
        return name == null || name.trim().isEmpty() ? "" : " on " + name.trim();
    }

    private static String codeKey(String userName, String email) {
        return userName.trim() + ' ' + email.trim();
    }

    // --- squatted addresses -----------------------------------------------------------------------

    /**
     * Whether a new account may take this account's address: it never proved it reads it, it was registered more
     * than {@link #SQUAT_MILLIS} ago (or before registration times were kept), and it hasn't signed in for as long.
     * Staff and the owner keep theirs. Without the admin database nothing is released.
     */
    public static boolean releasable(AuthorizedUser holder, long now) {
        if (holder == null || isBlank(holder.getEmail())) {
            return false;
        }
        AccountEmail row;
        AccessPolicy access;
        try {
            access = AccessPolicy.get();
            row = access.store().accountEmail(holder.getName());
        } catch (SQLException | IllegalStateException e) {
            return false;
        }
        if (access.isOwner(holder.getName()) || access.roleOf(holder.getName()) != null) {
            return false;
        }
        return squatted(holder.getEmail(), row, holder.getLastConnection(), now);
    }

    static boolean squatted(String email, AccountEmail row, Date lastConnection, long now) {
        if (row != null && row.verifiedEmail != null && row.verifiedEmail.equalsIgnoreCase(email.trim())) {
            return false;
        }
        if (row != null && row.registeredAt > 0 && now - row.registeredAt < SQUAT_MILLIS) {
            return false;
        }
        return lastConnection == null || now - lastConnection.getTime() >= SQUAT_MILLIS;
    }

    /**
     * Takes the address off an account that holds it unverified (see {@link #releasable}), so a new account can
     * register with it; recorded in the audit log.
     *
     * @return whether it was released
     */
    public static boolean release(AuthorizedUser holder, String by, long now) {
        if (!releasable(holder, now)) {
            return false;
        }
        AuthorizedUserRepository.getInstance().changeEmail(holder.getName(), null);
        try {
            AdminStore store = AccessPolicy.get().store();
            store.emailUnverified(holder.getName());
            store.audit(by, "user.email.release", holder.getName(), "unverified address taken by a new account", null, now);
        } catch (SQLException | IllegalStateException e) {
            logger.error("Email release not recorded for " + holder.getName(), e);
        }
        logger.info("Unverified email released from " + holder.getName() + " for a new account");
        return true;
    }

    /** "n***@example.com": enough for the player to recognise their address, not enough to harvest it. */
    public static String mask(String email) {
        if (isBlank(email)) {
            return "";
        }
        String text = email.trim();
        int at = text.lastIndexOf('@');
        if (at <= 0) {
            return text.charAt(0) + "***";
        }
        return text.charAt(0) + "***" + text.substring(at);
    }

    private static AdminStore store() {
        return AccessPolicy.get().store();
    }

    private static boolean isBlank(String text) {
        return text == null || text.trim().isEmpty();
    }
}
