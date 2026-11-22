package org.mage.test.player;

import mage.abilities.Ability;
import mage.abilities.common.SimpleActivatedAbility;
import mage.abilities.costs.mana.GenericManaCost;
import mage.abilities.effects.common.InfoEffect;
import mage.cards.Card;
import mage.cards.decks.Deck;
import mage.cards.repository.CardInfo;
import mage.cards.repository.CardRepository;
import mage.constants.MultiplayerAttackOption;
import mage.constants.PlayerAction;
import mage.constants.RangeOfInfluence;
import mage.constants.Zone;
import mage.game.Game;
import mage.game.TwoPlayerDuel;
import mage.game.mulligan.LondonMulligan;
import mage.game.permanent.Permanent;
import mage.game.stack.SpellStack;
import mage.game.stack.StackAbility;
import mage.game.turn.CombatPhase;
import mage.game.turn.DeclareAttackersStep;
import mage.player.human.HumanPlayer;
import mage.players.net.UserData;
import org.junit.Assert;
import org.junit.Test;

import java.util.Collections;
import java.util.UUID;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * Regression tests for #460: PASS_PRIORITY_UNTIL_STACK_RESOLVED ("Resolve the stack") must resolve
 * all existing items on the stack during Declare Attackers (including for the defending player when
 * stopOnDeclareBlockersWithAnyPermanents is enabled), stop if a new object is added to the stack,
 * and stop before Declare Blockers once the stack empties.
 */
public class ResolveStackPriorityTest {

    private static final class RecordingHumanPlayer extends HumanPlayer {
        final AtomicInteger prompts = new AtomicInteger();

        RecordingHumanPlayer(String name) {
            super(name, RangeOfInfluence.ALL, 5);
            UserData ud = UserData.getDefaultUserDataView();
            ud.getUserSkipPrioritySteps().setStopOnDeclareBlockersWithAnyPermanents(true);
            ud.getUserSkipPrioritySteps().setStopOnStackNewObjects(true);
            setUserData(ud);
        }

        @Override
        protected void prepareForResponse(Game game) {
            // Avoid game-thread assertion in direct unit test invocation
        }

        @Override
        protected void waitForResponse(Game game) {
            prompts.incrementAndGet();
            skip();
        }
    }

    private static Card loadCard(String name, Game game, UUID ownerId) {
        CardInfo info = CardRepository.instance.findCard(name);
        Assert.assertNotNull("Card not found: " + name, info);
        Card card = info.createCard();
        game.loadCards(Collections.singleton(card), ownerId);
        return card;
    }

    private static void setDeclareAttackersStep(Game game) {
        CombatPhase combat = new CombatPhase();
        combat.setStep(new DeclareAttackersStep());
        game.getState().getTurn().setPhase(combat);
    }

    @Test
    public void testDefenderResolveStackInDeclareAttackersPassesAllExistingTriggersAndStopsWhenEmpty() {
        TwoPlayerDuel game = new TwoPlayerDuel(
                MultiplayerAttackOption.LEFT, RangeOfInfluence.ALL, new LondonMulligan(0), 60, 20, 7
        );
        RecordingHumanPlayer attacker = new RecordingHumanPlayer("Attacker");
        RecordingHumanPlayer defender = new RecordingHumanPlayer("Defender");
        game.addPlayer(attacker, new Deck());
        game.addPlayer(defender, new Deck());
        game.getState().getPlayerList().setCurrent(attacker.getId());
        game.getState().setActivePlayerId(attacker.getId());
        setDeclareAttackersStep(game);

        Card goblinCard = loadCard("Raging Goblin", game, attacker.getId());
        attacker.moveCards(goblinCard, Zone.BATTLEFIELD, null, game);
        Permanent goblin = game.getPermanent(goblinCard.getId());
        Assert.assertNotNull(goblin);
        game.getCombat().addAttackerToCombat(goblin.getId(), defender.getId(), game);

        Card blockerCard = loadCard("Grizzly Bears", game, defender.getId());
        defender.moveCards(blockerCard, Zone.BATTLEFIELD, null, game);
        game.applyEffects();

        // Push 4 Shared Animosity-like triggers onto the stack
        for (int i = 0; i < 4; i++) {
            Ability trigger = new SimpleActivatedAbility(new InfoEffect("Shared Animosity trigger " + i), new GenericManaCost(0));
            trigger.setSourceId(goblin.getId());
            trigger.setControllerId(attacker.getId());
            game.getStack().push(game, new StackAbility(trigger, attacker.getId()));
        }

        // Defender presses "Resolve the stack" (F10) on the first trigger
        defender.sendPlayerAction(PlayerAction.PASS_PRIORITY_UNTIL_STACK_RESOLVED, game, null);
        Assert.assertTrue(defender.getPassedUntilStackResolved());

        // Simulate 1st trigger resolving; 3 triggers remain on stack -> defender auto-passes all 3 without prompting
        game.getStack().remove(game.getStack().peek(), game);
        for (int remaining = 3; remaining >= 1; remaining--) {
            defender.resetPassed();
            defender.priority(game);
            Assert.assertTrue("Defender should have passed priority with " + remaining + " items left", defender.isPassed());
            Assert.assertEquals("Defender should NOT be prompted while resolving existing stack", 0, defender.prompts.get());
            game.getStack().remove(game.getStack().peek(), game);
        }

        // Stack is now empty in Declare Attackers -> defender MUST stop before Declare Blockers and flag cleared
        Assert.assertTrue(game.getStack().isEmpty());
        defender.resetPassed();
        defender.priority(game);
        Assert.assertEquals("Defender must be prompted once stack empties before blockers", 1, defender.prompts.get());
        Assert.assertFalse("passedUntilStackResolved must be cleared once stack empties", defender.getPassedUntilStackResolved());
    }

    @Test
    public void testDefenderResolveStackStopsWhenNewObjectAddedToStack() {
        TwoPlayerDuel game = new TwoPlayerDuel(
                MultiplayerAttackOption.LEFT, RangeOfInfluence.ALL, new LondonMulligan(0), 60, 20, 7
        );
        RecordingHumanPlayer attacker = new RecordingHumanPlayer("Attacker");
        RecordingHumanPlayer defender = new RecordingHumanPlayer("Defender");
        game.addPlayer(attacker, new Deck());
        game.addPlayer(defender, new Deck());
        game.getState().getPlayerList().setCurrent(attacker.getId());
        game.getState().setActivePlayerId(attacker.getId());
        setDeclareAttackersStep(game);

        Card goblinCard = loadCard("Raging Goblin", game, attacker.getId());
        attacker.moveCards(goblinCard, Zone.BATTLEFIELD, null, game);
        Permanent goblin = game.getPermanent(goblinCard.getId());
        Assert.assertNotNull(goblin);
        game.getCombat().addAttackerToCombat(goblin.getId(), defender.getId(), game);

        Card blockerCard = loadCard("Grizzly Bears", game, defender.getId());
        defender.moveCards(blockerCard, Zone.BATTLEFIELD, null, game);
        game.applyEffects();

        for (int i = 0; i < 2; i++) {
            Ability trigger = new SimpleActivatedAbility(new InfoEffect("trigger " + i), new GenericManaCost(0));
            trigger.setSourceId(goblin.getId());
            trigger.setControllerId(attacker.getId());
            game.getStack().push(game, new StackAbility(trigger, attacker.getId()));
        }

        defender.sendPlayerAction(PlayerAction.PASS_PRIORITY_UNTIL_STACK_RESOLVED, game, null);

        // Attacker responds with a new spell/ability on top of the stack (even within the same millisecond)
        Ability response = new SimpleActivatedAbility(new InfoEffect("new spell"), new GenericManaCost(0));
        response.setSourceId(goblin.getId());
        response.setControllerId(attacker.getId());
        game.getStack().push(game, new StackAbility(response, attacker.getId()));

        defender.resetPassed();
        defender.priority(game);
        Assert.assertEquals("Defender must be prompted when a new object is added to the stack", 1, defender.prompts.get());
        Assert.assertFalse("passedUntilStackResolved must be cancelled when a new object is added", defender.getPassedUntilStackResolved());
    }

    @Test
    public void testSpellStackCopyPreservesLastAddedMetadata() {
        TwoPlayerDuel game = new TwoPlayerDuel(
                MultiplayerAttackOption.LEFT, RangeOfInfluence.ALL, new LondonMulligan(0), 60, 20, 7
        );
        RecordingHumanPlayer p = new RecordingHumanPlayer("P");
        game.addPlayer(p, new Deck());
        Ability ability = new SimpleActivatedAbility(new InfoEffect("test"), new GenericManaCost(0));
        ability.setControllerId(p.getId());
        game.getStack().push(game, new StackAbility(ability, p.getId()));

        SpellStack copy = game.getStack().copy();
        Assert.assertEquals(game.getStack().getLastAddedSeq(), copy.getLastAddedSeq());
        Assert.assertEquals(game.getStack().getDateLastAdded(), copy.getDateLastAdded());
    }
}
