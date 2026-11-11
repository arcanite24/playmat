package mage.server;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import mage.cards.Card;
import mage.game.Game;
import mage.game.GameOptions;
import mage.game.match.Match;
import mage.game.match.MatchPlayer;
import mage.game.trace.RustAiBridge;
import mage.interfaces.callback.ClientCallback;
import mage.interfaces.callback.ClientCallbackMethod;
import mage.players.Player;
import mage.server.managers.ManagerFactory;
import mage.server.websocket.rpc.JsonCodec;
import org.apache.log4j.Logger;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Games the Rust host plays instead of Java (docs/rust/GameHost.md, step D): a duel of one listed user against the AI,
 * every card in the Rust engine, connected through the gateway. A hosted solo game doesn't save itself (it can't be
 * resumed), and one resumed from a save stays on Java. Java keeps the table and the match: at each game's
 * start it sends the gateway the seats and decks ({@code HOST_GAME}, which the client never sees) instead of starting
 * its own game, and the gateway reports the end ({@code hostGameEnded}), which ends the game here as if Java had
 * played it. A match of more than one game is a hosted game at a time: Java records each result, runs sideboarding,
 * and hands the next game to the host again, with the last game's loser choosing who plays first.
 */
public final class RustHost {

    private static final Logger logger = Logger.getLogger(RustHost.class);

    /** what a hosted game needs at its end: the table, the game, and the token the end must carry */
    private static final class Hosted {
        final UUID tableId;
        final Match match;
        final Game game;
        final String token;
        final ManagerFactory managers;
        /** the human's user and player ids */
        final UUID userId;
        final UUID playerId;
        /** when the sweep last found the human gone or connected without a hosting gateway, or 0 */
        volatile long goneSince;

        Hosted(UUID tableId, Match match, Game game, String token, ManagerFactory managers, UUID userId, UUID playerId) {
            this.tableId = tableId;
            this.match = match;
            this.game = game;
            this.token = token;
            this.managers = managers;
            this.userId = userId;
            this.playerId = playerId;
        }
    }

    private static final Map<UUID, Hosted> HOSTED = new ConcurrentHashMap<>();

    /**
     * A game whose human has been gone, or connected without a hosting gateway, this long ends as their loss: its
     * host (and the gateway with it) is gone, or would have ended it itself (the host waits 30 minutes for an answer).
     */
    private static final long GONE_MS = 35 * 60 * 1000L;
    private static final java.util.concurrent.ScheduledExecutorService SWEEP =
            java.util.concurrent.Executors.newSingleThreadScheduledExecutor(r -> {
                Thread t = new Thread(r, "rust-host-sweep");
                t.setDaemon(true);
                return t;
            });

    static {
        SWEEP.scheduleWithFixedDelay(RustHost::sweep, 1, 1, java.util.concurrent.TimeUnit.MINUTES);
    }

    private RustHost() {
    }

    static boolean enabled() {
        return (!env("XMAGE_RUST_HOST_USERS").isEmpty() || optIn()) && !"java".equalsIgnoreCase(env("XMAGE_ENGINE"));
    }

    /**
     * Whether players can opt in ({@code XMAGE_RUST_OPT_IN=on}): a player who turned the Rust engine on in their
     * settings ({@code UserData.useRustEngine}) has their qualifying games played by it, as a listed one does
     */
    public static boolean optIn() {
        String v = env("XMAGE_RUST_OPT_IN");
        return ("on".equalsIgnoreCase(v) || "true".equalsIgnoreCase(v)) && !"java".equalsIgnoreCase(env("XMAGE_ENGINE"));
    }

    /** a listed player ({@code XMAGE_RUST_HOST_USERS}), or one who opted in where the server allows it */
    static boolean forUser(User user) {
        return forUser(user.getName()) || optIn() && user.getUserData() != null && user.getUserData().isUseRustEngine();
    }

    static boolean forUser(String name) {
        for (String u : env("XMAGE_RUST_HOST_USERS").split(",")) {
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

    public static boolean isHosted(UUID gameId) {
        return gameId != null && HOSTED.containsKey(gameId);
    }

    /**
     * Hands the match's new game to the host when it qualifies. True: the host plays it, and Java starts no game of
     * its own.
     *
     * @param userPlayerMap each human's user id and player id
     * @param chooser who chooses who plays first (a match's last loser), or null for a random player
     */
    static boolean host(ManagerFactory managers, UUID tableId, Match match, Map<UUID, UUID> userPlayerMap,
                        GameOptions options, UUID chooser) {
        try {
            Game game = match.getGame();
            if (!enabled() || game == null || userPlayerMap.size() != 1) {
                return false;
            }
            Map.Entry<UUID, UUID> human = userPlayerMap.entrySet().iterator().next();
            Optional<User> user = managers.userManager().getUser(human.getKey());
            if (!user.isPresent() || !forUser(user.get())) {
                return false;
            }
            String why = null;
            if (match.getPlayers().size() != 2) {
                why = "not a duel";
            } else if (options.setup != null || options.resumeFrom != null) {
                why = "it starts from a setup or a save";
            } else if (mage.server.career.CareerService.get().isCareerTable(tableId)) {
                why = "a Career table";
            } else if (!user.get().hostsGames()) {
                why = "the player isn't connected through a gateway that hosts games";
            }
            if (why != null) {
                logger.info("Rust host: " + user.get().getName() + "'s game stays on Java: " + why);
                return false;
            }
            List<String> names = new ArrayList<>();
            JsonArray seats = new JsonArray();
            for (MatchPlayer p : match.getPlayers()) {
                Player player = game.getPlayer(p.getPlayer().getId());
                if (p.getDeck() == null || player == null) {
                    return false;
                }
                boolean isHuman = p.getPlayer().getId().equals(human.getValue());
                if (!isHuman && p.getPlayer().isHuman()) {
                    return false;
                }
                JsonObject seat = new JsonObject();
                seat.addProperty("playerId", player.getId().toString());
                seat.addProperty("name", player.getName());
                seat.addProperty("human", isHuman);
                if (match.getOptions().getMatchTimeLimit().getPrioritySecs() > 0 && match.getNumGames() > 1) {
                    // a match's later game: the time left from the earlier ones (MatchImpl.initGame)
                    seat.addProperty("priorityTimeLeft", Math.max(0, player.getPriorityTimeLeft()));
                }
                JsonArray deck = new JsonArray();
                // and each card's printing, the picture its views show (the engine's own otherwise)
                JsonArray printings = new JsonArray();
                for (Card c : p.getDeck().getCards()) {
                    deck.add(c.getName());
                    printings.add(c.getExpansionSetCode() + ":" + c.getCardNumber() + ":" + c.getImageNumber());
                    names.add(c.getName());
                }
                seat.add("deck", deck);
                seat.add("printings", printings);
                if (isHuman) {
                    seat.add("userData", JsonCodec.GSON.toJsonTree(user.get().getUserData()));
                }
                seats.add(seat);
            }
            Set<String> missing = RustAiBridge.missing(names);
            if (!missing.isEmpty()) {
                logger.info("Rust host: " + user.get().getName() + "'s game doesn't qualify, the Rust engine lacks "
                        + missing.size() + " card" + (missing.size() == 1 ? "" : "s") + ": " + String.join(", ", missing));
                return false;
            }
            String token = UUID.randomUUID().toString();
            long seed = options.seed != null ? options.seed : mage.util.RandomUtil.getRandom().nextLong();
            JsonObject data = new JsonObject();
            data.addProperty("token", token);
            data.addProperty("seed", seed);
            data.addProperty("tableId", tableId.toString());
            if (chooser != null) {
                data.addProperty("chooser", chooser.toString());
            }
            data.add("seats", seats);
            // the match's priority timer, which the host runs as GameController does (0: none)
            data.addProperty("priorityTime", match.getOptions().getMatchTimeLimit().getPrioritySecs());
            data.addProperty("bufferTime", match.getOptions().getMatchBufferTime().getBufferSecs());
            Hosted hosted = new Hosted(tableId, match, game, token, managers, human.getKey(), human.getValue());
            // the game's end summary (the match's score, what the client shows between games), as GameController
            // sends it for a game Java plays: MatchImpl.endGame fires it once the result is in
            game.addTableEventListener(event -> {
                if (event.getEventType() == mage.game.events.TableEvent.EventType.END_GAME_INFO) {
                    endGameInfo(hosted);
                }
            });
            HOSTED.put(game.getId(), hosted);
            user.get().fireCallback(new ClientCallback(ClientCallbackMethod.HOST_GAME, game.getId(), data));
            logger.info("Rust host: plays " + user.get().getName() + "'s game " + game.getId() + " (seed " + seed + ")");
            return true;
        } catch (RuntimeException e) {
            logger.warn("Rust host: the game stays on Java: " + e, e);
            return false;
        }
    }

    /**
     * A user who reconnects gets the start of each hosted game they're in again, as {@link User#onReconnect} sends
     * the games Java plays; the client joins it, and the gateway moves the game's callbacks to the new connection.
     */
    static void reconnect(User user) {
        for (Map.Entry<UUID, Hosted> e : HOSTED.entrySet()) {
            Hosted h = e.getValue();
            if (!h.userId.equals(user.getId())) {
                continue;
            }
            mage.game.Table table = h.managers.tableManager().getTable(h.tableId);
            if (table != null) {
                user.ccGameStarted(table.getId(), table.getParentTableId(), e.getKey(), h.playerId);
            }
        }
    }

    /**
     * The host's report of a game's end: the game ends here with that result, and the table goes on as after a game
     * Java played (the match's result, the table's end). The Career doesn't pay hosted games yet (GameHost.md, step F).
     *
     * @param timeLeft each player's priority time left in seconds, by player id, for the match's next game; null
     *                 without a timer
     * @return false when the game isn't hosted or the token is wrong
     */
    public static boolean ended(UUID gameId, String token, String winner, int turns, String reason,
                                JsonObject timeLeft) {
        Hosted hosted = HOSTED.get(gameId);
        if (hosted == null || token == null || !hosted.token.equals(token) || !HOSTED.remove(gameId, hosted)) {
            return false;
        }
        if (timeLeft != null) {
            for (Map.Entry<String, com.google.gson.JsonElement> e : timeLeft.entrySet()) {
                try {
                    Player p = hosted.game.getPlayer(UUID.fromString(e.getKey()));
                    if (p != null) {
                        // MatchImpl.endGame keeps it for the next game
                        p.setPriorityTimeLeft(e.getValue().getAsInt());
                    }
                } catch (RuntimeException ignore) {
                }
            }
        }
        return end(gameId, hosted, winner, turns, reason);
    }

    /**
     * A hosted game's join or quit that reached Java: the gateway routes those to the host, so the host doesn't have
     * the game (the gateway restarted, or the player came in around it). The game ends: as the quitter's loss, or as
     * a draw for a join, as a host that fails does.
     *
     * @return true when the game was hosted (and is now over), so Java has nothing to join or quit
     */
    public static boolean orphaned(UUID gameId, UUID userId, boolean quit) {
        Hosted h = HOSTED.get(gameId);
        if (h == null || !h.userId.equals(userId) || !HOSTED.remove(gameId, h)) {
            return false;
        }
        String winner = !quit ? null : h.game.getPlayers().keySet().stream().filter(id -> !id.equals(h.playerId))
                .findFirst().map(UUID::toString).orElse(null);
        logger.warn("Rust host: game " + gameId + " isn't on its host any more (" + (quit ? "quit" : "joined")
                + " through Java), ending it");
        end(gameId, h, winner, 0, quit ? "quit, its host is gone" : "its host is gone");
        return true;
    }

    /** ends the hosted games whose human is gone (above) as their loss */
    static void sweep() {
        long now = System.currentTimeMillis();
        for (Map.Entry<UUID, Hosted> e : HOSTED.entrySet()) {
            try {
                Hosted h = e.getValue();
                boolean here = h.managers.userManager().getUser(h.userId).map(User::hostsGames).orElse(false);
                if (here) {
                    h.goneSince = 0;
                } else if (h.goneSince == 0) {
                    h.goneSince = now;
                } else if (now - h.goneSince > GONE_MS && HOSTED.remove(e.getKey(), h)) {
                    String winner = h.game.getPlayers().keySet().stream().filter(id -> !id.equals(h.playerId))
                            .findFirst().map(UUID::toString).orElse(null);
                    logger.warn("Rust host: game " + e.getKey() + " lost its host, ending it");
                    end(e.getKey(), h, winner, 0, "quit, its host is gone");
                }
            } catch (RuntimeException ex) {
                logger.warn("Rust host: sweep: " + ex, ex);
            }
        }
    }

    private static boolean end(UUID gameId, Hosted hosted, String winner, int turns, String reason) {
        UUID winnerId = null;
        try {
            winnerId = winner == null || winner.isEmpty() ? null : UUID.fromString(winner);
        } catch (IllegalArgumentException ignore) {
        }
        // a draw leaves both without a result, as Java's draws do: no loser, so a match's next chooser is random
        if (winnerId != null) {
            for (Player p : hosted.game.getPlayers().values()) {
                p.setTechnicalResult(p.getId().equals(winnerId));
            }
        }
        if (turns > 0) {
            hosted.game.getState().setTurnNum(turns);
        }
        if (quitsMatch(reason)) {
            // the player left the match, not only this game, or ran out of time, which loses it: no next game
            // (MatchImpl.checkIfMatchEnds)
            UUID quitterId = hosted.playerId;
            if (reason.equals("timed out") && winnerId != null) {
                final UUID won = winnerId;
                quitterId = hosted.game.getPlayers().keySet().stream().filter(id -> !id.equals(won)).findFirst()
                        .orElse(hosted.playerId);
            }
            mage.game.match.MatchPlayer quitter = hosted.match.getPlayer(quitterId);
            if (quitter != null) {
                quitter.setQuit(true);
            }
        }
        logger.info("Rust host: game " + gameId + " ended after " + turns + " turns, " + reason + ", winner "
                + (winnerId == null ? "none (a draw)" : hosted.game.getPlayer(winnerId).getName()));
        hosted.managers.tableManager().endGameOnGameThread(hosted.tableId);
        return true;
    }

    /**
     * Whether a hosted game's end reason means the human left the match, not only the game: {@code matchQuit}, no
     * answer for 30 minutes, or a host that's gone with them ({@code quit}), and running out of time, which loses the
     * match as in Java ({@code timed out}); a concede ends only the game.
     */
    static boolean quitsMatch(String reason) {
        return reason != null && (reason.equals("quit") || reason.startsWith("quit,") || reason.equals("timed out"));
    }

    /** the human's {@code END_GAME_INFO}: the game's result and the match's score */
    private static void endGameInfo(Hosted hosted) {
        try {
            mage.game.Table table = hosted.managers.tableManager().getTable(hosted.tableId);
            if (table == null) {
                return;
            }
            mage.view.GameEndView view = new mage.view.GameEndView(hosted.game.getState(), hosted.game, hosted.playerId, table);
            hosted.managers.userManager().getUser(hosted.userId).ifPresent(user -> user.fireCallback(
                    new ClientCallback(ClientCallbackMethod.END_GAME_INFO, hosted.game.getId(), view)));
        } catch (RuntimeException e) {
            logger.warn("Rust host: game " + hosted.game.getId() + "'s end summary: " + e, e);
        }
    }
}
