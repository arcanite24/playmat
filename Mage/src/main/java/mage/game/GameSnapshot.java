package mage.game;

import mage.cards.Card;
import mage.cards.MeldCard;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.ObjectInputStream;
import java.io.ObjectOutputStream;
import java.io.ObjectStreamClass;
import java.io.Serializable;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.zip.GZIPInputStream;
import java.util.zip.GZIPOutputStream;

/**
 * A game as it stood at the start of a turn, frozen into bytes: the whole game state (zones, permanents, effects,
 * triggers, watchers, players, the game's random generator) and every card object. A new table can carry on from it
 * (see {@link GameOptions#resumeFrom}), which is how a solo game survives a server restart and how a game forks into a
 * what-if copy.
 * <p>
 * The game picks up at the start of that turn, the way a turn rollback does: the engine's game loop restarts cleanly
 * there and nowhere else. The new table's players get new ids; {@link #withPlayers(Map)} maps the saved ids onto them.
 */
public final class GameSnapshot implements Serializable {

    private static final long serialVersionUID = 1L;
    // bump when the payload layout changes: older saves are then refused instead of half-read
    public static final int FORMAT = 1;

    private static volatile ClassLoader loader;

    private final int format;
    private final byte[] data;
    private final int turnNum;
    private final UUID activePlayerId;
    private final Long seed;
    private final List<UUID> playerIds;
    private final Map<UUID, UUID> remap;

    /**
     * The class loader that sees every card, player and game plugin (the server's plugin loader); without one, this
     * class's own loader.
     */
    public static void setLoader(ClassLoader classLoader) {
        loader = classLoader;
    }

    private GameSnapshot(byte[] data, int turnNum, UUID activePlayerId, Long seed, List<UUID> playerIds, Map<UUID, UUID> remap) {
        this.format = FORMAT;
        this.data = data;
        this.turnNum = turnNum;
        this.activePlayerId = activePlayerId;
        this.seed = seed;
        this.playerIds = Collections.unmodifiableList(new ArrayList<>(playerIds));
        this.remap = remap == null ? Collections.emptyMap() : Collections.unmodifiableMap(new HashMap<>(remap));
    }

    /**
     * Freezes a game. Call it on the game's own thread at a quiet point (the start of a turn): it reads the live
     * objects.
     */
    public static GameSnapshot of(GameState state, Map<UUID, Card> gameCards, Map<UUID, MeldCard> meldCards, UUID startingPlayerId) {
        Payload payload = new Payload();
        payload.state = state;
        payload.gameCards = new LinkedHashMap<>(gameCards);
        payload.meldCards = new LinkedHashMap<>(meldCards);
        payload.startingPlayerId = startingPlayerId;
        ByteArrayOutputStream bytes = new ByteArrayOutputStream(256 * 1024);
        try (ObjectOutputStream out = new ObjectOutputStream(new GZIPOutputStream(bytes, 64 * 1024))) {
            out.writeObject(payload);
        } catch (IOException e) {
            throw new IllegalStateException("The game can't be saved: " + e.getMessage(), e);
        }
        return new GameSnapshot(bytes.toByteArray(), state.getTurnNum(), state.getActivePlayerId(), state.getSeed(),
                new ArrayList<>(state.getPlayers().keySet()), null);
    }

    /**
     * The same snapshot for a table whose players have other ids: every saved id in {@code savedToNew} is replaced as
     * the game is read back.
     */
    public GameSnapshot withPlayers(Map<UUID, UUID> savedToNew) {
        Map<UUID, UUID> combined = new HashMap<>();
        // a snapshot already remapped once maps from its original ids
        for (Map.Entry<UUID, UUID> e : remap.entrySet()) {
            combined.put(e.getKey(), savedToNew.getOrDefault(e.getValue(), e.getValue()));
        }
        for (Map.Entry<UUID, UUID> e : savedToNew.entrySet()) {
            if (!remap.containsValue(e.getKey())) {
                combined.put(e.getKey(), e.getValue());
            }
        }
        return new GameSnapshot(data, turnNum, activePlayerId, seed, playerIds, combined);
    }

    /** What a snapshot reads back into: fresh objects, owned by whoever restores them. */
    public static final class Restored {
        public final GameState state;
        public final Map<UUID, Card> gameCards;
        public final Map<UUID, MeldCard> meldCards;
        public final UUID startingPlayerId;

        Restored(Payload payload) {
            this.state = payload.state;
            this.gameCards = payload.gameCards;
            this.meldCards = payload.meldCards;
            this.startingPlayerId = payload.startingPlayerId;
        }
    }

    public Restored restore() {
        if (format != FORMAT) {
            throw new IllegalStateException("This saved game is from another version of the server and can't be read");
        }
        try (ObjectInputStream in = new SnapshotInput(new GZIPInputStream(new ByteArrayInputStream(data)), remap)) {
            return new Restored((Payload) in.readObject());
        } catch (IOException | ClassNotFoundException | ClassCastException e) {
            throw new IllegalStateException("The saved game can't be read: " + e.getMessage(), e);
        }
    }

    public byte[] toBytes() {
        ByteArrayOutputStream bytes = new ByteArrayOutputStream(data.length + 1024);
        try (ObjectOutputStream out = new ObjectOutputStream(bytes)) {
            out.writeObject(this);
        } catch (IOException e) {
            throw new IllegalStateException(e);
        }
        return bytes.toByteArray();
    }

    public static GameSnapshot fromBytes(byte[] bytes) {
        try (ObjectInputStream in = new SnapshotInput(new ByteArrayInputStream(bytes), Collections.emptyMap())) {
            GameSnapshot snapshot = (GameSnapshot) in.readObject();
            if (snapshot.format != FORMAT) {
                throw new IllegalStateException("This saved game is from another version of the server and can't be read");
            }
            return snapshot;
        } catch (IOException | ClassNotFoundException | ClassCastException e) {
            throw new IllegalStateException("The saved game can't be read: " + e.getMessage(), e);
        }
    }

    /** @return the turn the game carries on from */
    public int getTurnNum() {
        return turnNum;
    }

    /** @return the saved id of the player whose turn it is */
    public UUID getActivePlayerId() {
        return activePlayerId;
    }

    public Long getSeed() {
        return seed;
    }

    /** @return the saved ids of every player, in table order */
    public List<UUID> getPlayerIds() {
        return playerIds;
    }

    /** @return the size of the frozen game in bytes (compressed) */
    public int size() {
        return data.length;
    }

    private static final class Payload implements Serializable {
        private static final long serialVersionUID = 1L;
        GameState state;
        Map<UUID, Card> gameCards;
        Map<UUID, MeldCard> meldCards;
        UUID startingPlayerId;
    }

    /** Resolves classes through the plugin loader and swaps saved player ids for the new table's. */
    private static final class SnapshotInput extends ObjectInputStream {
        private final Map<UUID, UUID> remap;

        SnapshotInput(InputStream in, Map<UUID, UUID> remap) throws IOException {
            super(in);
            this.remap = remap;
            if (!remap.isEmpty()) {
                enableResolveObject(true);
            }
        }

        @Override
        protected Class<?> resolveClass(ObjectStreamClass desc) throws IOException, ClassNotFoundException {
            ClassLoader first = loader;
            if (first != null) {
                try {
                    return Class.forName(desc.getName(), false, first);
                } catch (ClassNotFoundException ignored) {
                    // fall through
                }
            }
            ClassLoader context = Thread.currentThread().getContextClassLoader();
            if (context != null) {
                try {
                    return Class.forName(desc.getName(), false, context);
                } catch (ClassNotFoundException ignored) {
                    // fall through
                }
            }
            return super.resolveClass(desc);
        }

        @Override
        protected Object resolveObject(Object obj) {
            if (obj instanceof UUID) {
                UUID mapped = remap.get(obj);
                return mapped != null ? mapped : obj;
            }
            return obj;
        }
    }
}
