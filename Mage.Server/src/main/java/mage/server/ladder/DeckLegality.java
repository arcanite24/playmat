package mage.server.ladder;

import mage.cards.decks.Deck;
import mage.cards.decks.DeckCardLists;
import mage.cards.decks.DeckValidator;
import mage.cards.decks.DeckValidatorError;
import mage.cards.decks.DeckValidatorFactory;

import java.util.ArrayList;
import java.util.List;

/** Whether a deck passes a format's rules, as the ladder and the League check it */
public final class DeckLegality {

    private DeckLegality() {
    }

    /** the validator of a deck type, or null when this server has none */
    public static DeckValidator validator(String deckType) {
        return deckType == null ? null : DeckValidatorFactory.instance.createDeckValidator(deckType);
    }

    /**
     * Why the deck isn't legal in the deck type (the first few reasons), or nothing. Throws IllegalStateException when
     * the server can't check the deck type, IllegalArgumentException when the deck can't be read.
     */
    public static List<String> problems(String deckType, DeckCardLists lists) {
        List<String> out = new ArrayList<>();
        DeckValidator validator = validator(deckType);
        if (validator == null) {
            throw new IllegalStateException("This server can't check " + deckType + " decks.");
        }
        Deck deck;
        try {
            deck = Deck.load(lists, false, false);
        } catch (Exception e) {
            throw new IllegalArgumentException("The deck can't be read: " + e.getMessage(), e);
        }
        if (!validator.validate(deck)) {
            for (DeckValidatorError error : validator.getErrorsListSorted(3)) {
                out.add(error.getMessage() == null || error.getMessage().isEmpty() ? error.getGroup() : error.getGroup() + ": " + error.getMessage());
            }
            if (out.isEmpty()) {
                out.add("it doesn't pass the format's rules");
            }
        }
        return out;
    }
}
