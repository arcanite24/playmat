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
    private final List<Map<String, Object>> prompts = new ArrayList<>();
    private final List<Map<String, Object>> log = new ArrayList<>();

    /** @param every record the view at every n-th answer (0: none) */
    ShadowViews(int every) {
        this.every = every;
    }

    /** on the game thread, as the player is asked: {@code decision} is the index of the answer it will get */
    void ask(Game game, PlayerQueryEvent event, int decision) {
        if (every <= 0 || event.getPlayerId() == null) {
            return;
        }
        // every question's prompt (they're small), the view at every n-th
        com.google.gson.JsonElement prompt = mage.server.game.RustPrompts.prompt(game, event);
        if (prompt != null) {
            Map<String, Object> p = new LinkedHashMap<>();
            p.put("decision", decision);
            p.put("seat", StateFingerprint.seats(game).indexOf(event.getPlayerId()));
            p.put("prompt", prompt);
            prompts.add(p);
        }
        if (decision % every != 0) {
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

    /** a game log line, as it's sent: {@code decision} answers were given before it */
    void log(Game game, String line, int decision) {
        if (every <= 0 || line == null) {
            return;
        }
        Map<String, Object> m = new LinkedHashMap<>();
        m.put("decision", decision);
        m.put("line", mage.server.game.RustPrompts.logLine(game, line));
        log.add(m);
    }

    List<Map<String, Object>> getPrompts() {
        return prompts;
    }

    List<Map<String, Object>> getLog() {
        return log;
    }

    static JsonElement mapIds(JsonElement e, Map<UUID, String> refs) {
        return RustViews.mapIds(e, refs, refs);
    }
}
