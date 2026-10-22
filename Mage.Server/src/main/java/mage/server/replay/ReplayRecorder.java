package mage.server.replay;

import com.google.gson.JsonObject;
import mage.game.Game;
import mage.game.Table;
import mage.players.Player;
import mage.server.game.GameChangeKey;
import mage.server.websocket.rpc.JsonCodec;
import mage.view.CardsView;
import mage.view.ChatMessage;
import mage.view.GameView;
import org.apache.log4j.Logger;

import java.io.BufferedWriter;
import java.io.IOException;
import java.io.OutputStreamWriter;
import java.io.Writer;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentMap;
import java.util.zip.GZIPOutputStream;

/**
 * Records one game for replay, as a spectator sees it: a board snapshot (the watcher's game view) on every update and
 * every line of the game log, plus each human player's hand so they can review their own decisions later. Events
 * stream to disk as the game goes; the replay is listed once the game ends.
 */
public final class ReplayRecorder {

    private static final Logger logger = Logger.getLogger(ReplayRecorder.class);

    /** recorders by game chat, so log lines reach the recording of their game */
    private static final ConcurrentMap<UUID, ReplayRecorder> BY_CHAT = new ConcurrentHashMap<>();

    private final ReplayStore store;
    private final Game game;
    private final UUID chatId;
    private final ReplayInfo info;
    /** the recorder watches like a spectator nobody else can be */
    private final UUID watcherId = UUID.randomUUID();
    private Writer out;
    private String lastFrame;
    /** what the game looked like at the last snapshot, cheaply (see {@link GameChangeKey}) */
    private String lastKey;
    private boolean finished;
    /** the file couldn't be written */
    private boolean broken;

    private ReplayRecorder(ReplayStore store, Game game, UUID chatId, ReplayInfo info) {
        this.store = store;
        this.game = game;
        this.chatId = chatId;
        this.info = info;
    }

    /**
     * Starts recording a game, or does nothing (null) when replays are off, the game is a simulation or only AI
     * players play it. Call from the game thread or before the game thread starts.
     */
    public static ReplayRecorder start(Game game, Table table, UUID chatId) {
        try {
            ReplayStore store = ReplayStore.get();
            if (!store.settings().enabled || game == null || game.isSimulation()
                    || game.getPlayers().values().stream().noneMatch(Player::isHuman)) {
                return null;
            }
            ReplayInfo info = new ReplayInfo();
            info.gameId = game.getId().toString();
            info.startedAt = System.currentTimeMillis();
            info.gameType = game.getGameType() == null ? null : game.getGameType().getName();
            if (table != null) {
                info.tableId = table.getId().toString();
                info.tableName = table.getName();
                info.deckType = table.getDeckType();
            }
            for (Player player : game.getPlayers().values()) {
                info.players.add(new ReplayInfo.ReplayPlayer(player.getName(), player.isHuman()));
            }

            Files.createDirectories(store.settings().dir);
            ReplayRecorder recorder = new ReplayRecorder(store, game, chatId, info);
            recorder.out = new BufferedWriter(new OutputStreamWriter(
                    new GZIPOutputStream(Files.newOutputStream(store.partFile(info.gameId))), StandardCharsets.UTF_8));
            if (chatId != null) {
                BY_CHAT.put(chatId, recorder);
            }
            recorder.frame(true);
            return recorder;
        } catch (Throwable e) {
            logger.error("Can't record game " + (game == null ? null : game.getId()) + " for replay", e);
            return null;
        }
    }

    /**
     * Starts recording an AI-against-AI game of a player's batch (the matchup lab keeps the closest few): frames come
     * from the game's update events and log lines from its info events, since no game controller runs it.
     */
    public static ReplayRecorder startBatch(Game game, String name, String owner) {
        try {
            ReplayStore store = ReplayStore.get();
            if (!store.settings().enabled || game == null) {
                return null;
            }
            ReplayInfo info = new ReplayInfo();
            info.gameId = game.getId().toString();
            info.startedAt = System.currentTimeMillis();
            info.gameType = game.getGameType() == null ? null : game.getGameType().getName();
            info.tableName = name;
            info.batchOwner = owner;
            for (Player player : game.getPlayers().values()) {
                info.players.add(new ReplayInfo.ReplayPlayer(player.getName(), false));
            }
            Files.createDirectories(store.settings().dir);
            ReplayRecorder recorder = new ReplayRecorder(store, game, null, info);
            recorder.out = new BufferedWriter(new OutputStreamWriter(
                    new GZIPOutputStream(Files.newOutputStream(store.partFile(info.gameId))), StandardCharsets.UTF_8));
            game.addTableEventListener((mage.game.events.Listener<mage.game.events.TableEvent>) event -> {
                switch (event.getEventType()) {
                    case UPDATE:
                        recorder.frame();
                        break;
                    case INFO:
                    case STATUS:
                        recorder.log(new ChatMessage("", event.getMessage(), new java.util.Date(), game,
                                mage.view.ChatMessage.MessageColor.BLACK, mage.view.ChatMessage.MessageType.GAME, null));
                        break;
                    default:
                        break;
                }
            });
            recorder.frame(true);
            return recorder;
        } catch (Throwable e) {
            logger.error("Can't record batch game " + (game == null ? null : game.getId()), e);
            return null;
        }
    }

    public String gameId() {
        return info.gameId;
    }

    /** a game log line, from the chat of a game being recorded */
    public static void onLog(UUID chatId, ChatMessage message) {
        ReplayRecorder recorder = chatId == null ? null : BY_CHAT.get(chatId);
        if (recorder != null) {
            recorder.log(message);
        }
    }

    /** a board snapshot, unless nothing changed since the last one; game thread only */
    public synchronized void frame() {
        frame(false);
    }

    private void frame(boolean always) {
        if (!canWrite()) {
            return;
        }
        try {
            // building a spectator's view costs far more than this key: skip it while nothing visible has changed
            String key = GameChangeKey.of(game);
            if (!always && key.equals(lastKey)) {
                return;
            }
            lastKey = key;
            // written straight to text, {"view":...,"hands":{"name":...},"t":ms}: a JSON tree first would cost as
            // much again
            StringBuilder frame = new StringBuilder(32 * 1024).append("{\"view\":");
            JsonCodec.GSON.toJson(new GameView(game.getState(), game, null, watcherId), frame);
            frame.append(",\"hands\":{");
            boolean first = true;
            for (Player player : game.getPlayers().values()) {
                if (player.isHuman()) {
                    frame.append(first ? "" : ",").append(JsonCodec.GSON.toJson(player.getName())).append(':');
                    JsonCodec.GSON.toJson(new CardsView(game, player.getHand().getCards(game), player.getId()), frame);
                    first = false;
                }
            }
            frame.append('}');
            String snapshot = frame.toString();
            if (snapshot.equals(lastFrame)) {
                return;
            }
            lastFrame = snapshot;
            write(snapshot + ",\"t\":" + elapsed() + "}");
        } catch (Throwable e) {
            logger.warn("Replay of game " + info.gameId + " missed a snapshot: " + e);
        }
    }

    private synchronized void log(ChatMessage message) {
        if (!canWrite()) {
            return;
        }
        JsonObject event = new JsonObject();
        event.addProperty("t", elapsed());
        event.add("log", JsonCodec.GSON.toJsonTree(message));
        write(event.toString());
    }

    /** the game is over (or abandoned): the replay becomes available */
    public synchronized void finish(String result) {
        if (finished) {
            return;
        }
        if (result != null) {
            info.result = result;
        }
        frame(true);
        finished = true;
        if (chatId != null) {
            BY_CHAT.remove(chatId, this);
        }
        info.endedAt = System.currentTimeMillis();
        info.turns = game.getTurnNum();
        boolean closed = true;
        try {
            out.close();
        } catch (IOException e) {
            logger.error("Can't close the replay of game " + info.gameId, e);
            closed = false;
        }
        // an unclosed gzip stream is unreadable: such a recording stays a .part file and is cleaned up later
        if (closed && info.events > 0) {
            store.complete(info);
        }
    }

    private boolean canWrite() {
        if (finished || broken) {
            return false;
        }
        if (info.events >= ReplaySettings.MAX_EVENTS_PER_GAME) {
            info.truncated = true;
            return false;
        }
        return true;
    }

    private void write(String event) {
        try {
            out.write(event);
            out.write('\n');
            info.events++;
        } catch (IOException e) {
            // nothing more can be written; finish() still lists what was recorded
            logger.error("Replay of game " + info.gameId + " stopped recording: " + e);
            info.truncated = true;
            broken = true;
        }
    }

    private long elapsed() {
        return System.currentTimeMillis() - info.startedAt;
    }
}
