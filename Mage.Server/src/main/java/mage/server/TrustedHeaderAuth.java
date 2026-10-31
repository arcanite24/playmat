package mage.server;

import mage.server.admin.AccessPolicy;
import org.apache.log4j.Logger;

import java.security.SecureRandom;
import java.sql.SQLException;
import java.util.Base64;
import java.util.Locale;
import java.util.regex.Pattern;

/**
 * Single sign-on through a proxy in front (Authelia, Authentik, oauth2-proxy, tailscale serve): a request header names
 * the signed-in user, and the server believes it. See docs/TrustedHeaderAuth.md.
 * <p>
 * Off unless {@value #HEADER_PROP} names the header. The web bridge reads it only in the WebSocket handshake and only
 * from a trusted proxy ({@code xmage.web.trustedProxies}); in the Docker stack that is Caddy, which deletes the header
 * from every request that doesn't come from the sign-on proxy. The name is pinned to the connection.
 * <p>
 * The first sign-in links the account of that name (any case) or creates one, with a random password and no email.
 * With {@value #PASSWORD_SIGNIN_PROP}=false the header is the only way in: no password sign-in, registration, reset
 * codes, remembered devices or GitHub sign-in. The admin password ({@code connectAdmin}) still works.
 */
public final class TrustedHeaderAuth {

    private static final Logger logger = Logger.getLogger(TrustedHeaderAuth.class);

    public static final String HEADER_PROP = "xmage.auth.trustedHeader";
    public static final String PASSWORD_SIGNIN_PROP = "xmage.auth.passwordSignIn";
    public static final String STRIP_DOMAIN_PROP = "xmage.auth.trustedHeaderStripDomain";

    /** what a header value may be at most; anything longer is not a name */
    static final int MAX_VALUE_LENGTH = 254;
    /** names of new accounts, like registration and GitHub sign-in */
    private static final Pattern NEW_NAME = Pattern.compile("[A-Za-z0-9_]{3,14}");
    public static final String PASSWORDS_OFF = "This server signs in through its single sign-on. Open it through the sign-on page.";

    private static final TrustedHeaderAuth OFF = new TrustedHeaderAuth("", true, false);
    private static final SecureRandom random = new SecureRandom();
    private static volatile TrustedHeaderAuth instance;

    private final String header;
    private final boolean passwordSignIn;
    private final boolean stripDomain;

    public TrustedHeaderAuth(String header, boolean passwordSignIn, boolean stripDomain) {
        this.header = header == null ? "" : header.trim();
        // passwords can be turned off only when something else signs people in
        this.passwordSignIn = passwordSignIn || this.header.isEmpty();
        this.stripDomain = stripDomain;
    }

    public static TrustedHeaderAuth get() {
        TrustedHeaderAuth current = instance;
        if (current == null) {
            synchronized (TrustedHeaderAuth.class) {
                current = instance;
                if (current == null) {
                    current = new TrustedHeaderAuth(System.getProperty(HEADER_PROP, ""),
                            !"false".equalsIgnoreCase(System.getProperty(PASSWORD_SIGNIN_PROP, "true").trim()),
                            Boolean.parseBoolean(System.getProperty(STRIP_DOMAIN_PROP, "false").trim()));
                    instance = current;
                }
            }
        }
        return current;
    }

    /** For tests; null goes back to the system properties. */
    public static void use(TrustedHeaderAuth auth) {
        instance = auth;
    }

    public static TrustedHeaderAuth off() {
        return OFF;
    }

    public boolean isEnabled() {
        return !header.isEmpty();
    }

    /** The header the sign-on proxy sets, "" when off. */
    public String header() {
        return header;
    }

    /** Whether accounts can sign in with a password (and register, reset it, remember devices, use GitHub). */
    public boolean passwordSignIn() {
        return passwordSignIn;
    }

    /**
     * The name the header's value signs in, or null when it names nobody.
     */
    public String nameFrom(String value) {
        if (!isEnabled() || value == null) {
            return null;
        }
        String name = value.trim();
        if (name.isEmpty() || name.length() > MAX_VALUE_LENGTH || name.indexOf(',') >= 0) {
            // several values (the header sent twice) are not one person
            return null;
        }
        if (stripDomain && name.indexOf('@') > 0) {
            name = name.substring(0, name.indexOf('@'));
        }
        return name;
    }

    /**
     * The account the signed-on name signs in to: the one with that name in any case, or a new one.
     *
     * @return the account's name as stored
     * @throws IllegalArgumentException with the reason for the player when there is no account and none can be made
     */
    public String account(AuthorizedUserRepository accounts, String name, long now) {
        synchronized (accounts) {
            AuthorizedUser existing = accounts.getByNameIgnoreCase(name);
            if (existing != null) {
                return existing.getName();
            }
            if (!NEW_NAME.matcher(name).matches() || name.equalsIgnoreCase(User.ADMIN_NAME)) {
                throw new IllegalArgumentException("Your sign-on name \"" + name + "\" can't be a player name here (3 to 14 "
                        + "letters, digits or underscores). Ask whoever runs the sign-on to change it.");
            }
            // the gates registration has: a contributors-only server's allowlist, and new accounts closed in /admin
            String refused = Session.admitNewAccount(name, null, null);
            if (refused != null) {
                throw new IllegalArgumentException(AccessPolicy.REFUSED.equals(refused) ? refused
                        : "There is no account for " + name + " here, and new accounts are closed. Ask the server's staff.");
            }
            byte[] bytes = new byte[24];
            random.nextBytes(bytes);
            // nobody knows this password: the account signs in through the sign-on (or a reset code, once it has an email)
            accounts.add(name, Base64.getEncoder().encodeToString(bytes) + "a1", null);
            if (accounts.getByName(name) == null) {
                throw new IllegalArgumentException("The account can't be created right now. Try again in a minute.");
            }
            try {
                AccessPolicy.get().store().accountRegistered(name, now);
                AccessPolicy.get().store().audit(name, "access.sso.register", name, "header " + header.toLowerCase(Locale.ROOT), null, now);
            } catch (SQLException e) {
                logger.warn("Account " + name + " created by single sign-on, but not recorded: " + e.getMessage());
            }
            logger.info("Account " + name + " created by single sign-on");
            return name;
        }
    }
}
