package mage.server;

import mage.cards.Card;
import mage.game.Game;
import mage.game.GameOptions;
import mage.game.match.Match;
import mage.game.match.MatchPlayer;
import mage.game.trace.DecisionLog;
import mage.game.trace.GameTrace;
import mage.game.trace.RustAiBridge;
import org.apache.log4j.Logger;

import java.util.*;

/**
 * Which games the Rust AI plays the AI's seat of (docs/rust/Ai.md, ops/web/README.md "Rust AI"): a one-on-one game
 * of a listed player against the Java AI, from opening hands, with every card in both decks in the Rust engine.
 * <p>
 * {@code XMAGE_AI_BACKEND=rust} turns it on; {@code XMAGE_RUST_AI_USERS} lists the players (comma separated user names,
 * {@code *} for everyone; empty: nobody); {@code XMAGE_ENGINE=java} turns every Rust part off. Whenever the Rust copy
 * of a game differs from Java's, that game goes back to the Java AI.
 */
public final class RustAi {

    private static final Logger logger = Logger.getLogger(RustAi.class);

    private RustAi() {
    }

    public static boolean enabled() {
        return "rust".equalsIgnoreCase(env("XMAGE_AI_BACKEND")) && !"java".equalsIgnoreCase(env("XMAGE_ENGINE"));
    }

    static boolean forUser(String name) {
        for (String u : env("XMAGE_RUST_AI_USERS").split(",")) {
            u = u.trim();
            if (!u.isEmpty() && (u.equals("*") || u.equalsIgnoreCase(name))) {
                return true;
            }
        }
        return false;
    }

    private static String env(String name) {
        String v = System.getenv(name);
        return v == null ? "" : v.trim();
    }

    /**
     * Sets the game up for the Rust AI when it qualifies: traces it, logs its answers, and hands the AI's seat to a
     * bridge. Changes nothing otherwise.
     *
     * @param humans the user name of each human player, by player id
     */
    public static void prepare(Match match, Game game, GameOptions options, Map<UUID, String> humans) {
        try {
            if (!enabled() || game == null || match.getPlayers().size() != 2 || humans.size() != 1) {
                return;
            }
            if (options.setup != null || options.resumeFrom != null || options.testMode
                    || options.trace != null || options.decisions != null) {
                return;
            }
            String user = humans.values().iterator().next();
            if (!forUser(user)) {
                return;
            }
            MatchPlayer ai = null;
            List<String> names = new ArrayList<>();
            for (MatchPlayer p : match.getPlayers()) {
                if (!humans.containsKey(p.getPlayer().getId())) {
                    ai = p;
                }
                if (p.getDeck() == null) {
                    return;
                }
                for (Card c : p.getDeck().getCards()) {
                    names.add(c.getName());
                }
            }
            if (ai == null) {
                return;
            }
            Set<String> missing = RustAiBridge.missing(names);
            if (!missing.isEmpty()) {
                logger.info("Rust AI: " + user + " vs " + ai.getName() + " doesn't qualify, the Rust engine lacks "
                        + missing.size() + " card" + (missing.size() == 1 ? "" : "s") + ": " + String.join(", ", missing));
                return;
            }
            DecisionLog log = DecisionLog.recording();
            RustAiBridge bridge = new RustAiBridge(ai.getPlayer().getId(), log, user + " vs " + ai.getName());
            mage.players.Player seat = game.getPlayer(ai.getPlayer().getId());
            if (seat == null || !seat.attachRustAi(bridge)) {
                return;
            }
            GameTrace trace = new GameTrace(false, entry -> bridge.onTrace(game, entry));
            trace.setOnEnd(bridge::end);
            options.decisions = log;
            options.trace = trace;
            logger.info("Rust AI: " + user + " vs " + ai.getName() + " qualifies (AI deck: " + ai.getDeck().getName() + ")");
        } catch (RuntimeException e) {
            // the Rust AI is an extra: never stop a game over it
            logger.warn("Rust AI: setting up the game failed, the Java AI plays", e);
        }
    }
}
