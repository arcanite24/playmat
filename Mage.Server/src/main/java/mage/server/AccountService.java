package mage.server;

import mage.server.managers.ManagerFactory;
import org.apache.log4j.Logger;

/**
 * Password resets by emailed code, shared by the desktop protocol and the web client bridge. Methods return null when
 * they worked, or the reason they didn't, for the caller to show.
 */
public final class AccountService {

    private static final Logger logger = Logger.getLogger(AccountService.class);

    /** the reason when no account has the address; the web client doesn't tell, so addresses can't be probed */
    public static final String NO_SUCH_EMAIL = "No user was found with that email address";

    // one set of pending codes for every caller, so a code asked for in one client works in the other
    private static final ResetTokens TOKENS = new ResetTokens();

    private final ManagerFactory managers;

    public AccountService(ManagerFactory managers) {
        this.managers = managers;
    }

    public String sendResetCode(String email) {
        if (!managers.configSettings().isAuthenticationActivated()) {
            return Session.REGISTRATION_DISABLED_MESSAGE;
        }
        // single sign-on only: a password would sign nobody in
        if (!TrustedHeaderAuth.get().passwordSignIn()) {
            return TrustedHeaderAuth.PASSWORDS_OFF;
        }
        AuthorizedUser user = email == null ? null : AuthorizedUserRepository.getInstance().getByEmail(email.trim());
        if (user == null) {
            logger.info("Password reset code asked for an address without an account");
            return NO_SUCH_EMAIL;
        }
        String code = TOKENS.issue(email, System.currentTimeMillis());
        String server = managers.configSettings().getServerName();
        String subject = "Password reset code" + (server == null || server.trim().isEmpty() ? "" : " for " + server.trim());
        String text = "Use this code to reset " + user.getName() + "'s password: " + code + '\n'
                + "It works for " + (ResetTokens.VALID_MILLIS / 60000) + " minutes and " + ResetTokens.ATTEMPTS + " tries."
                + " If you didn't ask for it, ignore this email.";
        boolean sent = Mailer.send(managers, user.email, subject, text);
        if (!sent) {
            if (Main.isTestMode()) {
                logger.info("Test mode, email failed: the password reset code for " + user.getName() + " is " + code);
                return null;
            }
            return "The server couldn't send the email. Try again later or ask the server's admin.";
        }
        return null;
    }

    /** how long a code made by staff works: they pass it on by hand, which takes longer than an email */
    public static final long STAFF_CODE_MILLIS = java.util.concurrent.TimeUnit.HOURS.toMillis(24);

    /** A reset code staff made for a player. */
    public static class StaffResetCode {
        /** shown to staff once, to pass on */
        public String code;
        /** the account's email, which the player enters with the code */
        public String email;
        /** the code was emailed to the player as well */
        public boolean emailed;
        public long expiresAt;
    }

    /**
     * A password reset code for an account, for servers without mail or players without access to theirs. The player
     * enters it under Forgot password with the account's email. It replaces any earlier code for the account.
     */
    public StaffResetCode staffResetCode(String userName) {
        if (!managers.configSettings().isAuthenticationActivated()) {
            throw new IllegalStateException(Session.REGISTRATION_DISABLED_MESSAGE);
        }
        AuthorizedUser user = AuthorizedUserRepository.getInstance().getByName(userName == null ? "" : userName.trim());
        if (user == null) {
            throw new IllegalArgumentException("There is no account named " + userName);
        }
        if (user.email == null || user.email.trim().isEmpty()) {
            throw new IllegalArgumentException(user.getName() + " has no email address, which a reset code needs");
        }
        long now = System.currentTimeMillis();
        StaffResetCode result = new StaffResetCode();
        result.code = TOKENS.issue(user.email, now, STAFF_CODE_MILLIS, true);
        result.email = user.email;
        result.expiresAt = now + STAFF_CODE_MILLIS;
        if (Mailer.configured(managers.configSettings())) {
            String text = "The server's staff made a code to reset " + user.getName() + "'s password: " + result.code + '\n'
                    + "Enter it under Forgot password with this email address. It works for 24 hours.";
            result.emailed = Mailer.send(managers, user.email, "Password reset code", text);
        }
        logger.info("Password reset code made by staff for " + user.getName());
        return result;
    }

    public String resetPassword(String email, String code, String password) {
        if (!managers.configSettings().isAuthenticationActivated()) {
            return Session.REGISTRATION_DISABLED_MESSAGE;
        }
        // single sign-on only: a password would sign nobody in
        if (!TrustedHeaderAuth.get().passwordSignIn()) {
            return TrustedHeaderAuth.PASSWORDS_OFF;
        }
        AuthorizedUser user = email == null ? null : AuthorizedUserRepository.getInstance().getByEmail(email.trim());
        if (user == null) {
            // same answer as a wrong code
            return "That code is wrong or has expired. Ask for a new one.";
        }
        // checked before the code, so a weak password doesn't use the code up
        String problem = Session.passwordProblem(managers.configSettings(), password, user.getName());
        if (problem != null) {
            return problem;
        }
        long now = System.currentTimeMillis();
        ResetTokens.Redeemed redeemed = TOKENS.redeemCode(email, code, now);
        if (redeemed == null) {
            logger.info("Wrong or expired password reset code for " + user.getName());
            return "That code is wrong or has expired. Ask for a new one.";
        }
        // the new password only: locks, the active flag and the email stay as they were
        if (!AuthorizedUserRepository.getInstance().changePassword(user.getName(), password)) {
            return "The password couldn't be changed. Try again.";
        }
        // a code that came only by mail proves the player reads the address; one staff passed on doesn't
        if (redeemed == ResetTokens.Redeemed.BY_MAIL) {
            EmailVerification.markVerified(user.getName(), user.getEmail(), now);
        }
        logger.info("Password reset for " + user.getName());
        return null;
    }
}
