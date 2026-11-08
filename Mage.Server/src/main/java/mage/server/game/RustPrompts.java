package mage.server.game;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import mage.game.Game;
import mage.game.events.PlayerQueryEvent;
import mage.game.permanent.Permanent;
import mage.game.trace.DecisionLog;
import mage.server.websocket.rpc.JsonCodec;
import mage.view.AbilityPickerView;
import mage.view.CardsView;
import mage.view.GameClientMessage;
import mage.view.PermanentView;

import java.util.Map;
import java.util.UUID;

/**
 * A question as the Rust engine renders it to compare (docs/rust/Views.md, prompts): what the client gets with it
 * ({@code GameController} and {@code GameSessionPlayer}'s callbacks), without the game view, which is compared on its
 * own, ids mapped as {@link RustViews} maps them. {@code method} is the callback's name.
 */
public final class RustPrompts {

    private RustPrompts() {
    }

    /** the prompt the asked player's client gets now, or null for a question that isn't one */
    public static JsonElement prompt(Game game, PlayerQueryEvent e) {
        UUID player = e.getPlayerId();
        String method;
        Object data;
        switch (e.getQueryType()) {
            case ASK:
                method = "GAME_ASK";
                data = new GameClientMessage(null, e.getOptions(), e.getMessage());
                break;
            case PICK_TARGET: {
                method = "GAME_TARGET";
                CardsView cards;
                if (e.getCards() != null) {
                    cards = new CardsView(game, e.getCards().getCards(game), player, true);
                } else if (e.getPerms() != null) {
                    cards = new CardsView();
                    for (Permanent perm : e.getPerms()) {
                        cards.put(perm.getId(), new PermanentView(perm, game.getCard(perm.getId()), player, game));
                    }
                } else {
                    cards = new CardsView();
                }
                data = new GameClientMessage(null, e.getOptions(), e.getMessage(), cards, e.getTargets(), e.isRequired());
                break;
            }
            case PICK_ABILITY:
                method = "GAME_TARGET";
                data = new GameClientMessage(null, e.getOptions(), e.getMessage(), new CardsView(e.getAbilities(), game),
                        null, e.isRequired());
                break;
            case SELECT:
                method = "GAME_SELECT";
                data = new GameClientMessage(null, e.getOptions(), e.getMessage());
                break;
            case PLAY_MANA:
                method = "GAME_PLAY_MANA";
                data = new GameClientMessage(null, e.getOptions(), e.getMessage());
                break;
            case PLAY_X_MANA:
                method = "GAME_PLAY_XMANA";
                data = new GameClientMessage(null, null, e.getMessage());
                break;
            case CHOOSE_ABILITY: {
                method = "GAME_CHOOSE_ABILITY";
                String objectName = e.getChoices() != null && !e.getChoices().isEmpty() ? e.getChoices().iterator().next() : null;
                data = new AbilityPickerView(null, objectName, e.getAbilities(), e.getMessage());
                break;
            }
            case CHOOSE_PILE:
                method = "GAME_CHOOSE_PILE";
                data = new GameClientMessage(null, null, e.getMessage(), new CardsView(game, e.getPile1(), player),
                        new CardsView(game, e.getPile2(), player));
                break;
            case CHOOSE_MODE:
                method = "GAME_CHOOSE_ABILITY";
                data = new AbilityPickerView(null, e.getModes(), e.getMessage());
                break;
            case CHOOSE_CHOICE:
                method = "GAME_CHOOSE_CHOICE";
                data = new GameClientMessage(null, null, e.getChoice());
                break;
            case AMOUNT:
                method = "GAME_GET_AMOUNT";
                data = new GameClientMessage(null, null, e.getMessage(), e.getMin(), e.getMax());
                break;
            case MULTI_AMOUNT:
                method = "GAME_GET_MULTI_AMOUNT";
                data = new GameClientMessage(null, e.getOptions(), e.getMessages(), e.getMin(), e.getMax());
                break;
            default:
                return null;
        }
        JsonObject out = new JsonObject();
        out.addProperty("method", method);
        JsonElement tree = JsonCodec.GSON.toJsonTree(data);
        if (tree.isJsonObject()) {
            tree.getAsJsonObject().remove("gameView");
        }
        out.add("data", tree);
        Map<UUID, String> refs = DecisionLog.refs(game);
        return RustViews.mapIds(out, refs, RustViews.named(game, refs));
    }

    /**
     * A game log line, its ids and short ids mapped as a view's are, and its players named by seat ({@code A},
     * {@code B}) as the engine names them
     */
    public static String logLine(Game game, String line) {
        java.util.List<UUID> seats = mage.game.trace.StateFingerprint.seats(game);
        for (int i = 0; i < seats.size(); i++) {
            mage.players.Player p = game.getPlayer(seats.get(i));
            if (p != null) {
                line = line.replace(mage.util.GameLog.getColoredPlayerName(p.getName()),
                        mage.util.GameLog.getColoredPlayerName(String.valueOf((char) ('A' + i))));
            }
        }
        Map<UUID, String> refs = DecisionLog.refs(game);
        return RustViews.mapIds(new com.google.gson.JsonPrimitive(line), refs, RustViews.named(game, refs)).getAsString();
    }
}
