package org.mage.test.serverside.trace;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonPrimitive;
import mage.game.Game;
import mage.game.events.PlayerQueryEvent;
import mage.game.trace.DecisionLog;
import mage.game.trace.StateFingerprint;
import mage.server.game.GameSessionPlayer;
import mage.server.websocket.rpc.JsonCodec;
import mage.view.GameView;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.regex.Pattern;

/**
 * The views of a shadow game (step C, docs/rust/Views.md): at a question, the {@link GameView} the asked player's
 * client gets, as {@link JsonCodec} writes it for the web client, with every id replaced by its decision-log
 * reference ({@link DecisionLog#refs}) so the Rust engine can render the same view and compare it field by field.
 * An id the log can't place (the game's, a combat group's) becomes {@code ?}.
 */
final class ShadowViews {

    private static final Pattern UUID_TEXT = Pattern.compile(
            "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}");

    private final int every;
    private final List<Map<String, Object>> views = new ArrayList<>();

    /** @param every record the view at every n-th answer (0: none) */
    ShadowViews(int every) {
        this.every = every;
    }

    /** on the game thread, as the player is asked: {@code decision} is the index of the answer it will get */
    void ask(Game game, PlayerQueryEvent event, int decision) {
        if (every <= 0 || decision % every != 0 || event.getPlayerId() == null) {
            return;
        }
        GameView view = GameSessionPlayer.prepareGameView(game, event.getPlayerId(), null);
        Map<UUID, String> refs = DecisionLog.refs(game);
        Map<String, Object> m = new LinkedHashMap<>();
        m.put("decision", decision);
        m.put("seat", StateFingerprint.seats(game).indexOf(event.getPlayerId()));
        m.put("query", String.valueOf(event.getQueryType()));
        // a spell's id name (`Target on stack: Name [c3b]`) is its card's: the card's short id names the spell
        Map<UUID, String> named = new java.util.HashMap<>(refs);
        int i = 0;
        for (mage.game.stack.StackObject object : game.getState().getStack()) {
            if (object instanceof mage.game.stack.Spell) {
                named.putIfAbsent(((mage.game.stack.Spell) object).getCard().getId(), "S" + i);
            }
            i++;
        }
        m.put("view", mapIdsNamed(JsonCodec.GSON.toJsonTree(view), refs, named));
        views.add(m);
    }

    List<Map<String, Object>> getViews() {
        return views;
    }

    private static final Pattern SHORT_ID = Pattern.compile("\\[([0-9a-f]{3})]");

    /**
     * Ids replaced by references, in text too, and the short ids of objects' id names ({@code Name [c3b]}, the
     * first three digits of the id) by their objects' references ({@code Name [B4]}); a short id two placed objects
     * share stays as it is
     */
    static JsonElement mapIds(JsonElement e, Map<UUID, String> refs) {
        return mapIdsNamed(e, refs, refs);
    }

    /** @param named the references of short ids: {@code refs}, and more */
    static JsonElement mapIdsNamed(JsonElement e, Map<UUID, String> refs, Map<UUID, String> named) {
        // by short id, the references of the highest rank: an object's own (a card in a library lower), then its
        // abilities' and faces'; two of the same rank leave the short id as it is
        Map<String, String> shorts = new java.util.HashMap<>();
        Map<String, Integer> ranks = new java.util.HashMap<>();
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
        return mapIds(e, refs, shorts);
    }

    private static JsonElement mapIds(JsonElement e, Map<UUID, String> refs, Map<String, String> shorts) {
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
                out.add(mapIds(x, refs, shorts));
            }
            return out;
        }
        JsonObject out = new JsonObject();
        for (Map.Entry<String, JsonElement> x : e.getAsJsonObject().entrySet()) {
            out.add(mapText(x.getKey(), refs), mapIds(x.getValue(), refs, shorts));
        }
        return out;
    }

    private static String mapShort(String s, Map<String, String> shorts) {
        if (s.indexOf('[') < 0) {
            return s;
        }
        java.util.regex.Matcher m = SHORT_ID.matcher(s);
        StringBuffer out = new StringBuffer();
        while (m.find()) {
            String ref = shorts.get(m.group(1));
            m.appendReplacement(out, java.util.regex.Matcher.quoteReplacement(
                    ref == null || ref.isEmpty() ? m.group() : "[" + ref + "]"));
        }
        m.appendTail(out);
        return out.toString();
    }

    /** a whole id becomes its reference; ids inside text (rules' object_id links) too */
    private static String mapText(String s, Map<UUID, String> refs) {
        if (s.length() < 36 || !s.contains("-")) {
            return s;
        }
        java.util.regex.Matcher m = UUID_TEXT.matcher(s);
        StringBuffer out = new StringBuffer();
        while (m.find()) {
            String ref = refs.get(UUID.fromString(m.group()));
            m.appendReplacement(out, java.util.regex.Matcher.quoteReplacement(ref == null ? "?" : ref));
        }
        m.appendTail(out);
        return out.toString();
    }
}
