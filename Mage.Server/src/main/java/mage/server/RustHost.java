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
 * Games the Rust host plays instead of Java (docs/rust/GameHost.md): a duel of a player against the AI, or of two
 * players, every card in the Rust engine, each player listed or opted in and connected through the gateway (both
 * through the same one, for two). Java keeps the table and the match: at each game's start it sends the gateway the
 * seats and decks ({@code HOST_GAME}, on each human's connection, which the client never sees) instead of starting
 * its own game, and the gateway reports the end ({@code hostGameEnded}), which ends the game here as if Java had played
 * it. A match of more than one game is a hosted game at a time: Java records each result, runs sideboarding, and hands
 * the next game to the host again, with the last game's loser choosing who plays first. At a solo table the host sends
 * the game's line at each turn's start ({@code hostGameTurn}), which the solo saves keep, and a save carried on goes
 * back to the host with it ({@link GameOptions#hostedLine}).
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
        /** each human's user id, and their player ids in the same order */
        final List<UUID> userIds;
        final List<UUID> playerIds;
        /** when the host was handed the game, for its replay */
        final long startedAt = System.currentTimeMillis();
        /** what the host recorded of the game for its replay (summary.replayEvents) */
        volatile int replayEvents;
        /** when each player last sent an emote, for the rate limit (GameController.EMOTE_GAP_MS) */
        final Map<UUID, Long> lastEmote = new ConcurrentHashMap<>();
        /** when the sweep last found a human gone or connected without a hosting gateway, or 0 */
        volatile long goneSince;

        Hosted(UUID tableId, Match match, Game game, String token, ManagerFactory managers, List<UUID> userIds,
               List<UUID> playerIds) {
            this.tableId = tableId;
            this.match = match;
            this.game = game;
            this.token = token;
            this.managers = managers;
            this.userIds = userIds;
            this.playerIds = playerIds;
        }

        /** the player id of a user of the game, or null */
        UUID playerOf(UUID userId) {
            int i = userIds.indexOf(userId);
            return i < 0 ? null : playerIds.get(i);
        }

        /** the other seat's player id */
        UUID otherThan(UUID playerId) {
            return game.getPlayers().keySet().stream().filter(id -> !id.equals(playerId)).findFirst().orElse(null);
        }
    }

    private static final Map<UUID, Hosted> HOSTED = new ConcurrentHashMap<>();

    /**
     * A game whose human has been gone, or connected without a hosting gateway, this long ends as their loss (as a
     * draw when both players of a game between people are): its host (and the gateway with it) is gone, or would have
     * ended it itself (the host waits 30 minutes for an answer).
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

    /**
     * Whether a game the Rust host played can be carried on now (a save or a fork of it): the host is on and the
     * player connects through a gateway that hosts games. A hosted game's save can only be played on the host.
     */
    public static boolean canCarryOn(User user) {
        return enabled() && user != null && user.hostsGames();
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

    private static boolean isTournamentTable(ManagerFactory managers, UUID tableId) {
        mage.game.Table table = managers.tableManager().getTable(tableId);
        return table != null && (table.isTournament() || table.isTournamentSubTable());
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
                        GameOptions options, UUID chooser, java.util.function.Function<UUID, Integer> skillOf) {
        try {
            Game game = match.getGame();
            if (!enabled() || game == null || userPlayerMap.isEmpty() || userPlayerMap.size() > 2) {
                return false;
            }
            // a saved hosted game carried on goes to the host whoever plays it: Java has no game to carry on
            JsonObject line = parseLine(options.hostedLine);
            List<User> users = new ArrayList<>();
            List<UUID> userIds = new ArrayList<>();
            List<UUID> playerIds = new ArrayList<>();
            for (Map.Entry<UUID, UUID> e : userPlayerMap.entrySet()) {
                Optional<User> user = managers.userManager().getUser(e.getKey());
                if (!user.isPresent() || (line == null && !forUser(user.get()))) {
                    return false;
                }
                users.add(user.get());
                userIds.add(e.getKey());
                playerIds.add(e.getValue());
            }
            String names = users.stream().map(User::getName).reduce((a, b) -> a + " and " + b).orElse("");
            String why = null;
            if (match.getPlayers().size() != 2) {
                why = "not a duel";
            } else if (options.setup != null || options.resumeFrom != null) {
                why = "it starts from a setup or a save";
            } else if (mage.server.career.CareerService.get().isCareerTable(tableId)) {
                why = "a Career table";
            } else if (isTournamentTable(managers, tableId)) {
                // tournaments and drafts play limited pools, which need more cards in the engine first
                why = "a tournament's table";
            } else if (users.stream().anyMatch(u -> !u.hostsGames())) {
                why = "a player isn't connected through a gateway that hosts games";
            } else if (users.size() == 2 && (users.get(0).gameHostId() == null
                    || !users.get(0).gameHostId().equals(users.get(1).gameHostId()))) {
                why = "the players aren't connected through the same gateway";
            } else {
                why = unplayedOption(match, game);
            }
            if (why != null) {
                logger.info("Rust host: " + names + "'s game stays on Java: " + why);
                return false;
            }
            List<String> cards = new ArrayList<>();
            List<JsonObject> seatList = new ArrayList<>();
            Integer aiSkill = null;
            for (MatchPlayer p : match.getPlayers()) {
                Player player = game.getPlayer(p.getPlayer().getId());
                if (p.getDeck() == null || player == null) {
                    return false;
                }
                int human = playerIds.indexOf(p.getPlayer().getId());
                boolean isHuman = human >= 0;
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
                    cards.add(c.getName());
                }
                seat.add("deck", deck);
                for (Card c : p.getDeck().getSideboard()) {
                    if (c.getAbilities().stream().anyMatch(a -> a instanceof mage.abilities.keyword.CompanionAbility)) {
                        logger.info("Rust host: " + names + "'s game stays on Java: a companion (" + c.getName() + ")");
                        return false;
                    }
                }
                if (!isHuman && skillOf != null && skillOf.apply(player.getId()) != null) {
                    aiSkill = skillOf.apply(player.getId());
                }
                seat.add("printings", printings);
                if (isHuman) {
                    seat.add("userData", JsonCodec.GSON.toJsonTree(users.get(human).getUserData()));
                }
                seatList.add(seat);
            }
            if (line != null) {
                // the seats in the line's order (the host plays the line's game again, seat for seat)
                List<String> order = new ArrayList<>();
                for (com.google.gson.JsonElement s : line.getAsJsonObject("start").getAsJsonArray("seats")) {
                    order.add(s.getAsJsonObject().get("name").getAsString());
                }
                seatList.sort(java.util.Comparator.comparingInt(s -> order.indexOf(s.get("name").getAsString())));
            }
            JsonArray seats = new JsonArray();
            seatList.forEach(seats::add);
            Set<String> missing = RustAiBridge.missing(cards);
            if (!missing.isEmpty()) {
                logger.info("Rust host: " + names + "'s game doesn't qualify, the Rust engine lacks "
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
            // the table's options the host plays by (the others keep the game on Java: unplayedOption)
            data.addProperty("startingLife", game.getStartingLife());
            data.addProperty("freeMulligans", game.getMulligan().getFreeMulligans());
            data.addProperty("rollbackTurns", options.rollbackTurnsAllowed);
            if (aiSkill != null) {
                data.addProperty("aiSkill", aiSkill);
            }
            // the match's priority timer, which the host runs as GameController does (0: none)
            data.addProperty("priorityTime", match.getOptions().getMatchTimeLimit().getPrioritySecs());
            data.addProperty("bufferTime", match.getOptions().getMatchBufferTime().getBufferSecs());
            // a solo table's game saves itself at each turn's start: the host sends its line (hostGameTurn)
            data.addProperty("lines", mage.server.solo.SoloGames.get().isSolo(tableId));
            if (line != null) {
                data.add("prelude", line);
            }
            Hosted hosted = new Hosted(tableId, match, game, token, managers, userIds, playerIds);
            // the game's end summary (the match's score, what the client shows between games), as GameController
            // sends it for a game Java plays: MatchImpl.endGame fires it once the result is in
            game.addTableEventListener(event -> {
                if (event.getEventType() == mage.game.events.TableEvent.EventType.END_GAME_INFO) {
                    endGameInfo(hosted);
                }
            });
            HOSTED.put(game.getId(), hosted);
            // on each player's connection: the gateway starts the game once, and each knows it before its START_GAME
            for (User user : users) {
                user.fireCallback(new ClientCallback(ClientCallbackMethod.HOST_GAME, game.getId(), data));
            }
            logger.info("Rust host: plays " + names + "'s game " + game.getId() + " (seed " + seed
                    + (line != null ? ", carried on from turn " + line.get("turn") : "") + ")");
            return true;
        } catch (RuntimeException e) {
            logger.warn("Rust host: the game stays on Java: " + e, e);
            return false;
        }
    }

    private static JsonObject parseLine(String line) {
        if (line == null || line.isEmpty()) {
            return null;
        }
        try {
            return com.google.gson.JsonParser.parseString(line).getAsJsonObject();
        } catch (RuntimeException e) {
            logger.warn("Rust host: a saved game's line can't be read: " + e);
            return null;
        }
    }

    /**
     * Why a game stays on Java for an option the host doesn't play, or null: another game type than a plain duel
     * (Commander, Brawl… have a command zone), a mulligan other than London, another starting hand size, emblems or
     * planes the table adds.
     */
    static String unplayedOption(Match match, Game game) {
        mage.game.match.MatchOptions o = match.getOptions();
        if (o.getGameType() != null && !o.getGameType().equals("Two Player Duel")) {
            return "its game type (" + o.getGameType() + ")";
        }
        mage.game.mulligan.Mulligan mulligan = game.getMulligan();
        if (!(mulligan instanceof mage.game.mulligan.LondonMulligan)
                || mulligan instanceof mage.game.mulligan.SmoothedLondonMulligan) {
            return "its mulligan (" + (mulligan == null ? "none" : mulligan.getClass().getSimpleName()) + ")";
        }
        if (o.isCustomStartHandSizeEnabled() && o.getCustomStartHandSize() != 7) {
            return "its starting hand size";
        }
        if (o.isPlaneChase() || !o.getPerPlayerEmblemCards().isEmpty() || !o.getGlobalEmblemCards().isEmpty()) {
            return "its planes or emblems";
        }
        return null;
    }

    /**
     * A user who reconnects gets the start of each hosted game they're in again, as {@link User#onReconnect} sends
     * the games Java plays; the client joins it, and the gateway moves the game's callbacks to the new connection.
     */
    static void reconnect(User user) {
        for (Map.Entry<UUID, Hosted> e : HOSTED.entrySet()) {
            Hosted h = e.getValue();
            UUID playerId = h.playerOf(user.getId());
            if (playerId == null) {
                continue;
            }
            mage.game.Table table = h.managers.tableManager().getTable(h.tableId);
            if (table != null) {
                user.ccGameStarted(table.getId(), table.getParentTableId(), e.getKey(), playerId);
            }
        }
    }

    /**
     * Leaves a hosted game for a player (a solo save, "Try this turn again"): the gateway has the host end it as their
     * quit, which it then reports. False when the game isn't hosted, or not theirs.
     */
    public static boolean leave(UUID gameId, UUID userId) {
        Hosted h = gameId == null ? null : HOSTED.get(gameId);
        if (h == null || h.playerOf(userId) == null) {
            return false;
        }
        h.managers.userManager().getUser(userId).ifPresent(user ->
                user.fireCallback(new ClientCallback(ClientCallbackMethod.HOST_GAME_QUIT, gameId, null)));
        return true;
    }

    /**
     * The host's line of a solo table's game at a turn's start ({@code hostGameTurn}): the solo saves keep it, as they
     * keep a Java game's turn snapshots.
     *
     * @return false when the game isn't hosted or the token is wrong
     */
    public static boolean turn(UUID gameId, String token, int turn, String activePlayerId, JsonObject line) {
        Hosted h = HOSTED.get(gameId);
        if (h == null || token == null || !h.token.equals(token) || line == null) {
            return false;
        }
        UUID active = null;
        try {
            active = activePlayerId == null || activePlayerId.isEmpty() ? null : UUID.fromString(activePlayerId);
        } catch (IllegalArgumentException ignore) {
        }
        mage.server.solo.SoloGames.get().hostedTurn(h.tableId, gameId, turn, active, line.toString());
        return true;
    }

    /**
     * The host's report of a game's end: the game ends here with that result, and the table goes on as after a game
     * Java played (the match's result, the table's end, the Career's pay from the host's tally).
     *
     * @param timeLeft each player's priority time left in seconds, by player id, for the match's next game; null
     *                 without a timer
     * @param summary  each seat's life and poison at the end ({@code players}, by seat) and each human's Career tally
     *                 ({@code tallies}, by player id; {@code tally}, the first human's); null when the game never
     *                 started
     * @return false when the game isn't hosted or the token is wrong
     */
    public static boolean ended(UUID gameId, String token, String winner, int turns, String reason,
                                JsonObject timeLeft, JsonObject summary) {
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
        if (summary != null) {
            summarize(hosted, summary);
        }
        return end(gameId, hosted, winner, turns, reason);
    }

    /**
     * The host's end summary: each human's Career tally, kept for the game's end as the game's watcher would have left
     * it, so the table pays the game as one Java played. (Java's copy of the game isn't touched: its life totals are
     * the start's, which nothing reads at the end; the Career reads the tally's own.)
     */
    static void summarize(Hosted hosted, JsonObject summary) {
        try {
            if (summary.has("replayEvents")) {
                hosted.replayEvents = summary.get("replayEvents").getAsInt();
            }
            if (Main.isTestMode()
                    || !mage.server.career.CareerService.enabled(hosted.managers.configSettings().isAuthenticationActivated())) {
                return;
            }
            JsonObject tallies = summary.has("tallies") && summary.get("tallies").isJsonObject()
                    ? summary.getAsJsonObject("tallies") : null;
            for (UUID playerId : hosted.playerIds) {
                JsonObject tally = null;
                if (tallies != null && tallies.has(playerId.toString()) && tallies.get(playerId.toString()).isJsonObject()) {
                    tally = tallies.getAsJsonObject(playerId.toString());
                } else if (tallies == null && playerId.equals(hosted.playerIds.get(0)) && summary.has("tally")
                        && summary.get("tally").isJsonObject()) {
                    // a host from before games between people: the one human's
                    tally = summary.getAsJsonObject("tally");
                }
                if (tally != null) {
                    mage.server.career.CareerTally.fromHost(hosted.game.getId(), playerId, tally);
                }
            }
        } catch (RuntimeException e) {
            logger.warn("Rust host: game " + hosted.game.getId() + "'s end summary: " + e, e);
        }
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
        UUID playerId = h == null ? null : h.playerOf(userId);
        if (playerId == null || !HOSTED.remove(gameId, h)) {
            return false;
        }
        UUID other = h.otherThan(playerId);
        String winner = !quit || other == null ? null : other.toString();
        logger.warn("Rust host: game " + gameId + " isn't on its host any more (" + (quit ? "quit" : "joined")
                + " through Java), ending it");
        end(gameId, h, winner, 0, quit ? "quit, its host is gone" : "its host is gone");
        return true;
    }

    /**
     * An emote in a hosted game (gameEmote): the host has no Java GameController, so the emote goes to the game's
     * players here, as GameController.emote sends it (the AI has no client).
     *
     * @return null when the game isn't hosted; otherwise whether it was sent
     */
    public static Boolean emote(UUID gameId, UUID userId, String emote, long now) {
        Hosted h = HOSTED.get(gameId);
        if (h == null) {
            return null;
        }
        UUID playerId = h.playerOf(userId);
        Long last = h.lastEmote.get(userId);
        if (playerId == null || last != null && now - last < mage.server.game.GameController.EMOTE_GAP_MS) {
            return false;
        }
        h.lastEmote.put(userId, now);
        mage.players.Player player = h.game.getPlayer(playerId);
        mage.server.game.PlayerEmote sent = new mage.server.game.PlayerEmote(gameId.toString(), player == null ? "" : player.getName(), emote);
        for (UUID to : h.userIds) {
            h.managers.userManager().getUser(to).ifPresent(user -> user.fireCallback(new mage.interfaces.callback.ClientCallback(
                    mage.interfaces.callback.ClientCallbackMethod.GAME_EMOTE, gameId, sent)));
        }
        return true;
    }

    /** ends the hosted games whose humans are gone (above): as the gone one's loss, or a draw when both are */
    static void sweep() {
        long now = System.currentTimeMillis();
        for (Map.Entry<UUID, Hosted> e : HOSTED.entrySet()) {
            try {
                Hosted h = e.getValue();
                List<UUID> gone = new ArrayList<>();
                for (int i = 0; i < h.userIds.size(); i++) {
                    boolean here = h.managers.userManager().getUser(h.userIds.get(i)).map(User::hostsGames).orElse(false);
                    if (!here) {
                        gone.add(h.playerIds.get(i));
                    }
                }
                if (gone.isEmpty()) {
                    h.goneSince = 0;
                } else if (h.goneSince == 0) {
                    h.goneSince = now;
                } else if (now - h.goneSince > GONE_MS && HOSTED.remove(e.getKey(), h)) {
                    UUID winner = gone.size() == 1 ? h.otherThan(gone.get(0)) : null;
                    logger.warn("Rust host: game " + e.getKey() + " lost its host, ending it");
                    end(e.getKey(), h, winner == null ? null : winner.toString(), 0, "quit, its host is gone");
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
            // the loser left the match, not only this game, or ran out of time, which loses it: no next game
            // (MatchImpl.checkIfMatchEnds). In a draw (a host gone with both players), both left.
            List<UUID> quitters = new ArrayList<>();
            if (winnerId != null) {
                quitters.add(hosted.otherThan(winnerId));
            } else {
                quitters.addAll(hosted.playerIds);
            }
            for (UUID quitterId : quitters) {
                mage.game.match.MatchPlayer quitter = quitterId == null ? null : hosted.match.getPlayer(quitterId);
                if (quitter != null) {
                    quitter.setQuit(true);
                }
            }
        }
        logger.info("Rust host: game " + gameId + " ended after " + turns + " turns, " + reason + ", winner "
                + (winnerId == null ? "none (a draw)" : hosted.game.getPlayer(winnerId).getName()));
        // its replay, which the host wrote where Java keeps its games' (GameImpl.getWinner's result)
        String result = winnerId == null ? "Game is a draw" : "Player " + hosted.game.getPlayer(winnerId).getName() + " is the winner";
        mage.server.replay.ReplayRecorder.hosted(hosted.game, hosted.managers.tableManager().getTable(hosted.tableId),
                hosted.startedAt, result, turns, hosted.replayEvents);
        hosted.managers.tableManager().endGameOnGameThread(hosted.tableId);
        return true;
    }

    /**
     * Whether a hosted game's end reason means the loser left the match, not only the game: {@code matchQuit}, no
     * answer for 30 minutes, or a host that's gone with them ({@code quit}), and running out of time, which loses the
     * match as in Java ({@code timed out}); a concede ends only the game.
     */
    static boolean quitsMatch(String reason) {
        return reason != null && (reason.equals("quit") || reason.startsWith("quit,") || reason.equals("timed out"));
    }

    /** each human's {@code END_GAME_INFO}: the game's result and the match's score */
    private static void endGameInfo(Hosted hosted) {
        try {
            mage.game.Table table = hosted.managers.tableManager().getTable(hosted.tableId);
            if (table == null) {
                return;
            }
            for (int i = 0; i < hosted.userIds.size(); i++) {
                UUID playerId = hosted.playerIds.get(i);
                mage.view.GameEndView view = new mage.view.GameEndView(hosted.game.getState(), hosted.game, playerId, table);
                hosted.managers.userManager().getUser(hosted.userIds.get(i)).ifPresent(user -> user.fireCallback(
                        new ClientCallback(ClientCallbackMethod.END_GAME_INFO, hosted.game.getId(), view)));
            }
        } catch (RuntimeException e) {
            logger.warn("Rust host: game " + hosted.game.getId() + "'s end summary: " + e, e);
        }
    }
}
