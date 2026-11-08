package mage.server.admin;

import mage.server.AuthorizedUser;
import mage.server.AuthorizedUserRepository;
import mage.server.User;

import java.io.IOException;
import java.security.SecureRandom;
import java.sql.SQLException;
import java.util.Base64;

/**
 * The end of an identity-provider sign-in: the account it signs in to (linked before, or with the
 * provider's verified email), or a new one. Answers with a remembered-device token that signs the account
 * in like "keep me signed in" does, so the account needs no password of its own.
 */
public final class OidcSignIn {

    /** What the web client gets back. */
    public static class OidcSignInResult {
        /** the account to sign in to, with rememberToken */
        public String userName;
        public String rememberToken;
        /**
         * a one-use ref for the sign-in's ID token (connectUser's idTokenRef): it lets the session end the
         * provider session on sign-out (RFC 7063). Null when there is no token to carry.
         */
        public String idTokenRef;
        /** set when a new account needs a name first: the provider's display name made to fit the name rules */
        public String suggestedName;
        /** why the suggested or chosen name can't be used */
        public String nameProblem;
        /**
         * set with suggestedName: the one secret the naming retry sends in place of the provider's code (the
         * state is in the callback URL, so it alone can't finish the sign-in)
         */
        public String resume;
    }

    private static final SecureRandom random = new SecureRandom();

    private final OidcAuth oidc;
    private final AccessPolicy access;
    private final AuthorizedUserRepository accounts;

    public OidcSignIn(OidcAuth oidc, AccessPolicy access, AuthorizedUserRepository accounts) {
        this.oidc = oidc;
        this.access = access;
        this.accounts = accounts;
    }

    /**
     * @param chosenName the name the player picked for a new account, or null to try the provider's display name
     */
    public OidcSignInResult finish(boolean accountsOn, String state, String code, String chosenName, long now) throws IOException, SQLException {
        try {
            return attempt(accountsOn, state, code, chosenName, now);
        } catch (RuntimeException | IOException | SQLException e) {
            oidc.forget(state); // a refused or failed sign-in leaves nothing behind for its state
            throw e;
        }
    }

    static final String UNCONFIRMED_EMAIL = "An account on this server already uses that email address, but hasn't"
            + " confirmed it. Sign in to that account and confirm the email in Settings, then try again.";

    private OidcSignInResult attempt(boolean accountsOn, String state, String code, String chosenName, long now) throws IOException, SQLException {
        if (!accountsOn) {
            throw new IllegalStateException("This server has no accounts, so there is nothing to sign in to with " + oidc.label());
        }
        String label = oidc.label();
        OidcAuth.Identity who = oidc.identify(state, code);
        AdminStore store = access.store();

        // signed in with the provider before, or an account with the same verified email
        String account = null;
        OidcLink link = store.oidcLink(who.sub);
        if (link != null && accounts.getByName(link.account) != null) {
            account = link.account;
        } else if (who.email != null && who.emailVerified) {
            AuthorizedUser byEmail = accounts.getByEmail(who.email);
            if (byEmail != null) {
                // Both sides must have proved the address. Accounts register with any email, unconfirmed, so a
                // match on the provider's word alone would hand an account someone pre-registered with this
                // address (still holding its password) to the provider user, or the reverse.
                AccountEmail proved = store.accountEmail(byEmail.getName());
                if (proved == null || proved.verifiedEmail == null || !proved.verifiedEmail.equalsIgnoreCase(who.email)) {
                    throw new IllegalArgumentException(UNCONFIRMED_EMAIL);
                }
                account = byEmail.getName();
            }
        }
        if (account != null) {
            AuthorizedUser user = accounts.getByName(account);
            // refused before anything is written: a refused sign-in leaves no link behind
            if (!access.allows(account, user.getEmail())) {
                throw new IllegalArgumentException(AccessPolicy.REFUSED);
            }
            store.linkOidc(who.sub, who.displayName, who.email, account, now);
            // the provider vouches that this person reads the address
            if (who.emailVerified && who.email != null && who.email.equalsIgnoreCase(user.getEmail())) {
                store.emailVerified(account, user.getEmail(), now);
            }
            return signedIn(account, state);
        }

        // a new account: the server must let this provider user in, and it needs a verified email for password resets
        if (access.isAllowlist() && !access.allows(null, who.email)) {
            throw new IllegalArgumentException(AccessPolicy.REFUSED);
        }
        if (who.email == null || !who.emailVerified) {
            throw new IllegalArgumentException("Your " + label + " account has no verified email address. Verify one in " + label + ", then try again.");
        }
        String name = chosenName == null || chosenName.trim().isEmpty() ? OidcAuth.suggestName(who.displayName) : chosenName.trim();
        String problem = nameProblem(name);
        if (problem != null) {
            OidcSignInResult result = new OidcSignInResult();
            result.suggestedName = chosenName == null ? name : OidcAuth.suggestName(who.displayName);
            result.nameProblem = chosenName == null && problem.equals(TAKEN) ? null : problem;
            result.resume = oidc.resumeSecret(state);
            return result;
        }
        byte[] bytes = new byte[24];
        random.nextBytes(bytes);
        // nobody knows this password: the account signs in with the provider, or with a reset code sent to its email
        accounts.add(name, Base64.getEncoder().encodeToString(bytes) + "a1", who.email);
        store.linkOidc(who.sub, who.displayName, who.email, name, now);
        store.accountRegistered(name, now);
        if (who.emailVerified) {
            store.emailVerified(name, who.email, now);
        }
        String display = who.displayName == null ? who.sub : who.displayName;
        if (access.isAllowlist()) {
            store.allow(name, "oidc: " + display, "oidc", now);
        }
        store.audit(name, "access.oidc.register", name, "oidc: " + display, null, now);
        return signedIn(name, state);
    }

    static final String TAKEN = "That name is taken. Choose another.";

    String nameProblem(String name) {
        if (!name.matches("[A-Za-z0-9_]{3,14}")) {
            return "3 to 14 letters, digits or underscores.";
        }
        if (name.equalsIgnoreCase(User.ADMIN_NAME) || accounts.getByName(name) != null) {
            return TAKEN;
        }
        return null;
    }

    private OidcSignInResult signedIn(String account, String state) {
        OidcSignInResult result = new OidcSignInResult();
        result.userName = account;
        result.rememberToken = accounts.rememberLogin(account);
        result.idTokenRef = oidc.done(state);
        return result;
    }
}
