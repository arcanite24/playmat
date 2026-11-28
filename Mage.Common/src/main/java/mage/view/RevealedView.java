

package mage.view;

import java.io.Serializable;

import mage.cards.Card;
import mage.cards.Cards;
import mage.game.Game;
import mage.game.permanent.PermanentCard;

/**
 * @author BetaSteward_at_googlemail.com
 */
public class RevealedView implements Serializable {

    private final String name;
    private final CardsView cards = new CardsView();

    public RevealedView(String name, Cards cards, Game game) {
        this.name = name;
        for (Card card : cards.getCards(game)) {
            if (card instanceof PermanentCard && card.isFaceDown(game)) {
                this.cards.put(card.getId(), new CardView(card.getMainCard())); // do not use game param, so it will take default card
            } else {
                // with the zone it was revealed from, so a client can tell a card revealed from a hand; never its
                // current zone, which would follow the card into a hidden zone for as long as the reveal is listed
                CardView view = new CardView(card, game, false, false);
                view.zone = game.getState().getRevealed().getRevealedFrom(card.getId());
                this.cards.put(card.getId(), view);
            }
        }
    }

    public String getName() {
        return name;
    }

    public CardsView getCards() {
        return cards;
    }
}
