package mage.server.career;

import mage.players.PlayerType;

/**
 * The Archmage seat (S050): Career's tier five is meant for the hybrid AI of the Archmage plan (an LLM guiding the
 * search, with a spending cap a day). Until that AI lands, the seat plays the usual AI; the cap setting
 * ({@code archmageDailyCents}) is already there, and 0 keeps the hybrid off even once it exists.
 */
public final class Archmage {

    public static final String AI_TYPE = "archmage";

    private Archmage() {
    }

    /** whether the hybrid AI could play now: it exists, and today's cap leaves room */
    public static boolean available() {
        return false;
    }

    /** the AI to seat for tier five: the usual one, until the hybrid exists */
    public static PlayerType seat() {
        return PlayerType.COMPUTER_MAD;
    }

    /** a line for the opponent card */
    public static String status() {
        int cap = mage.server.admin.RuntimeSettings.get().archmageDailyCents();
        return cap <= 0 ? "The Archmage AI is off on this server; the usual AI plays at its best."
                : "The Archmage AI isn't ready yet; the usual AI plays at its best until it is.";
    }
}
