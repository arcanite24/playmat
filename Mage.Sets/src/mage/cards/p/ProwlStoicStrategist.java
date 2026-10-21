package mage.cards.p;

import mage.abilities.Ability;
import mage.abilities.TriggeredAbilityImpl;
import mage.abilities.common.AttacksTriggeredAbility;
import mage.abilities.common.EntersBattlefieldControlledTriggeredAbility;
import mage.abilities.effects.OneShotEffect;
import mage.abilities.effects.common.DrawCardSourceControllerEffect;
import mage.abilities.effects.common.IfAbilityHasResolvedXTimesEffect;
import mage.abilities.effects.common.TransformSourceEffect;
import mage.abilities.effects.common.counter.AddCountersSourceEffect;
import mage.abilities.keyword.LivingMetalAbility;
import mage.abilities.keyword.MoreThanMeetsTheEyeAbility;
import mage.cards.Card;
import mage.cards.CardSetInfo;
import mage.cards.TransformingDoubleFacedCard;
import mage.constants.CardType;
import mage.constants.Duration;
import mage.constants.Outcome;
import mage.constants.SubType;
import mage.constants.SuperType;
import mage.constants.Zone;
import mage.counters.CounterType;
import mage.filter.FilterPermanent;
import mage.filter.common.FilterControlledPermanent;
import mage.filter.predicate.Predicates;
import mage.filter.predicate.mageobject.AnotherPredicate;
import mage.filter.predicate.permanent.TappedPredicate;
import mage.game.Game;
import mage.game.events.GameEvent;
import mage.game.permanent.Permanent;
import mage.players.Player;
import mage.target.TargetPermanent;
import mage.util.CardUtil;
import mage.watchers.common.AbilityResolvedWatcher;

import java.util.UUID;

/**
 * @author arcanite24
 */
public final class ProwlStoicStrategist extends TransformingDoubleFacedCard {

    private static final FilterPermanent filterTapped = new FilterPermanent("other target tapped creature or Vehicle");
    private static final FilterPermanent filterAnother = new FilterControlledPermanent("another creature or Vehicle");

    static {
        filterTapped.add(Predicates.or(
                CardType.CREATURE.getPredicate(),
                SubType.VEHICLE.getPredicate()
        ));
        filterTapped.add(TappedPredicate.TAPPED);
        filterTapped.add(AnotherPredicate.instance);
        filterAnother.add(Predicates.or(
                CardType.CREATURE.getPredicate(),
                SubType.VEHICLE.getPredicate()
        ));
        filterAnother.add(AnotherPredicate.instance);
    }

    public ProwlStoicStrategist(UUID ownerId, CardSetInfo setInfo) {
        super(ownerId, setInfo,
                new SuperType[]{SuperType.LEGENDARY}, new CardType[]{CardType.ARTIFACT, CardType.CREATURE}, new SubType[]{SubType.ROBOT}, "{3}{W}",
                "Prowl, Pursuit Vehicle",
                new SuperType[]{SuperType.LEGENDARY}, new CardType[]{CardType.ARTIFACT}, new SubType[]{SubType.VEHICLE}, "W"
        );

        // Prowl, Stoic Strategist
        this.getLeftHalfCard().setPT(3, 3);

        // More Than Meets the Eye {2}{W}
        this.getLeftHalfCard().addAbility(new MoreThanMeetsTheEyeAbility(this, "{2}{W}"));

        // Whenever Prowl attacks, exile up to one other target tapped creature or Vehicle. For as long as that card remains exiled, its owner may play it.
        Ability ability = new AttacksTriggeredAbility(new ProwlStoicStrategistEffect());
        ability.addTarget(new TargetPermanent(0, 1, filterTapped));
        this.getLeftHalfCard().addAbility(ability);

        // Whenever a player plays a card exiled with Prowl, you may draw a card and convert Prowl.
        this.getLeftHalfCard().addAbility(new ProwlStoicStrategistTriggeredAbility());

        // Prowl, Pursuit Vehicle
        this.getRightHalfCard().setPT(2, 3);

        // Living metal
        this.getRightHalfCard().addAbility(new LivingMetalAbility());

        // Whenever another creature or Vehicle enters the battlefield under your control, put a +1/+1 counter on Prowl. If this is the second time this ability has resolved this turn, convert Prowl.
        ability = new EntersBattlefieldControlledTriggeredAbility(
                new AddCountersSourceEffect(CounterType.P1P1.createInstance()), filterAnother
        );
        ability.addEffect(new IfAbilityHasResolvedXTimesEffect(
                2, new TransformSourceEffect().setText("convert {this}")
        ).concatBy("."));
        ability.addWatcher(new AbilityResolvedWatcher());
        this.getRightHalfCard().addAbility(ability);
    }

    private ProwlStoicStrategist(final ProwlStoicStrategist card) {
        super(card);
    }

    @Override
    public ProwlStoicStrategist copy() {
        return new ProwlStoicStrategist(this);
    }
}

class ProwlStoicStrategistEffect extends OneShotEffect {

    ProwlStoicStrategistEffect() {
        super(Outcome.Exile);
        staticText = "exile up to one other target tapped creature or Vehicle. " +
                "For as long as that card remains exiled, its owner may play it";
    }

    private ProwlStoicStrategistEffect(final ProwlStoicStrategistEffect effect) {
        super(effect);
    }

    @Override
    public ProwlStoicStrategistEffect copy() {
        return new ProwlStoicStrategistEffect(this);
    }

    @Override
    public boolean apply(Game game, Ability source) {
        Player controller = game.getPlayer(source.getControllerId());
        Permanent permanent = game.getPermanent(getTargetPointer().getFirst(game, source));
        if (controller == null || permanent == null) {
            return false;
        }
        UUID ownerId = permanent.getOwnerId();
        controller.moveCardsToExile(
                permanent, source, game, true,
                CardUtil.getExileZoneId(game, source),
                CardUtil.getSourceName(game, source)
        );
        Card card = game.getCard(permanent.getId());
        if (card == null || game.getState().getZone(card.getId()) != Zone.EXILED) {
            return true;
        }
        CardUtil.makeCardPlayable(game, source, card, false, Duration.Custom, false, ownerId, null);
        return true;
    }
}

class ProwlStoicStrategistTriggeredAbility extends TriggeredAbilityImpl {

    ProwlStoicStrategistTriggeredAbility() {
        super(Zone.BATTLEFIELD, new DrawCardSourceControllerEffect(1), true);
        this.addEffect(new TransformSourceEffect().setText("convert {this}").concatBy("and"));
        setTriggerPhrase("Whenever a player plays a card exiled with {this}, ");
    }

    private ProwlStoicStrategistTriggeredAbility(final ProwlStoicStrategistTriggeredAbility ability) {
        super(ability);
    }

    @Override
    public ProwlStoicStrategistTriggeredAbility copy() {
        return new ProwlStoicStrategistTriggeredAbility(this);
    }

    @Override
    public boolean checkEventType(GameEvent event, Game game) {
        return event.getType() == GameEvent.EventType.SPELL_CAST
                || event.getType() == GameEvent.EventType.LAND_PLAYED;
    }

    @Override
    public boolean checkTrigger(GameEvent event, Game game) {
        return event.getApprovingObject() != null
                && event
                .getApprovingObject()
                .getApprovingMageObjectReference()
                .refersTo(this.getSourceObject(game), game);
    }
}
