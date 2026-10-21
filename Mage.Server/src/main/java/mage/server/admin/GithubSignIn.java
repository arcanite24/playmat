package mage.server.admin;

import mage.server.AuthorizedUser;
import mage.server.AuthorizedUserRepository;
import mage.server.User;

import java.io.IOException;
import java.security.SecureRandom;
import java.sql.SQLException;
import java.util.Base64;

/**
 * The end of a GitHub sign-in: the account it signs in to (linked before, or with the GitHub user's verified email),
 * or a new one. Answers with a remembered-device token that signs the account in like "keep me signed in" does, so
 * the account needs no password of its own.
 */
public final class GithubSignIn {

    /** What the web client gets back. */
    public static class GithubSignInResult {
        /** the account to sign in to, with rememberToken */
        public String userName;
        public String rememberToken;
        /** set when a new account needs a name first: the GitHub login made to fit the name rules */
        public String suggestedName;
        /** why the suggested or chosen name can't be used */
        public String nameProblem;
    }

    private static final SecureRandom random = new SecureRandom();

    private final GithubAuth github;
    private final AccessPolicy access;
    private final AuthorizedUserRepository accounts;

    /** Told when a GitHub user signs in to an account (support links their sponsorship to it); set at startup. */
    public interface LinkListener {
        void linked(long githubId, String account, long now);
    }

    private static volatile LinkListener linkListener;

    public static void onLinked(LinkListener listener) {
        linkListener = listener;
    }

    private static void linked(long githubId, String account, long now) {
        LinkListener listener = linkListener;
        if (listener != null) {
            try {
                listener.linked(githubId, account, now);
            } catch (RuntimeException e) {
                // a sign-in never fails over what follows it
                org.apache.log4j.Logger.getLogger(GithubSignIn.class).warn("After GitHub sign-in: " + e.getMessage());
            }
        }
    }

    public GithubSignIn(GithubAuth github, AccessPolicy access, AuthorizedUserRepository accounts) {
        this.github = github;
        this.access = access;
        this.accounts = accounts;
    }

    /**
     * @param chosenName the name the player picked for a new account, or null to try the GitHub login
     */
    public GithubSignInResult finish(boolean accountsOn, String state, String code, String chosenName, long now) throws IOException, SQLException {
        if (!accountsOn) {
            throw new IllegalStateException("This server has no accounts, so there is nothing to sign in to with GitHub");
        }
        GithubAuth.Identity who = github.identify(state, code);
        AdminStore store = access.store();

        // signed in with GitHub before, or an account with the same verified email
        String account = null;
        GithubLink link = store.githubLink(who.id);
        if (link != null && accounts.getByName(link.account) != null) {
            account = link.account;
        } else if (who.email != null) {
            AuthorizedUser byEmail = accounts.getByEmail(who.email);
            account = byEmail == null ? null : byEmail.getName();
        }
        if (account != null) {
            AuthorizedUser user = accounts.getByName(account);
            store.linkGithub(who.id, who.login, account, now);
            linked(who.id, account, now);
            // GitHub vouches that this person reads the address
            if (who.email != null && who.email.equalsIgnoreCase(user.getEmail())) {
                store.emailVerified(account, user.getEmail(), now);
            }
            if (!access.allows(account, user.getEmail()) && !github.isRepoPerson(who.login)) {
                throw new IllegalArgumentException(AccessPolicy.REFUSED);
            }
            github.done(state);
            return signedIn(account);
        }

        // a new account: the server must let this GitHub user in, and it needs an email for password resets
        if (access.isAllowlist() && !github.isRepoPerson(who.login) && !access.allows(null, who.email)) {
            throw new IllegalArgumentException(AccessPolicy.REFUSED);
        }
        if (who.email == null) {
            throw new IllegalArgumentException("Your GitHub account has no verified email address. Verify one on GitHub, then try again.");
        }
        String name = chosenName == null || chosenName.trim().isEmpty() ? GithubAuth.suggestName(who.login) : chosenName.trim();
        String problem = nameProblem(name);
        if (problem != null) {
            GithubSignInResult result = new GithubSignInResult();
            result.suggestedName = chosenName == null ? name : GithubAuth.suggestName(who.login);
            result.nameProblem = chosenName == null && problem.equals(TAKEN) ? null : problem;
            return result;
        }
        byte[] bytes = new byte[24];
        random.nextBytes(bytes);
        // nobody knows this password: the account signs in with GitHub, or with a reset code sent to its email
        accounts.add(name, Base64.getEncoder().encodeToString(bytes) + "a1", who.email);
        store.linkGithub(who.id, who.login, name, now);
        linked(who.id, name, now);
        store.accountRegistered(name, now);
        store.emailVerified(name, who.email, now);
        if (access.isAllowlist() && github.isRepoPerson(who.login)) {
            store.allow(name, "github: " + who.login, "github", now);
        }
        store.audit(name, "access.github.register", name, "github: " + who.login, null, now);
        github.done(state);
        return signedIn(name);
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

    private GithubSignInResult signedIn(String account) {
        GithubSignInResult result = new GithubSignInResult();
        result.userName = account;
        result.rememberToken = accounts.rememberLogin(account);
        return result;
    }
}
