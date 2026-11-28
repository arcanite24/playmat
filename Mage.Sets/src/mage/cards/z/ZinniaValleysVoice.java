package mage.cards.z;

import mage.MageInt;
import mage.MageObjectReference;
import mage.abilities.common.EntersBattlefieldControlledTriggeredAbility;
import mage.abilities.common.SimpleStaticAbility;
import mage.abilities.dynamicvalue.DynamicValue;
import mage.abilities.dynamicvalue.common.PermanentsOnBattlefieldCount;
import mage.abilities.dynamicvalue.common.StaticValue;
import mage.abilities.effects.common.CreateTokenCopyTargetEffect;
import mage.abilities.effects.common.continuous.BoostSourceEffect;
import mage.abilities.effects.common.continuous.GainAbilityControlledSpellsEffect;
import mage.abilities.keyword.FlyingAbility;
import mage.abilities.keyword.OffspringAbility;
import mage.cards.CardImpl;
import mage.cards.CardSetInfo;
import mage.constants.CardType;
import mage.constants.ComparisonType;
import mage.constants.Duration;
import mage.constants.SetTargetPointer;
import mage.constants.SubType;
import mage.constants.SuperType;
import mage.constants.Zone;
import mage.filter.StaticFilters;
import mage.filter.common.FilterControlledCreaturePermanent;
import mage.filter.common.FilterNonlandCard;
import mage.filter.predicate.Predicates;
import mage.filter.predicate.mageobject.AbilityPredicate;
import mage.filter.predicate.mageobject.AnotherPredicate;
import mage.filter.predicate.mageobject.BasePowerPredicate;
import mage.game.Game;
import mage.game.events.GameEvent;
import mage.game.permanent.Permanent;

import java.util.Map;
import java.util.UUID;

/**
 * @author arcanite24
 */
public final class ZinniaValleysVoice extends CardImpl {

    private static final FilterControlledCreaturePermanent filter
            = new FilterControlledCreaturePermanent("other creatures you control with base power 1");

    static {
        filter.add(AnotherPredicate.instance);
        filter.add(new BasePowerPredicate(ComparisonType.EQUAL_TO, 1));
    }

    // a creature with offspring of its own keeps it: the two would share one cost tag and copy twice
    private static final FilterNonlandCard filter2 = new FilterNonlandCard("creature spells you cast");

    static {
        filter2.add(CardType.CREATURE.getPredicate());
        filter2.add(Predicates.not(new AbilityPredicate(OffspringAbility.class)));
    }

    private static final DynamicValue xValue = new PermanentsOnBattlefieldCount(filter);

    public ZinniaValleysVoice(UUID ownerId, CardSetInfo setInfo) {
        super(ownerId, setInfo, new CardType[]{CardType.CREATURE}, "{U}{R}{W}");

        this.supertype.add(SuperType.LEGENDARY);
        this.subtype.add(SubType.BIRD);
        this.subtype.add(SubType.BARD);
        this.power = new MageInt(1);
        this.toughness = new MageInt(3);

        // Flying
        this.addAbility(FlyingAbility.getInstance());

        // Zinnia, Valley's Voice gets +X/+0, where X is the number of other creatures you control with base power 1.
        this.addAbility(new SimpleStaticAbility(new BoostSourceEffect(
                xValue, StaticValue.get(0), Duration.WhileOnBattlefield
        )));

        // Creature spells you cast have offspring {2}.
        this.addAbility(new SimpleStaticAbility(new GainAbilityControlledSpellsEffect(
                new OffspringAbility("{2}"), filter2
        )));
        // The granted offspring's trigger stays with the spell, so Zinnia makes the copy when the creature enters
        // (a creature only has offspring from Zinnia while Zinnia is on the battlefield, so the ruling holds)
        this.addAbility(new ZinniaOffspringTriggeredAbility());
    }

    private ZinniaValleysVoice(final ZinniaValleysVoice card) {
        super(card);
    }

    @Override
    public ZinniaValleysVoice copy() {
        return new ZinniaValleysVoice(this);
    }
}

class ZinniaOffspringTriggeredAbility extends EntersBattlefieldControlledTriggeredAbility {

    ZinniaOffspringTriggeredAbility() {
        super(Zone.BATTLEFIELD, new CreateTokenCopyTargetEffect(
                null, null, false, 1, false,
                false, null, 1, 1, false
        ), StaticFilters.FILTER_PERMANENT_CREATURE, false, SetTargetPointer.PERMANENT);
        this.setRuleVisible(false);
    }

    private ZinniaOffspringTriggeredAbility(final ZinniaOffspringTriggeredAbility ability) {
        super(ability);
    }

    @Override
    public ZinniaOffspringTriggeredAbility copy() {
        return new ZinniaOffspringTriggeredAbility(this);
    }

    @Override
    public boolean checkTrigger(GameEvent event, Game game) {
        if (!super.checkTrigger(event, game)) {
            return false;
        }
        Permanent permanent = game.getPermanent(event.getTargetId());
        if (permanent == null || permanent.getAbilities(game).containsClass(OffspringAbility.class)) {
            return false;
        }
        // costs are tagged on the spell, one zone change before the permanent
        Map<String, Object> tags = game.getPermanentCostsTags().get(
                new MageObjectReference(permanent.getId(), permanent.getZoneChangeCounter(game) - 1, game)
        );
        return tags != null && tags.containsKey(OffspringAbility.OFFSPRING_ACTIVATION_VALUE_KEY);
    }

    @Override
    public String getRule() {
        return "When a creature spell you cast with its offspring cost paid enters, create a 1/1 token copy of it.";
    }
}
