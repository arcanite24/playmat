package mage.server.solo;

import mage.cards.decks.DeckCardLists;
import mage.game.Game;
import mage.game.GameOptions;
import mage.game.GameSnapshot;
import mage.game.Seat;
import mage.game.Table;
import mage.game.match.Match;
import mage.game.match.MatchOptions;
import mage.game.match.MatchPlayer;
import mage.players.PlayerType;
import mage.server.TableController;
import mage.server.career.CareerService;
import mage.server.util.PluginUtil;
import org.apache.log4j.Logger;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.ObjectInputStream;
import java.io.ObjectOutputStream;
import java.io.ObjectStreamClass;
import java.io.Serializable;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;

/**
 * Solo games against the AI (S002 and S006): every one saves itself at the start of each turn, so it survives a server
 * restart and can be resumed; a free-play game can also be saved on purpose ("save and leave") or forked into a new
 * table from the start of the current turn ("try this turn again").
 * <p>
 * A solo game is one human against one or more AI players, outside tournaments. The latest snapshot is kept in memory
 * and written to {@link SoloStore} on one writer thread; a newer snapshot replaces an older one still waiting to be
 * written. Career games save too, but only for crash recovery: keeping a save or forking would be a free take-back.
 */
public final class SoloGames {

    private static final Logger logger = Logger.getLogger(SoloGames.class);
    private static final SoloGames INSTANCE = new SoloGames();

    public static final String AUTO = "auto";
    public static final String KEPT = "kept";
    /** saves older than this go */
    public static final long RETENTION_MS = TimeUnit.DAYS.toMillis(30);
    /** a player's newest saves kept; older ones go */
    public static final int SAVES_PER_USER = 20;

    /** How a solo table was set up, to set it up again. */
    public static class SavedTable implements Serializable {
        private static final long serialVersionUID = 1L;
        public String user;
        public String title;
        public MatchOptions options;
        public final List<SavedSeat> seats = new ArrayList<>();
        public CareerService.CareerTable career;
    }

    public static class SavedSeat implements Serializable {
        private static final long serialVersionUID = 1L;
        public String name;
        public PlayerType type;
        public int skill;
        public boolean human;
        public DeckCardLists deck;
        public int wins;
        /** the player's id in the saved game */
        public UUID playerId;
    }

    /** A solo table running now. */
    private static final class Live {
        final UUID tableId;
        final SavedTable info;
        volatile GameSnapshot latest;
        volatile UUID gameId;
        /** kept on purpose: no more writes, and the game ending doesn't remove it */
        volatile boolean kept;

        Live(UUID tableId, SavedTable info) {
            this.tableId = tableId;
            this.info = info;
        }
    }

    /** A resume waiting for its new table to start: the saved table and the save it came from. */
    public static final class Resume {
        public final SavedTable info;
        public final GameSnapshot snapshot;
        /** the save to remove once the game is running again; null for a fork */
        public final String saveId;

        public Resume(SavedTable info, GameSnapshot snapshot, String saveId) {
            this.info = info;
            this.snapshot = snapshot;
            this.saveId = saveId;
        }
    }

    private final Map<UUID, Live> live = new ConcurrentHashMap<>();
    private final Map<UUID, Resume> pending = new ConcurrentHashMap<>();
    /** tables waiting for the writer, newest snapshot wins */
    private final Map<UUID, Live> dirty = new ConcurrentHashMap<>();
    private final ExecutorService writer = Executors.newSingleThreadExecutor(r -> {
        Thread thread = new Thread(r, "solo-save-writer");
        thread.setDaemon(true);
        return thread;
    });
    private volatile boolean pruned;

    private SoloGames() {
        GameSnapshot.setLoader(PluginUtil.classLoader);
    }

    public static SoloGames get() {
        return INSTANCE;
    }

    // ---- the table hooks

    /**
     * A game is about to start at a table: when it's a solo table, its turns save themselves, and a resume waiting
     * for the table carries on from its save.
     */
    public void prepareGame(TableController controller, GameOptions gameOptions) {
        try {
            Table table = controller.getTable();
            Match match = controller.getMatch();
            if (table == null || match == null || table.isTournamentSubTable() || table.isTournament()) {
                return;
            }
            SavedTable info = describe(controller, table, match);
            if (info == null) {
                return;
            }
            UUID tableId = table.getId();
            Resume resume = pending.remove(tableId);
            if (resume != null) {
                applyResume(resume, match, gameOptions, info);
            }
            Live entry = new Live(tableId, info);
            entry.gameId = match.getGame() == null ? null : match.getGame().getId();
            live.put(tableId, entry);
            gameOptions.turnSnapshots = snapshot -> snapshotTaken(entry, snapshot);
        } catch (RuntimeException e) {
            // saving is a convenience: never stop a game over it
            logger.warn("Solo: the game at table " + controller.getTable().getId() + " won't save itself", e);
        }
    }

    /** a game at a table ended: its automatic save goes (a kept one stays) */
    public void gameEnded(UUID tableId, Game game) {
        Live entry = live.remove(tableId);
        if (entry == null) {
            return;
        }
        dirty.remove(tableId);
        if (!entry.kept) {
            writer.submit(() -> {
                try {
                    SoloStore.get().deleteGame(tableId.toString());
                } catch (SQLException | RuntimeException e) {
                    logger.warn("Solo: an ended game's save couldn't be removed", e);
                }
            });
        }
    }

    // ---- what players do with saves

    /** whether a table is a solo table running now */
    public boolean isSolo(UUID tableId) {
        return tableId != null && live.containsKey(tableId);
    }

    public boolean isRunning(String saveId) {
        try {
            return live.containsKey(UUID.fromString(saveId));
        } catch (IllegalArgumentException e) {
            return false;
        }
    }

    /**
     * "Save and leave": the table's latest save stays after the player leaves the game. Free play only.
     *
     * @return null when kept, else why not
     */
    public String keep(UUID tableId, String user) {
        Live entry = tableId == null ? null : live.get(tableId);
        if (entry == null || !entry.info.user.equals(user)) {
            return "This isn't one of your games against the computer.";
        }
        if (entry.info.career != null) {
            return "Career games save themselves, but can't be kept: a save is a free take-back.";
        }
        GameSnapshot snapshot = entry.latest;
        if (snapshot == null) {
            return "The game saves at the start of each turn; there's nothing to keep before the first turn.";
        }
        entry.kept = true;
        dirty.remove(tableId);
        try {
            write(entry, snapshot, KEPT);
        } catch (SQLException | IOException | RuntimeException e) {
            entry.kept = false;
            logger.warn("Solo: a save couldn't be kept", e);
            return "The game couldn't be saved: " + e.getMessage();
        }
        // the player leaves it now: the save lists as resumable at once, not once the table has closed
        live.remove(tableId);
        return null;
    }

    /**
     * What a fork of a running free-play game starts from: the start of its current turn.
     *
     * @throws IllegalArgumentException when the game can't be forked, with why
     */
    public Resume forkOf(UUID tableId, String user) {
        Live entry = tableId == null ? null : live.get(tableId);
        if (entry == null || !entry.info.user.equals(user)) {
            throw new IllegalArgumentException("This isn't one of your games against the computer.");
        }
        if (entry.info.career != null) {
            throw new IllegalArgumentException("Career games can't be replayed from a turn.");
        }
        GameSnapshot snapshot = entry.latest;
        if (snapshot == null) {
            throw new IllegalArgumentException("There's no turn to go back to yet.");
        }
        return new Resume(entry.info, snapshot, null);
    }

    /** the table id a running game is at, for a player of it */
    public UUID tableOfGame(UUID gameId) {
        if (gameId == null) {
            return null;
        }
        for (Live entry : live.values()) {
            if (gameId.equals(entry.gameId)) {
                return entry.tableId;
            }
        }
        return null;
    }

    /**
     * A save to resume.
     *
     * @throws IllegalArgumentException when it can't be, with why
     */
    public Resume load(String saveId, String user) throws SQLException {
        if (isRunning(saveId)) {
            throw new IllegalArgumentException("That game is still running: rejoin it from your open games.");
        }
        byte[][] row = SoloStore.get().loadGame(saveId, user);
        if (row == null) {
            throw new IllegalArgumentException("That save is gone.");
        }
        try {
            SavedTable info = (SavedTable) readObject(row[0]);
            GameSnapshot snapshot = GameSnapshot.fromBytes(row[1]);
            return new Resume(info, snapshot, saveId);
        } catch (IOException | ClassNotFoundException | ClassCastException e) {
            logger.warn("Solo: save " + saveId + " can't be read", e);
            throw new IllegalArgumentException("That save can't be read on this server version.");
        }
    }

    /** a new table about to start will carry on from {@code resume} */
    public void expectResume(UUID tableId, Resume resume) {
        pending.put(tableId, resume);
        if (resume.info.career != null) {
            CareerService.get().adopt(tableId, resume.info.career);
        }
    }

    /** a table set up for a resume was abandoned before it started */
    public void cancelResume(UUID tableId) {
        if (tableId != null) {
            pending.remove(tableId);
        }
    }

    public List<SoloStore.SavedRow> list(String user) throws SQLException {
        prune();
        return SoloStore.get().savedGames(user);
    }

    public boolean delete(String saveId, String user) throws SQLException {
        if (isRunning(saveId)) {
            throw new IllegalArgumentException("That game is still running.");
        }
        return SoloStore.get().deleteGame(saveId, user);
    }

    // ---- inside

    private static SavedTable describe(TableController controller, Table table, Match match) {
        MatchOptions options = controller.getOptions();
        if (options == null) {
            return null;
        }
        SavedTable info = new SavedTable();
        int humans = 0;
        int computers = 0;
        Seat[] seats = table.getSeats();
        for (Seat seat : seats) {
            if (seat.getPlayer() == null) {
                return null;
            }
            MatchPlayer matchPlayer = match.getPlayer(seat.getPlayer().getId());
            if (matchPlayer == null || matchPlayer.getDeck() == null) {
                return null;
            }
            SavedSeat saved = new SavedSeat();
            saved.name = seat.getPlayer().getName();
            saved.type = seat.getPlayerType();
            saved.human = seat.getPlayer().isHuman();
            Integer skill = controller.skillOf(seat.getPlayer().getId());
            saved.skill = skill == null ? 1 : skill;
            saved.deck = matchPlayer.getDeck().prepareCardsOnlyDeck();
            saved.wins = matchPlayer.getWins();
            saved.playerId = seat.getPlayer().getId();
            if (saved.human) {
                humans++;
                info.user = saved.name;
            } else {
                computers++;
            }
            info.seats.add(saved);
        }
        if (humans != 1 || computers < 1) {
            return null;
        }
        info.options = options;
        info.title = options.getName();
        info.career = CareerService.get().tableInfo(table.getId());
        return info;
    }

    private static void applyResume(Resume resume, Match match, GameOptions gameOptions, SavedTable now) {
        Map<UUID, UUID> savedToNew = new HashMap<>();
        Map<String, UUID> byName = new HashMap<>();
        for (SavedSeat seat : now.seats) {
            byName.put(seat.name, seat.playerId);
        }
        for (SavedSeat seat : resume.info.seats) {
            UUID newId = byName.get(seat.name);
            if (newId == null) {
                throw new IllegalStateException("The resumed table has no seat for " + seat.name);
            }
            savedToNew.put(seat.playerId, newId);
            MatchPlayer matchPlayer = match.getPlayer(newId);
            // match wins from the games before this one
            if (matchPlayer != null) {
                for (int i = matchPlayer.getWins(); i < seat.wins; i++) {
                    matchPlayer.addWin();
                }
            }
        }
        gameOptions.resumeFrom = resume.snapshot.withPlayers(savedToNew);
        // the snapshot carries its own random state
        gameOptions.seed = null;
        if (resume.saveId != null) {
            String saveId = resume.saveId;
            INSTANCE.writer.submit(() -> {
                try {
                    SoloStore.get().deleteGame(saveId);
                } catch (SQLException | RuntimeException e) {
                    logger.warn("Solo: a resumed save couldn't be removed", e);
                }
            });
        }
    }

    private void snapshotTaken(Live entry, GameSnapshot snapshot) {
        if (entry.kept) {
            return;
        }
        entry.latest = snapshot;
        if (dirty.put(entry.tableId, entry) == null) {
            writer.submit(() -> {
                Live next = dirty.remove(entry.tableId);
                if (next == null || next.kept || !live.containsKey(entry.tableId)) {
                    return;
                }
                try {
                    write(next, next.latest, AUTO);
                } catch (SQLException | IOException | RuntimeException e) {
                    logger.warn("Solo: a turn couldn't be saved", e);
                }
            });
        }
    }

    private void write(Live entry, GameSnapshot snapshot, String kind) throws SQLException, IOException {
        SavedTable info = entry.info;
        StringBuilder opponents = new StringBuilder();
        for (SavedSeat seat : info.seats) {
            if (!seat.human) {
                if (opponents.length() > 0) {
                    opponents.append(", ");
                }
                opponents.append(seat.name);
            }
        }
        SoloStore.get().saveGame(entry.tableId.toString(), info.user, kind, info.career != null, snapshot.getTurnNum(),
                info.title == null ? "" : info.title, opponents.toString(),
                info.options.getGameType() == null ? "" : info.options.getGameType(),
                writeObject(info), snapshot.toBytes());
    }

    private void prune() {
        if (pruned) {
            return;
        }
        pruned = true;
        try {
            int removed = SoloStore.get().pruneGames(System.currentTimeMillis() - RETENTION_MS, SAVES_PER_USER);
            if (removed > 0) {
                logger.info("Solo: removed " + removed + " old saved games");
            }
        } catch (SQLException | RuntimeException e) {
            logger.warn("Solo: old saves couldn't be removed", e);
        }
    }

    /** waits for queued writes (tests, shutdown) */
    public void flush() {
        try {
            writer.submit(() -> null).get(10, TimeUnit.SECONDS);
        } catch (Exception e) {
            logger.warn("Solo: waiting for saves failed", e);
        }
    }

    static byte[] writeObject(Object value) throws IOException {
        ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        try (ObjectOutputStream out = new ObjectOutputStream(bytes)) {
            out.writeObject(value);
        }
        return bytes.toByteArray();
    }

    static Object readObject(byte[] data) throws IOException, ClassNotFoundException {
        try (ObjectInputStream in = new ObjectInputStream(new ByteArrayInputStream(data)) {
            @Override
            protected Class<?> resolveClass(ObjectStreamClass desc) throws IOException, ClassNotFoundException {
                try {
                    return Class.forName(desc.getName(), false, PluginUtil.classLoader);
                } catch (ClassNotFoundException e) {
                    return super.resolveClass(desc);
                }
            }
        }) {
            return in.readObject();
        }
    }
}
