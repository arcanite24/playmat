package mage.abilities.effects.common;

import mage.abilities.Ability;
import mage.abilities.effects.ContinuousEffectImpl;
import mage.cards.Card;
import mage.constants.Duration;
import mage.constants.Layer;
import mage.constants.Outcome;
import mage.constants.SubLayer;
import mage.game.Game;
import mage.game.permanent.Permanent;

import java.util.UUID;

/**
 * Alchemy's "perpetually gains [ability]": the granted ability stays with the card in every zone
 * for the rest of the game, through deaths, bounces, and shuffles.
 *
 * @author playmat
 */
public class PerpetualGainAbilityEffect extends ContinuousEffectImpl {

    private final UUID cardId;
    private final Ability ability;

    public PerpetualGainAbilityEffect(UUID cardId, Ability ability) {
        super(Duration.Custom, Layer.AbilityAddingRemovingEffects_6, SubLayer.NA, Outcome.AddAbility);
        this.cardId = cardId;
        this.ability = ability.copy();
    }

    private PerpetualGainAbilityEffect(final PerpetualGainAbilityEffect effect) {
        super(effect);
        this.cardId = effect.cardId;
        this.ability = effect.ability.copy();
    }

    @Override
    public PerpetualGainAbilityEffect copy() {
        return new PerpetualGainAbilityEffect(this);
    }

    @Override
    public boolean apply(Game game, Ability source) {
        Card card = game.getCard(cardId);
        if (card == null) {
            return false;
        }
        Ability gained = ability.copy();
        gained.setSourceId(cardId);
        Permanent permanent = game.getPermanent(cardId);
        if (permanent != null) {
            gained.setControllerId(permanent.getControllerId());
            permanent.addAbility(gained, source.getSourceId(), game);
        } else {
            gained.setControllerId(card.getOwnerId());
        }
        game.getState().addOtherAbility(card, gained, false);
        return true;
    }

    public UUID getCardId() {
        return cardId;
    }

    public Ability getAbility() {
        return ability;
    }
}
