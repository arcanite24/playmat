package mage.game.trace;

import mage.abilities.DelayedTriggeredAbility;
import mage.abilities.TriggeredAbility;
import mage.cards.Card;
import mage.game.Game;
import mage.game.stack.StackAbility;
import mage.game.stack.StackObject;
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
            List<String> libraryPrintings = new ArrayList<>();
            for (Card card : player.getLibrary().getCards(game)) {
                library.add(card.getName());
                libraryOrder.add(triggerSources.indexOf(card.getId()));
                libraryPrintings.add(printing(card));
            }
            List<String> hand = new ArrayList<>();
            List<Integer> handOrder = new ArrayList<>();
            List<String> handPrintings = new ArrayList<>();
            for (UUID id : player.getHand()) {
                hand.add(game.getCard(id).getName());
                handOrder.add(triggerSources.indexOf(id));
                handPrintings.add(printing(game.getCard(id)));
            }
            seat.put("library", library);
            seat.put("hand", hand);
            seat.put("libraryTriggerOrder", libraryOrder);
            seat.put("handTriggerOrder", handOrder);
            // the printing each card's view shows (docs/rust/Views.md)
            seat.put("libraryPrintings", libraryPrintings);
            seat.put("handPrintings", handPrintings);
            // for Learn's hint, which counts it
            seat.put("sideboard", player.getSideboard().size());
            // the sideboard's cards, in the order answers name them (`O<seat>.<index>`): Learn's Lessons
            List<String> sideboard = new ArrayList<>();
            for (UUID id : player.getSideboard()) {
                sideboard.add(game.getCard(id).getName());
            }
            seat.put("sideboardCards", sideboard);
            seats.add(seat);
        }
        start.put("seats", seats);
        // cards revealed from an opening hand for a delayed trigger (Devourer of Destiny): [seat, hand index], the
        // trigger waiting or on the stack already
        Set<UUID> delayedSources = new HashSet<>();
        for (DelayedTriggeredAbility ability : game.getState().getDelayed()) {
            delayedSources.add(ability.getSourceId());
        }
        for (StackObject object : game.getStack()) {
            if (object instanceof StackAbility && ((StackAbility) object).getStackAbility() instanceof DelayedTriggeredAbility) {
                delayedSources.add(object.getSourceId());
            }
        }
        List<List<Integer>> revealed = new ArrayList<>();
        List<UUID> seatIds = StateFingerprint.seats(game);
        for (int s = 0; s < seatIds.size(); s++) {
            int k = 0;
            for (UUID id : game.getPlayer(seatIds.get(s)).getHand()) {
                if (delayedSources.contains(id)) {
                    revealed.add(Arrays.asList(s, k));
                }
                k++;
            }
        }
        start.put("revealed", revealed);
        start.put("fingerprint", fingerprintText);
        return start;
    }

    /** {@code SET:number}, the printing a card's view shows */
    static String printing(Card card) {
        return card.getExpansionSetCode() + ":" + card.getCardNumber();
    }
}
