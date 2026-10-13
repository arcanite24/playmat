package mage.player.ai;

import mage.MageObject;
import mage.abilities.Ability;
import mage.abilities.PlayLandAbility;
import mage.abilities.SpellAbility;
import mage.abilities.common.PassAbility;
import mage.constants.PhaseStep;
import mage.constants.RangeOfInfluence;
import mage.game.Game;
import mage.game.advice.Advice;
import mage.game.advice.GameAdvisor;
import mage.game.combat.CombatGroup;
import mage.game.permanent.Permanent;
import mage.game.turn.Step;
import mage.players.Player;
import mage.target.Target;
import mage.util.CardUtil;

import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.UUID;

/**
 * The mad AI as an advisor: it takes the player's seat in a copy of the game, searches as it would for itself, and
 * reports what it chose instead of doing it. Priority moves come from the same search the AI plays with; attacks and
 * blocks from its combat logic.
 */
public final class MadAdvisor implements GameAdvisor {

    @Override
    public Advice advise(Game game, UUID playerId, int skill) {
        long started = System.currentTimeMillis();
        Player seat = game.getPlayer(playerId);
        if (seat == null || !seat.isInGame()) {
            return Advice.none("You aren't in this game any more.");
        }
        AdvisorPlayer advisor = new AdvisorPlayer(seat.getName(), seat.getRange(), Math.max(1, Math.min(10, skill)), playerId);
        Player real = seat.getRealPlayer();
        advisor.restore(real);
        // the AI's search copies the match player into its simulated players
        advisor.setMatchPlayer(real.getMatchPlayer());
        advisor.setTestMode(false);
        game.getState().getPlayers().put(playerId, advisor);

        Advice advice;
        Step step = game.getStep();
        PhaseStep stepType = game.getTurnStepType();
        // attackers and blockers are declared as the step begins, before anyone gets priority in it
        boolean declaring = step != null && step.getStepPart() != Step.StepPart.PRIORITY;
        if (stepType == PhaseStep.DECLARE_ATTACKERS && declaring && game.isActivePlayer(playerId)
                && game.getCombat().getAttackers().isEmpty()) {
            advice = attack(game, advisor, playerId);
        } else if (stepType == PhaseStep.DECLARE_BLOCKERS && declaring && !game.isActivePlayer(playerId)
                && game.getCombat().getBlockers().isEmpty()) {
            advice = block(game, advisor, playerId);
        } else {
            advice = priority(game, advisor);
        }
        advice.when = "Turn " + game.getTurnNum() + ", " + describe(stepType);
        advice.millis = System.currentTimeMillis() - started;
        return advice;
    }

    private static Advice priority(Game game, AdvisorPlayer advisor) {
        advisor.think(game);
        List<Ability> chosen = advisor.chosenActions();
        Advice advice = new Advice();
        List<String> lines = new ArrayList<>();
        for (Ability ability : chosen) {
            if (ability instanceof PassAbility) {
                continue;
            }
            Advice.AdviceStep step = new Advice.AdviceStep(describe(game, ability), ability.getSourceId());
            for (Target target : ability.getAllSelectedTargets()) {
                step.targetIds.addAll(target.getTargets());
            }
            advice.steps.add(step);
            lines.add(step.text);
        }
        if (advice.steps.isEmpty()) {
            advice.kind = Advice.AdviceKind.PASS;
            advice.text = "Pass: nothing here is worth doing now.";
        } else {
            advice.kind = Advice.AdviceKind.PLAY;
            advice.text = String.join(", then ", lines) + ".";
        }
        return advice;
    }

    private static Advice attack(Game game, AdvisorPlayer advisor, UUID playerId) {
        advisor.selectAttackers(game, playerId);
        Advice advice = new Advice();
        List<String> lines = new ArrayList<>();
        for (CombatGroup group : game.getCombat().getGroups()) {
            for (UUID attackerId : group.getAttackers()) {
                Advice.AdviceStep step = new Advice.AdviceStep(name(game, attackerId) + " attacks " + defender(game, group.getDefenderId(), playerId), attackerId);
                if (group.getDefenderId() != null) {
                    step.targetIds.add(group.getDefenderId());
                }
                advice.steps.add(step);
                lines.add(step.text);
            }
        }
        if (advice.steps.isEmpty()) {
            advice.kind = Advice.AdviceKind.NO_ATTACK;
            advice.text = "Don't attack this turn.";
        } else {
            advice.kind = Advice.AdviceKind.ATTACK;
            advice.text = String.join("; ", lines) + ".";
        }
        return advice;
    }

    private static Advice block(Game game, AdvisorPlayer advisor, UUID playerId) {
        advisor.selectBlockers(null, game, playerId);
        Advice advice = new Advice();
        List<String> lines = new ArrayList<>();
        for (CombatGroup group : game.getCombat().getGroups()) {
            for (UUID blockerId : group.getBlockers()) {
                Permanent blocker = game.getPermanent(blockerId);
                if (blocker == null || !blocker.isControlledBy(playerId)) {
                    continue;
                }
                List<String> attackers = new ArrayList<>();
                Advice.AdviceStep step = new Advice.AdviceStep(null, blockerId);
                for (UUID attackerId : group.getAttackers()) {
                    attackers.add(name(game, attackerId));
                    step.targetIds.add(attackerId);
                }
                step.text = name(game, blockerId) + " blocks " + String.join(" and ", attackers);
                advice.steps.add(step);
                lines.add(step.text);
            }
        }
        if (advice.steps.isEmpty()) {
            advice.kind = Advice.AdviceKind.NO_BLOCK;
            advice.text = "Don't block.";
        } else {
            advice.kind = Advice.AdviceKind.BLOCK;
            advice.text = String.join("; ", lines) + ".";
        }
        return advice;
    }

    static String describe(Game game, Ability ability) {
        MageObject source = ability.getSourceObject(game);
        String name = source == null ? "an ability" : source.getName();
        String text;
        if (ability instanceof PlayLandAbility) {
            text = "Play " + name;
        } else if (ability instanceof SpellAbility) {
            text = "Cast " + name;
        } else {
            text = "Use " + name + ": " + CardUtil.substring(ability.getRule(true), 60, "...");
        }
        List<String> targets = new ArrayList<>();
        for (Target target : ability.getAllSelectedTargets()) {
            for (UUID id : target.getTargets()) {
                targets.add(targetName(game, id, ability.getControllerId()));
            }
        }
        if (!targets.isEmpty()) {
            text += " targeting " + String.join(", ", targets);
        }
        return text;
    }

    private static String targetName(Game game, UUID id, UUID you) {
        if (id.equals(you)) {
            return "yourself";
        }
        Player player = game.getPlayer(id);
        if (player != null) {
            return player.getName();
        }
        return name(game, id);
    }

    private static String defender(Game game, UUID defenderId, UUID you) {
        if (defenderId == null) {
            return "";
        }
        Player player = game.getPlayer(defenderId);
        if (player != null) {
            return player.getName();
        }
        return name(game, defenderId);
    }

    private static String name(Game game, UUID objectId) {
        MageObject object = game.getObject(objectId);
        return object == null ? "a creature" : object.getName();
    }

    private static String describe(PhaseStep step) {
        if (step == null) {
            return "before the game";
        }
        switch (step) {
            case PRECOMBAT_MAIN:
                return "first main phase";
            case POSTCOMBAT_MAIN:
                return "second main phase";
            case FIRST_COMBAT_DAMAGE:
                return "first-strike damage";
            default:
                return step.getStepText().toLowerCase(Locale.ENGLISH);
        }
    }

    /**
     * The mad AI in someone else's seat: same search, same budget for the skill, but it keeps its choice instead of
     * acting on it.
     */
    static final class AdvisorPlayer extends ComputerPlayer7 {

        AdvisorPlayer(String name, RangeOfInfluence range, int skill, UUID playerId) {
            super(name, range, skill, playerId);
        }

        private AdvisorPlayer(final AdvisorPlayer player) {
            super(player);
        }

        @Override
        public AdvisorPlayer copy() {
            return new AdvisorPlayer(this);
        }

        void think(Game game) {
            actions.clear();
            calculateActions(game);
        }

        List<Ability> chosenActions() {
            return new ArrayList<>(actions);
        }
    }
}
