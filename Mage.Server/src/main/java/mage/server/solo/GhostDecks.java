package mage.server.solo;

import com.google.gson.Gson;
import mage.cards.decks.DeckCardInfo;
import mage.cards.decks.DeckCardLists;
import org.apache.log4j.Logger;

import java.security.SecureRandom;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Ghost decks (S047): a player shares a deck under a short code, anyone can play against it with the computer at the
 * wheel, and every game's result lands in the owner's inbox, so a deck keeps a record while its owner is away.
 */
public final class GhostDecks {

    private static final Logger logger = Logger.getLogger(GhostDecks.class);
    /** codes skip the letters and digits that read alike */
    private static final String ALPHABET = "ABCDEFGHJKMNPQRSTUVWXYZ23456789";
    public static final int CODE_LENGTH = 6;
    public static final int MAX_SHARED = 20;
    public static final int MAX_CARDS = 250;
    private static final SecureRandom RANDOM = new SecureRandom();
    private static final GhostDecks INSTANCE = new GhostDecks();

    /** tables playing against a ghost: the code and the player */
    private final Map<UUID, String[]> tables = new ConcurrentHashMap<>();

    /** A shared deck, as its owner sees it. */
    public static class GhostDeck {
        public String code;
        public String name;
        public long created;
        public int played;
        /** games the deck won (its challengers lost) */
        public int won;
    }

    /** A ghost to play against: the deck the computer pilots, and whose it is. */
    public static class GhostOpponent {
        public String code;
        public String name;
        public String owner;
        public int played;
        public int won;
        public DeckCardLists deck;
    }

    /** A note in a player's inbox. */
    public static class InboxNote {
        public long id;
        public long created;
        public String kind;
        public String text;
        public String ref;
        public boolean seen;
    }

    private GhostDecks() {
    }

    public static GhostDecks get() {
        return INSTANCE;
    }

    public static String normalize(String code) {
        return code == null ? "" : code.trim().toUpperCase(Locale.ENGLISH).replace("-", "");
    }

    public GhostDeck share(String owner, String name, DeckCardLists deck, long now) throws SQLException {
        String problem = problem(deck);
        if (problem != null) {
            throw new IllegalArgumentException(problem);
        }
        SoloStore store = SoloStore.get();
        if (store.sharedDecks(owner).size() >= MAX_SHARED) {
            throw new IllegalArgumentException("You share " + MAX_SHARED + " decks already; stop sharing one first.");
        }
        SoloStore.SharedDeckRow row = new SoloStore.SharedDeckRow();
        row.owner = owner;
        String trimmed = name == null ? "" : name.trim();
        if (trimmed.isEmpty()) {
            trimmed = deck.getName() == null || deck.getName().trim().isEmpty() ? "A deck" : deck.getName().trim();
        }
        row.name = trimmed.length() > 60 ? trimmed.substring(0, 60) : trimmed;
        row.created = now;
        row.deck = new Gson().toJson(deck);
        for (int attempt = 0; ; attempt++) {
            row.code = code();
            if (store.sharedDeck(row.code) == null) {
                break;
            }
            if (attempt > 20) {
                throw new IllegalStateException("No free code was found; try again.");
            }
        }
        store.putSharedDeck(row);
        return view(row);
    }

    static String problem(DeckCardLists deck) {
        if (deck == null || deck.getCards().isEmpty()) {
            return "The deck is empty.";
        }
        int cards = 0;
        for (DeckCardInfo card : deck.getCards()) {
            cards += card.getAmount();
        }
        for (DeckCardInfo card : deck.getSideboard()) {
            cards += card.getAmount();
        }
        return cards > MAX_CARDS ? "A shared deck holds at most " + MAX_CARDS + " cards." : null;
    }

    private static String code() {
        StringBuilder code = new StringBuilder();
        for (int i = 0; i < CODE_LENGTH; i++) {
            code.append(ALPHABET.charAt(RANDOM.nextInt(ALPHABET.length())));
        }
        return code.toString();
    }

    public List<GhostDeck> mine(String owner) throws SQLException {
        List<GhostDeck> decks = new ArrayList<>();
        for (SoloStore.SharedDeckRow row : SoloStore.get().sharedDecks(owner)) {
            decks.add(view(row));
        }
        return decks;
    }

    public boolean unshare(String owner, String code) throws SQLException {
        return SoloStore.get().deleteSharedDeck(normalize(code), owner);
    }

    public GhostOpponent opponent(String code) throws SQLException {
        SoloStore.SharedDeckRow row = SoloStore.get().sharedDeck(normalize(code));
        if (row == null) {
            throw new IllegalArgumentException("No deck is shared under " + code + ".");
        }
        GhostOpponent ghost = new GhostOpponent();
        ghost.code = row.code;
        ghost.name = row.name;
        ghost.owner = row.owner;
        ghost.played = row.played;
        ghost.won = row.won;
        ghost.deck = new Gson().fromJson(row.deck, DeckCardLists.class);
        return ghost;
    }

    /** a table plays against a ghost; its result goes to the owner when it ends */
    public void mark(UUID tableId, String player, String code) throws SQLException {
        String normalized = normalize(code);
        if (SoloStore.get().sharedDeck(normalized) == null) {
            throw new IllegalArgumentException("No deck is shared under " + code + ".");
        }
        tables.put(tableId, new String[]{normalized, player});
    }

    public boolean marked(UUID tableId) {
        return tableId != null && tables.containsKey(tableId);
    }

    /** a ghost table's match ended: the deck's record moves and its owner hears how it went */
    public void ended(UUID tableId, String player, boolean playerWon, int turns) {
        String[] marked = tableId == null ? null : tables.remove(tableId);
        if (marked == null || !marked[1].equalsIgnoreCase(player)) {
            return;
        }
        try {
            SoloStore store = SoloStore.get();
            SoloStore.SharedDeckRow row = store.sharedDeck(marked[0]);
            if (row == null) {
                return;
            }
            store.sharedDeckPlayed(row.code, !playerWon);
            if (!row.owner.equalsIgnoreCase(player)) {
                String text = playerWon
                        ? player + " beat your " + row.name + (turns > 0 ? " on turn " + turns : "") + "."
                        : "Your " + row.name + " beat " + player + (turns > 0 ? " on turn " + turns : "") + ".";
                store.addInbox(row.owner, "ghost", text, row.code, System.currentTimeMillis());
            }
        } catch (SQLException | RuntimeException e) {
            logger.warn("Ghost decks: a result wasn't recorded", e);
        }
    }

    public List<InboxNote> inbox(String user, int limit) throws SQLException {
        List<InboxNote> notes = new ArrayList<>();
        for (SoloStore.InboxRow row : SoloStore.get().inbox(user, limit)) {
            InboxNote note = new InboxNote();
            note.id = row.id;
            note.created = row.created;
            note.kind = row.kind;
            note.text = row.text;
            note.ref = row.ref;
            note.seen = row.seen;
            notes.add(note);
        }
        return notes;
    }

    private static GhostDeck view(SoloStore.SharedDeckRow row) {
        GhostDeck deck = new GhostDeck();
        deck.code = row.code;
        deck.name = row.name;
        deck.created = row.created;
        deck.played = row.played;
        deck.won = row.won;
        return deck;
    }
}
