package mage.cards.f;

import mage.MageInt;
import mage.abilities.Ability;
import mage.abilities.common.EntersBattlefieldTriggeredAbility;
import mage.abilities.costs.mana.ManaCostsImpl;
import mage.abilities.effects.ContinuousEffect;
import mage.abilities.effects.OneShotEffect;
import mage.abilities.effects.common.PerpetualGainAbilityEffect;
import mage.abilities.effects.common.PerpetuallyBoostSourceEffect;
import mage.abilities.keyword.HasteAbility;
import mage.abilities.keyword.UnearthAbility;
import mage.cards.Card;
import mage.cards.CardImpl;
import mage.cards.CardSetInfo;
import mage.cards.repository.CardCriteria;
import mage.cards.repository.CardInfo;
import mage.cards.repository.CardRepository;
import mage.constants.CardType;
import mage.constants.Outcome;
import mage.constants.SubType;
import mage.constants.Zone;
import mage.filter.FilterPermanent;
import mage.filter.common.FilterControlledPermanent;
import mage.filter.predicate.Predicates;
import mage.filter.predicate.mageobject.AnotherPredicate;
import mage.filter.predicate.permanent.TokenPredicate;
import mage.game.Game;
import mage.game.permanent.Permanent;
import mage.players.Player;
import mage.target.TargetPermanent;

import java.util.Collections;
import java.util.UUID;

/**
 * @author playmat
 */
public final class FallajiAntiquarian extends CardImpl {

    private static final FilterPermanent filter = new FilterControlledPermanent(
            "another target nontoken creature or artifact you control"
    );

    static {
        filter.add(AnotherPredicate.instance);
        filter.add(TokenPredicate.FALSE);
        filter.add(Predicates.or(
                CardType.CREATURE.getPredicate(),
                CardType.ARTIFACT.getPredicate()
        ));
    }

    public FallajiAntiquarian(UUID ownerId, CardSetInfo setInfo) {
        super(ownerId, setInfo, new CardType[]{CardType.CREATURE}, "{3}{R}");

        this.subtype.add(SubType.HUMAN);
        this.subtype.add(SubType.ARTIFICER);
        this.power = new MageInt(3);
        this.toughness = new MageInt(3);

        // Haste
        this.addAbility(HasteAbility.getInstance());

        // When Fallaji Antiquarian enters, conjure a duplicate of another target nontoken creature or artifact
        // you control into your graveyard. The duplicate perpetually gains unearth {1}{R}.
        Ability ability = new EntersBattlefieldTriggeredAbility(new FallajiAntiquarianEffect());
        ability.addTarget(new TargetPermanent(filter));
        this.addAbility(ability);
    }

    private FallajiAntiquarian(final FallajiAntiquarian card) {
        super(card);
    }

    @Override
    public FallajiAntiquarian copy() {
        return new FallajiAntiquarian(this);
    }
}

class FallajiAntiquarianEffect extends OneShotEffect {

    FallajiAntiquarianEffect() {
        super(Outcome.PutCardInPlay);
        staticText = "conjure a duplicate of another target nontoken creature or artifact you control "
                + "into your graveyard. The duplicate perpetually gains unearth {1}{R}";
    }

    private FallajiAntiquarianEffect(final FallajiAntiquarianEffect effect) {
        super(effect);
    }

    @Override
    public FallajiAntiquarianEffect copy() {
        return new FallajiAntiquarianEffect(this);
    }

    @Override
    public boolean apply(Game game, Ability source) {
        Player controller = game.getPlayer(source.getControllerId());
        Permanent target = game.getPermanent(getTargetPointer().getFirst(game, source));
        if (controller == null || target == null) {
            return false;
        }

        Card originalCard = target.getMainCard();
        Card duplicate = createDuplicateCard(target, originalCard);
        if (duplicate == null) {
            return false;
        }

        game.loadCards(Collections.singleton(duplicate), controller.getId());
        controller.moveCards(duplicate, Zone.GRAVEYARD, source, game);

        // Copy perpetual modifications from the original card onto the duplicate
        if (originalCard != null) {
            UUID originalId = originalCard.getMainCard().getId();
            int[] boost = PerpetuallyBoostSourceEffect.getTotal(originalId, game);
            if (boost != null) {
                PerpetuallyBoostSourceEffect.perpetuallyBoost(duplicate.getId(), boost[0], boost[1], game, source);
            }
            for (ContinuousEffect effect : game.getState().getContinuousEffects().getLayeredEffects(game)) {
                if (effect instanceof PerpetualGainAbilityEffect) {
                    PerpetualGainAbilityEffect gain = (PerpetualGainAbilityEffect) effect;
                    if (originalId.equals(gain.getCardId())) {
                        game.addEffect(new PerpetualGainAbilityEffect(duplicate.getId(), gain.getAbility()), source);
                    }
                }
            }
        }

        // The duplicate perpetually gains unearth {1}{R}
        game.addEffect(
                new PerpetualGainAbilityEffect(duplicate.getId(), new UnearthAbility(new ManaCostsImpl<>("{1}{R}"))),
                source
        );
        return true;
    }

    private static Card createDuplicateCard(Permanent target, Card originalCard) {
        CardInfo cardInfo = findCardInfo(target.getName());
        if (cardInfo == null && originalCard != null) {
            cardInfo = findCardInfo(originalCard.getName());
        }
        if (cardInfo != null) {
            return cardInfo.createCard();
        }
        if (originalCard != null) {
            Card copy = originalCard.copy();
            copy.assignNewId();
            return copy;
        }
        return null;
    }

    private static CardInfo findCardInfo(String name) {
        if (name == null || name.isEmpty()) {
            return null;
        }
        return CardRepository.instance
                .findCards(new CardCriteria().name(name))
                .stream()
                .findFirst()
                .orElse(null);
    }
}
