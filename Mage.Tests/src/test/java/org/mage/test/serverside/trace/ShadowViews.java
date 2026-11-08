package org.mage.test.serverside.trace;

import com.google.gson.JsonElement;
import mage.game.Game;
import mage.game.events.PlayerQueryEvent;
import mage.game.trace.StateFingerprint;
import mage.server.game.RustViews;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;

/**
 * The views of a shadow game (step C, docs/rust/Views.md): at a question, the {@link mage.view.GameView} the asked
 * player's client gets, ids mapped to decision-log references ({@link RustViews}), so the Rust engine can render the
 * same view and compare it field by field.
 */
final class ShadowViews {

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
        Map<String, Object> m = new LinkedHashMap<>();
        m.put("decision", decision);
        m.put("seat", StateFingerprint.seats(game).indexOf(event.getPlayerId()));
        m.put("query", String.valueOf(event.getQueryType()));
        m.put("view", RustViews.view(game, event.getPlayerId()));
        views.add(m);
    }

    List<Map<String, Object>> getViews() {
        return views;
    }

    static JsonElement mapIds(JsonElement e, Map<UUID, String> refs) {
        return RustViews.mapIds(e, refs, refs);
    }
}
