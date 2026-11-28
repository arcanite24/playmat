package mage.cards.t;

import mage.MageIdentifier;
import mage.MageObjectReference;
import mage.abilities.Ability;
import mage.abilities.SpellAbility;
import mage.abilities.common.SimpleStaticAbility;
import mage.abilities.effects.AsThoughEffectImpl;
import mage.abilities.hint.common.OpenSideboardHint;
import mage.cards.Card;
import mage.cards.CardImpl;
import mage.cards.CardSetInfo;
import mage.constants.AsThoughEffectType;
import mage.constants.CardType;
import mage.constants.Duration;
import mage.constants.Outcome;
import mage.constants.SuperType;
import mage.constants.WatcherScope;
import mage.constants.Zone;
import mage.game.Game;
import mage.game.events.GameEvent;
import mage.players.Player;
import mage.util.CardUtil;
import mage.watchers.Watcher;

import java.util.HashSet;
import java.util.Set;
import java.util.UUID;

/**
 * @author arcanite24
 */
public final class TheHolodeck extends CardImpl {

    public TheHolodeck(UUID ownerId, CardSetInfo setInfo) {
        super(ownerId, setInfo, new CardType[]{CardType.ARTIFACT}, "{7}");

        this.supertype.add(SuperType.LEGENDARY);

        // Once during each of your turns, you may play a permanent card you own from outside the game or from your graveyard.
        Ability ability = new SimpleStaticAbility(new TheHolodeckPlayEffect());
        ability.setIdentifier(MageIdentifier.TheHolodeckWatcher);
        ability.addHint(OpenSideboardHint.instance);
        this.addAbility(ability, new TheHolodeckWatcher());
    }

    private TheHolodeck(final TheHolodeck card) {
        super(card);
    }

    @Override
    public TheHolodeck copy() {
        return new TheHolodeck(this);
    }
}

class TheHolodeckPlayEffect extends AsThoughEffectImpl {

    TheHolodeckPlayEffect() {
        super(AsThoughEffectType.PLAY_FROM_NOT_OWN_HAND_ZONE, Duration.WhileOnBattlefield, Outcome.PutCardInPlay);
        staticText = "once during each of your turns, you may play a permanent card you own "
                + "from outside the game or from your graveyard";
    }

    private TheHolodeckPlayEffect(final TheHolodeckPlayEffect effect) {
        super(effect);
    }

    @Override
    public TheHolodeckPlayEffect copy() {
        return new TheHolodeckPlayEffect(this);
    }

    @Override
    public boolean apply(Game game, Ability source) {
        return true;
    }

    @Override
    public boolean applies(UUID objectId, Ability source, UUID affectedControllerId, Game game) {
        return applies(objectId, null, source, game, affectedControllerId);
    }

    @Override
    public boolean applies(UUID objectId, Ability affectedAbility, Ability source, Game game, UUID playerId) {
        if (!source.isControlledBy(playerId)
                || !game.isActivePlayer(playerId) // only during your turns
                || TheHolodeckWatcher.used(source, game)) { // once each turn
            return false;
        }
        Player player = game.getPlayer(playerId);
        UUID mainCardId = CardUtil.getMainCardId(game, objectId);
        Card card = game.getCard(mainCardId);
        if (player == null || card == null || !card.isOwnedBy(playerId)
                || !card.isPermanent(game)) { // a permanent card, as it is in that zone
            return false;
        }
        boolean fromGraveyard = Zone.GRAVEYARD.equals(game.getState().getZone(mainCardId))
                && player.getGraveyard().contains(mainCardId);
        boolean fromOutside = player.getSideboard().contains(mainCardId);
        if (!fromGraveyard && !fromOutside) {
            return false;
        }
        if (affectedAbility instanceof SpellAbility) {
            // cast it as a permanent spell, not as an Adventure or another nonpermanent half
            Card spellCard = ((SpellAbility) affectedAbility).getCharacteristics(game);
            return spellCard != null && spellCard.isPermanent(game);
        }
        return true;
    }
}

/**
 * The Holodecks whose permission was used this turn.
 */
class TheHolodeckWatcher extends Watcher {

    private final Set<MageObjectReference> usedFrom = new HashSet<>();

    TheHolodeckWatcher() {
        super(WatcherScope.GAME);
    }

    @Override
    public void watch(GameEvent event, Game game) {
        if ((event.getType() == GameEvent.EventType.SPELL_CAST
                || event.getType() == GameEvent.EventType.LAND_PLAYED)
                && event.hasApprovingIdentifier(MageIdentifier.TheHolodeckWatcher)) {
            usedFrom.add(event.getApprovingObject().getApprovingMageObjectReference());
        }
    }

    @Override
    public void reset() {
        super.reset();
        usedFrom.clear();
    }

    static boolean used(Ability source, Game game) {
        TheHolodeckWatcher watcher = game.getState().getWatcher(TheHolodeckWatcher.class);
        return watcher != null && watcher.usedFrom.contains(new MageObjectReference(source.getSourceId(), game));
    }
}
