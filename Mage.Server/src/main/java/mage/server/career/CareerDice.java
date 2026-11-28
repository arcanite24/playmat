package mage.server.career;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.sql.SQLException;
import java.util.Random;

/**
 * The Career's chance, seeded (docs/rust/Payouts.md, "Dice"): each roll is a {@link Random} seeded with the first eight
 * bytes of SHA-256 over the server's secret (career.db's {@code meta}), a newline and what the roll is for, such as
 * {@code quest/ana/2026-11-11/0}. The Rust platform service seeds and draws the same way (playmat-platform's
 * {@code dice.rs}, with Java's {@code Random} ported), so both sides pick the same quests and capsules from the same
 * rows and a shadow run can compare them. Players can't foresee a roll without the secret.
 */
public final class CareerDice {

    private CareerDice() {
    }

    /** the roll for one purpose, from the store's secret */
    public static Random roll(CareerStore store, String purpose) throws SQLException {
        return new Random(seed(store.secret(), purpose));
    }

    /** the seed: SHA-256 of the secret, a newline and the purpose, its first eight bytes big-endian */
    public static long seed(String secret, String purpose) {
        try {
            MessageDigest sha = MessageDigest.getInstance("SHA-256");
            byte[] digest = sha.digest((secret + "\n" + purpose).getBytes(StandardCharsets.UTF_8));
            long seed = 0;
            for (int i = 0; i < 8; i++) {
                seed = (seed << 8) | (digest[i] & 0xFF);
            }
            return seed;
        } catch (NoSuchAlgorithmException e) {
            throw new IllegalStateException(e);
        }
    }
}
