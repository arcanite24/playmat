package mage.cards.g;

import mage.MageInt;
import mage.MageObject;
import mage.abilities.TriggeredAbilityImpl;
import mage.abilities.common.ZoneChangeTriggeredAbility;
import mage.abilities.costs.mana.ManaCostsImpl;
import mage.abilities.effects.common.ConjureCardEffect;
import mage.abilities.keyword.UnearthAbility;
import mage.cards.CardImpl;
import mage.cards.CardSetInfo;
import mage.constants.CardType;
import mage.constants.SubType;
import mage.constants.Zone;
import mage.game.Game;
import mage.game.events.GameEvent;
import mage.game.events.ZoneChangeEvent;

import java.util.UUID;

/**
 * @author playmat
 */
public final class GixianRecycler extends CardImpl {

    public GixianRecycler(UUID ownerId, CardSetInfo setInfo) {
        super(ownerId, setInfo, new CardType[]{CardType.ARTIFACT, CardType.CREATURE}, "{2}");

        this.subtype.add(SubType.PHYREXIAN);
        this.subtype.add(SubType.HORROR);
        this.power = new MageInt(3);
        this.toughness = new MageInt(1);

        // When Gixian Recycler dies or is put into your graveyard from your hand or library, conjure a card named Gixian Recycler into your graveyard.
        this.addAbility(new GixianRecyclerTriggeredAbility());

        // Unearth {1}{B}
        this.addAbility(new UnearthAbility(new ManaCostsImpl<>("{1}{B}")));
    }

    private GixianRecycler(final GixianRecycler card) {
        super(card);
    }

    @Override
    public GixianRecycler copy() {
        return new GixianRecycler(this);
    }
}

class GixianRecyclerTriggeredAbility extends ZoneChangeTriggeredAbility {

    GixianRecyclerTriggeredAbility() {
        // from the battlefield it's a dies trigger, which looks back in time
        super(Zone.BATTLEFIELD, Zone.GRAVEYARD, new ConjureCardEffect("Gixian Recycler", Zone.GRAVEYARD, 1),
                "When {this} dies or is put into your graveyard from your hand or library, ", false);
    }

    private GixianRecyclerTriggeredAbility(final GixianRecyclerTriggeredAbility ability) {
        super(ability);
    }

    @Override
    public GixianRecyclerTriggeredAbility copy() {
        return new GixianRecyclerTriggeredAbility(this);
    }

    @Override
    public boolean isInUseableZone(Game game, MageObject sourceObject, GameEvent event) {
        // dying looks back to the permanent; from the hand or library it triggers from the graveyard
        if (event instanceof ZoneChangeEvent && ((ZoneChangeEvent) event).getFromZone() == Zone.BATTLEFIELD) {
            return TriggeredAbilityImpl.isInUseableZoneDiesTrigger(this, sourceObject, event, game);
        }
        return super.isInUseableZone(game, sourceObject, event);
    }

    @Override
    public boolean checkTrigger(GameEvent event, Game game) {
        if (!event.getTargetId().equals(getSourceId())) {
            return false;
        }
        ZoneChangeEvent zEvent = (ZoneChangeEvent) event;
        if (zEvent.getToZone() != Zone.GRAVEYARD) {
            return false;
        }
        switch (zEvent.getFromZone()) {
            case BATTLEFIELD:
            case HAND:
            case LIBRARY:
                return true;
            default:
                return false;
        }
    }
}
