package mage.server;

import java.security.SecureRandom;
import java.util.LinkedHashMap;
import java.util.Locale;
import java.util.Map;

/**
 * Codes sent by email (password resets, email verification): six digits, valid for a while and for a few guesses only, so they can't be
 * found by trying them all. Kept in memory, so a server restart cancels every pending reset.
 */
final class ResetTokens {

    static final long VALID_MILLIS = 30 * 60 * 1000L;
    static final int ATTEMPTS = 5;
    private static final int MAX_PENDING = 1024;

    private static final class Pending {
        final String code;
        final long expiresAt;
        /** someone other than the address's reader saw the code (staff made it), so it proves nothing about the address */
        final boolean seenByStaff;
        int attemptsLeft = ATTEMPTS;

        Pending(String code, long expiresAt, boolean seenByStaff) {
            this.code = code;
            this.expiresAt = expiresAt;
            this.seenByStaff = seenByStaff;
        }
    }

    /** What a right code says about how it reached the player. */
    enum Redeemed {
        /** it came by mail only: whoever typed it reads the address */
        BY_MAIL,
        /** staff made it and passed it on (or mailed it as well) */
        FROM_STAFF
    }

    private final SecureRandom random = new SecureRandom();
    private final Map<String, Pending> pending = new LinkedHashMap<String, Pending>() {
        @Override
        protected boolean removeEldestEntry(Map.Entry<String, Pending> eldest) {
            return size() > MAX_PENDING;
        }
    };

    /**
     * @return a new code for this email address; it replaces any earlier one
     */
    synchronized String issue(String email, long now) {
        return issue(email, now, VALID_MILLIS);
    }

    synchronized String issue(String email, long now, long validMillis) {
        return issue(email, now, validMillis, false);
    }

    synchronized String issue(String email, long now, long validMillis, boolean seenByStaff) {
        String code = String.format("%06d", random.nextInt(1_000_000));
        pending.put(key(email), new Pending(code, now + validMillis, seenByStaff));
        return code;
    }

    /**
     * Checks a code, using up one guess. A right code is used up too.
     */
    synchronized boolean redeem(String email, String code, long now) {
        return redeemCode(email, code, now) != null;
    }

    /**
     * Like {@link #redeem}, saying how a right code reached the player; null when it is wrong or gone.
     */
    synchronized Redeemed redeemCode(String email, String code, long now) {
        String key = key(email);
        Pending entry = pending.get(key);
        if (entry == null) {
            return null;
        }
        if (now > entry.expiresAt) {
            pending.remove(key);
            return null;
        }
        if (code != null && java.security.MessageDigest.isEqual(entry.code.getBytes(), code.trim().getBytes())) {
            pending.remove(key);
            return entry.seenByStaff ? Redeemed.FROM_STAFF : Redeemed.BY_MAIL;
        }
        entry.attemptsLeft--;
        if (entry.attemptsLeft <= 0) {
            pending.remove(key);
        }
        return null;
    }

    private static String key(String email) {
        return email == null ? "" : email.trim().toLowerCase(Locale.ROOT);
    }
}
