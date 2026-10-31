

package mage.game;

import java.io.Serializable;
import java.util.HashMap;
import java.util.Map;
import java.util.UUID;

import mage.cards.Card;
import mage.cards.Cards;
import mage.cards.CardsImpl;
import mage.constants.Zone;
import mage.util.Copyable;

/**
 * @author BetaSteward_at_googlemail.com
 */
public class Revealed extends HashMap<String, Cards> implements Serializable, Copyable<Revealed> {

    // the zone each card was in when it was revealed: views show that, never where the card went afterwards (a card
    // revealed from a hand and then put on the library must not be followed there)
    private final Map<UUID, Zone> revealedFrom = new HashMap<>();

    public Revealed() {
    }

    protected Revealed(final Revealed revealed) {
        for (Map.Entry<String, Cards> entry : revealed.entrySet()) {
            this.put(entry.getKey(), entry.getValue().copy());
        }
        this.revealedFrom.putAll(revealed.revealedFrom);
    }

    /**
     * Notes the zone each of these cards is in as it is revealed.
     */
    public void noteZones(Cards cards, Game game) {
        for (UUID cardId : cards) {
            Zone zone = game.getState().getZone(cardId);
            if (zone != null) {
                revealedFrom.put(cardId, zone);
            }
        }
    }

    /**
     * The zone the card was in when it was revealed, or null when it is not known.
     */
    public Zone getRevealedFrom(UUID cardId) {
        return revealedFrom.get(cardId);
    }

    public void add(String name, Card card) {
        this.get(name).add(card);
    }

    public void update(String name, Cards cards) {
        this.put(name, cards.copy());
    }

    public void add(String name, Cards cards) {
        if (this.containsKey(name)) {
            this.get(name).addAll(cards);
        } else {
            this.put(name, cards.copy());
        }
    }

    public Cards createRevealed(String name) {
        putIfAbsent(name, new CardsImpl());
        return this.get(name);
    }

    public Cards getRevealed(String name) {
        return this.get(name);
    }

    public void reset() {
        this.clear();
    }

    @Override
    public void clear() {
        super.clear();
        revealedFrom.clear();
    }

    public Card getCard(UUID cardId, Game game) {
        for (Cards cards : this.values()) {
            if (cards.contains(cardId)) {
                return game.getCard(cardId);
            }
        }
        return null;
    }

    @Override
    public Revealed copy() {
        return new Revealed(this);
    }
}
