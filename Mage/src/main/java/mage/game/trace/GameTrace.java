package mage.game.trace;

import mage.game.Game;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.function.Consumer;

/**
 * A game's state fingerprints, one each time a player is about to get priority and each time the stack resolves.
 * Two runs of the same seeded game with the same decisions give the same trace; so must a second engine (see
 * docs/rust/StateFingerprint.md). Set it on {@link mage.game.GameOptions#trace}. Simulations (AI search copies) are
 * never traced.
 */
public class GameTrace {

    public static final class Entry {
        public final int index;
        public final String point;
        public final int turn;
        public final String step;
        public final String hash;
        /** the state text, when the trace keeps it (for finding where two traces part) */
        public final String text;

        Entry(int index, String point, int turn, String step, String hash, String text) {
            this.index = index;
            this.point = point;
            this.turn = turn;
            this.step = step;
            this.hash = hash;
            this.text = text;
        }

        /** one NDJSON line, without the text */
        public String toJson() {
            return "{\"i\":" + index + ",\"point\":\"" + point + "\",\"turn\":" + turn + ",\"step\":\"" + step
                    + "\",\"fp\":\"" + hash + "\"}";
        }

        @Override
        public String toString() {
            return toJson();
        }
    }

    public static final String PRIORITY = "priority";
    public static final String RESOLVED = "resolved";
    public static final String END = "end";

    private final boolean keepText;
    private final Consumer<Entry> sink;
    private final List<Entry> entries = Collections.synchronizedList(new ArrayList<>());
    private String last;

    /**
     * @param keepText keep each state's text as well as its hash
     * @param sink     receives every entry as it is made (on the game thread); null for none
     */
    public GameTrace(boolean keepText, Consumer<Entry> sink) {
        this.keepText = keepText;
        this.sink = sink;
    }

    public GameTrace() {
        this(false, null);
    }

    /** records the game's state at this point, unless it's the same as the last one recorded */
    public void record(Game game, String point) {
        String text = StateFingerprint.text(game);
        String hash = StateFingerprint.hashText(text);
        synchronized (this) {
            if (hash.equals(last)) {
                return; // a player passing priority without anything changing
            }
            last = hash;
        }
        Entry entry = new Entry(entries.size(), point, game.getTurnNum(), game.getTurnStepType() == null ? "-" : game.getTurnStepType().name(),
                hash, keepText ? text : null);
        entries.add(entry);
        if (sink != null) {
            sink.accept(entry);
        }
    }

    public List<Entry> getEntries() {
        synchronized (entries) {
            return new ArrayList<>(entries);
        }
    }

    public List<String> hashes() {
        List<String> hashes = new ArrayList<>();
        for (Entry entry : getEntries()) {
            hashes.add(entry.hash);
        }
        return hashes;
    }

    /** index of the first entry where the two traces differ, or -1 when they are the same */
    public static int firstDifference(List<String> a, List<String> b) {
        int n = Math.min(a.size(), b.size());
        for (int i = 0; i < n; i++) {
            if (!a.get(i).equals(b.get(i))) {
                return i;
            }
        }
        return a.size() == b.size() ? -1 : n;
    }
}
