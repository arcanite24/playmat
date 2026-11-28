package mage.cards.d;

import mage.MageInt;
import mage.abilities.Ability;
import mage.abilities.common.ActivateIfConditionActivatedAbility;
import mage.abilities.common.EntersBattlefieldTappedAbility;
import mage.abilities.condition.Condition;
import mage.abilities.costs.mana.ManaCostsImpl;
import mage.abilities.effects.common.PerpetuallyBoostSourceEffect;
import mage.abilities.effects.common.ReturnSourceFromGraveyardToBattlefieldEffect;
import mage.cards.Card;
import mage.cards.CardImpl;
import mage.cards.CardSetInfo;
import mage.constants.CardType;
import mage.constants.SubType;
import mage.constants.Zone;
import mage.game.Game;

import java.util.UUID;

/**
 * @author playmat
 */
public final class DiminishedReturner extends CardImpl {

    public DiminishedReturner(UUID ownerId, CardSetInfo setInfo) {
        super(ownerId, setInfo, new CardType[]{CardType.CREATURE}, "{1}{B}{B}");

        this.subtype.add(SubType.PHYREXIAN);
        this.subtype.add(SubType.SKELETON);
        this.power = new MageInt(4);
        this.toughness = new MageInt(3);

        // Diminished Returner enters tapped.
        this.addAbility(new EntersBattlefieldTappedAbility());

        // {B}{B}: Diminished Returner perpetually gets -1/-1, then return it to the battlefield. Activate only if Diminished Returner is in your graveyard and its toughness is 2 or greater.
        Ability ability = new ActivateIfConditionActivatedAbility(
                Zone.GRAVEYARD, new PerpetuallyBoostSourceEffect(-1, -1),
                new ManaCostsImpl<>("{B}{B}"), DiminishedReturnerCondition.instance
        );
        ability.addEffect(new ReturnSourceFromGraveyardToBattlefieldEffect().setText("return it to the battlefield").concatBy(", then"));
        this.addAbility(ability);
    }

    private DiminishedReturner(final DiminishedReturner card) {
        super(card);
    }

    @Override
    public DiminishedReturner copy() {
        return new DiminishedReturner(this);
    }
}

enum DiminishedReturnerCondition implements Condition {
    instance;

    @Override
    public boolean apply(Game game, Ability source) {
        Card card = game.getCard(source.getSourceId());
        return card != null
                && game.getState().getZone(card.getId()) == Zone.GRAVEYARD
                && card.getToughness().getValue() >= 2;
    }

    @Override
    public String toString() {
        return "if {this} is in your graveyard and its toughness is 2 or greater";
    }
}
