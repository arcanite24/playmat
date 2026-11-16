package mage.abilities.effects.common;

import mage.abilities.Ability;
import mage.abilities.effects.ContinuousEffectImpl;
import mage.abilities.effects.OneShotEffect;
import mage.cards.Card;
import mage.constants.Duration;
import mage.constants.Layer;
import mage.constants.Outcome;
import mage.constants.SubLayer;
import mage.game.Game;
import mage.game.permanent.Permanent;
import mage.util.CardUtil;

import java.util.UUID;

/**
 * Alchemy's "perpetually gets +X/+Y": the change stays with the card in every zone for the rest of the game,
 * through deaths, bounces and shuffles.
 * <p>
 * The running total lives in the game state (so rollbacks and copied games keep it right), and one layer 7c effect
 * per card adds it: to the permanent while the card is on the battlefield, and to the card itself everywhere else.
 * Cards off the battlefield are never reset to their base P/T, so the card's value is set from its base each time
 * rather than added to.
 *
 * @author playmat
 */
public class PerpetuallyBoostSourceEffect extends OneShotEffect {

    private static final String KEY = "perpetualBoost|";

    private final int power;
    private final int toughness;

    public PerpetuallyBoostSourceEffect(int power, int toughness) {
        super(power < 0 || toughness < 0 ? Outcome.UnboostCreature : Outcome.BoostCreature);
        this.power = power;
        this.toughness = toughness;
        this.staticText = "{this} perpetually gets " + CardUtil.getBoostCountAsStr(power, toughness);
    }

    private PerpetuallyBoostSourceEffect(final PerpetuallyBoostSourceEffect effect) {
        super(effect);
        this.power = effect.power;
        this.toughness = effect.toughness;
    }

    @Override
    public PerpetuallyBoostSourceEffect copy() {
        return new PerpetuallyBoostSourceEffect(this);
    }

    @Override
    public boolean apply(Game game, Ability source) {
        Card card = game.getCard(source.getSourceId());
        if (card == null) {
            return false;
        }
        perpetuallyBoost(card.getMainCard().getId(), power, toughness, game, source);
        return true;
    }

    /** "the card perpetually gets +power/+toughness"; usable by any perpetual P/T change, not only the source's */
    public static void perpetuallyBoost(UUID cardId, int power, int toughness, Game game, Ability source) {
        int[] total = getTotal(cardId, game);
        if (total == null) {
            game.addEffect(new PerpetualBoostEffect(cardId), source);
            total = new int[]{0, 0};
        }
        game.getState().setValue(KEY + "power|" + cardId, total[0] + power);
        game.getState().setValue(KEY + "toughness|" + cardId, total[1] + toughness);
    }

    /** the perpetual P/T change a card carries, as {power, toughness}; null when it has none */
    public static int[] getTotal(UUID cardId, Game game) {
        Object power = game.getState().getValue(KEY + "power|" + cardId);
        Object toughness = game.getState().getValue(KEY + "toughness|" + cardId);
        if (!(power instanceof Integer) || !(toughness instanceof Integer)) {
            return null;
        }
        return new int[]{(Integer) power, (Integer) toughness};
    }

    static class PerpetualBoostEffect extends ContinuousEffectImpl {

        private final UUID cardId;

        PerpetualBoostEffect(UUID cardId) {
            super(Duration.Custom, Layer.PTChangingEffects_7, SubLayer.ModifyPT_7c, Outcome.Benefit);
            this.cardId = cardId;
        }

        private PerpetualBoostEffect(final PerpetualBoostEffect effect) {
            super(effect);
            this.cardId = effect.cardId;
        }

        @Override
        public PerpetualBoostEffect copy() {
            return new PerpetualBoostEffect(this);
        }

        @Override
        public boolean apply(Game game, Ability source) {
            int[] total = getTotal(cardId, game);
            if (total == null) {
                return false;
            }
            Card card = game.getCard(cardId);
            if (card != null) {
                card.getPower().setBoostedValue(CardUtil.overflowInc(card.getPower().getModifiedBaseValue(), total[0]));
                card.getToughness().setBoostedValue(CardUtil.overflowInc(card.getToughness().getModifiedBaseValue(), total[1]));
            }
            Permanent permanent = game.getPermanent(cardId);
            if (permanent != null) {
                permanent.addPower(total[0]);
                permanent.addToughness(total[1]);
            }
            return true;
        }
    }
}
