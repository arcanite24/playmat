package mage.server.game;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonPrimitive;
import mage.game.Game;
import mage.game.trace.DecisionLog;
import mage.server.websocket.rpc.JsonCodec;
import mage.view.GameView;

import java.util.HashMap;
import java.util.Map;
import java.util.UUID;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * A player's {@link GameView} as the Rust engine renders it to compare (docs/rust/Views.md): the JSON {@link JsonCodec}
 * writes for the web client, with every id replaced by its decision-log reference ({@link DecisionLog#refs}) and the
 * short ids in text ({@code Name [c3b]}) by the references of the objects they name. An id the log can't place
 * becomes {@code ?}. Shadow games record these, and the Rust AI's copy of a live game gets them to compare.
 */
public final class RustViews {

    private static final Pattern UUID_TEXT = Pattern.compile(
            "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}");
    private static final Pattern SHORT_ID = Pattern.compile("\\[([0-9a-f]{3})]");

    private RustViews() {
    }

    /** the view the player's client gets now, ids mapped (building it doesn't change the game) */
    public static JsonElement view(Game game, UUID playerId) {
        GameView view = GameSessionPlayer.prepareGameView(game, playerId, null);
        Map<UUID, String> refs = DecisionLog.refs(game);
        return mapIds(JsonCodec.GSON.toJsonTree(view), refs, named(game, refs));
    }

    /** the references of short ids: {@code refs}, and a spell's card id as its stack object's */
    public static Map<UUID, String> named(Game game, Map<UUID, String> refs) {
        // a spell's id name (`Target on stack: Name [c3b]`) is its card's: the card's short id names the spell
        Map<UUID, String> named = new HashMap<>(refs);
        int i = 0;
        for (mage.game.stack.StackObject object : game.getState().getStack()) {
            if (object instanceof mage.game.stack.Spell) {
                named.putIfAbsent(((mage.game.stack.Spell) object).getCard().getId(), "S" + i);
            }
            i++;
        }
        return named;
    }

    /**
     * Ids replaced by references, in text too, and the short ids of objects' id names by their objects' references; a
     * short id two placed objects of the same rank share stays as it is
     *
     * @param named the references of short ids: {@code refs}, and more
     */
    public static JsonElement mapIds(JsonElement e, Map<UUID, String> refs, Map<UUID, String> named) {
        // by short id, the references of the highest rank: an object's own (a card in a library lower), then its
        // abilities' and faces'
        Map<String, String> shorts = new HashMap<>();
        Map<String, Integer> ranks = new HashMap<>();
        for (Map.Entry<UUID, String> r : named.entrySet()) {
            String k = r.getKey().toString().substring(0, 3);
            String ref = r.getValue();
            int rank = ref.matches("[PHGBSEC][0-9]+(\\.[0-9]+)?") ? 2 : ref.matches("L[0-9]+\\.[0-9]+") ? 1 : 0;
            int was = ranks.getOrDefault(k, -1);
            if (rank > was) {
                ranks.put(k, rank);
                shorts.put(k, ref);
            } else if (rank == was && !ref.equals(shorts.get(k))) {
                shorts.put(k, "");
            }
        }
        return mapTree(e, refs, shorts);
    }

    private static JsonElement mapTree(JsonElement e, Map<UUID, String> refs, Map<String, String> shorts) {
        if (e == null || e.isJsonNull()) {
            return e;
        }
        if (e.isJsonPrimitive()) {
            JsonPrimitive p = e.getAsJsonPrimitive();
            return p.isString() ? new JsonPrimitive(mapShort(mapText(p.getAsString(), refs), shorts)) : p;
        }
        if (e.isJsonArray()) {
            JsonArray out = new JsonArray();
            for (JsonElement x : e.getAsJsonArray()) {
                out.add(mapTree(x, refs, shorts));
            }
            return out;
        }
        JsonObject out = new JsonObject();
        for (Map.Entry<String, JsonElement> x : e.getAsJsonObject().entrySet()) {
            out.add(mapText(x.getKey(), refs), mapTree(x.getValue(), refs, shorts));
        }
        return out;
    }

    private static String mapShort(String s, Map<String, String> shorts) {
        if (s.indexOf('[') < 0) {
            return s;
        }
        Matcher m = SHORT_ID.matcher(s);
        StringBuffer out = new StringBuffer();
        while (m.find()) {
            String ref = shorts.get(m.group(1));
            m.appendReplacement(out, Matcher.quoteReplacement(ref == null || ref.isEmpty() ? m.group() : "[" + ref + "]"));
        }
        m.appendTail(out);
        return out.toString();
    }

    /** a whole id becomes its reference; ids inside text (rules' object_id links) too */
    private static String mapText(String s, Map<UUID, String> refs) {
        if (s.length() < 36 || !s.contains("-")) {
            return s;
        }
        Matcher m = UUID_TEXT.matcher(s);
        StringBuffer out = new StringBuffer();
        while (m.find()) {
            String ref = refs.get(UUID.fromString(m.group()));
            m.appendReplacement(out, Matcher.quoteReplacement(ref == null ? "?" : ref));
        }
        m.appendTail(out);
        return out.toString();
    }
}
