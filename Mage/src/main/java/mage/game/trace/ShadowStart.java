package mage.game.trace;

import mage.abilities.TriggeredAbility;
import mage.cards.Card;
import mage.game.Game;
import mage.players.Player;

import java.util.*;

/**
 * A game as the Rust engine starts it (docs/rust/Engine.md): the state when the first player gets priority, taken at
 * the first trace entry. Shadow games record it with their answers; a game the Rust AI plays a seat of sends it to
 * start the Rust copy.
 */
public final class ShadowStart {

    private ShadowStart() {
    }

    /**
     * The start: answers given so far (mulligans), the game's random state, each seat's library and hand (with each
     * card's place in the triggered-ability registration order), and the first fingerprint. Also makes {@code log}
     * name exiled cards in the cards' order here ({@link DecisionLog#setStartOrder}).
     */
    public static Map<String, Object> capture(Game game, DecisionLog log, String fingerprintText) {
        Map<String, Object> start = new LinkedHashMap<>();
        start.put("decision", log.getDecisions().size());
        start.put("random", game.getState().getRandom().getState());
        // the order cards' triggered abilities were registered in (`TriggeredAbilities` is a LinkedHashMap):
        // abilities that trigger on one event wait in this order
        List<UUID> triggerSources = new ArrayList<>();
        for (TriggeredAbility ability : game.getState().getTriggers().values()) {
            if (!triggerSources.contains(ability.getSourceId())) {
                triggerSources.add(ability.getSourceId());
            }
        }
        // exile is named in the cards' order here
        Map<UUID, Integer> startOrder = new HashMap<>();
        for (UUID playerId : StateFingerprint.seats(game)) {
            Player player = game.getPlayer(playerId);
            for (Card card : player.getLibrary().getCards(game)) {
                startOrder.put(card.getId(), startOrder.size());
            }
            for (UUID id : player.getHand()) {
                startOrder.put(id, startOrder.size());
            }
        }
        log.setStartOrder(startOrder);
        start.put("exileOrder", "start");
        List<Map<String, Object>> seats = new ArrayList<>();
        for (UUID playerId : StateFingerprint.seats(game)) {
            Player player = game.getPlayer(playerId);
            Map<String, Object> seat = new LinkedHashMap<>();
            List<String> library = new ArrayList<>();
            List<Integer> libraryOrder = new ArrayList<>();
            for (Card card : player.getLibrary().getCards(game)) {
                library.add(card.getName());
                libraryOrder.add(triggerSources.indexOf(card.getId()));
            }
            List<String> hand = new ArrayList<>();
            List<Integer> handOrder = new ArrayList<>();
            for (UUID id : player.getHand()) {
                hand.add(game.getCard(id).getName());
                handOrder.add(triggerSources.indexOf(id));
            }
            seat.put("library", library);
            seat.put("hand", hand);
            seat.put("libraryTriggerOrder", libraryOrder);
            seat.put("handTriggerOrder", handOrder);
            seats.add(seat);
        }
        start.put("seats", seats);
        start.put("fingerprint", fingerprintText);
        return start;
    }
}
