package mage.game.advice;

import java.io.Serializable;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;

/**
 * What an advisor would do in a player's seat: the moves in order, or the attack or blocks to declare.
 */
public class Advice implements Serializable {

    private static final long serialVersionUID = 1L;

    public enum AdviceKind {
        /** cast, play or activate the steps in order */
        PLAY,
        /** nothing worth doing: pass */
        PASS,
        /** attack with the listed creatures */
        ATTACK,
        /** declare no attackers */
        NO_ATTACK,
        /** block as listed */
        BLOCK,
        /** declare no blockers */
        NO_BLOCK,
        /** nothing to say about this question */
        NONE
    }

    /** One thing to do: a spell, land or ability, or one attacker or blocker. */
    public static class AdviceStep implements Serializable {
        private static final long serialVersionUID = 1L;
        /** e.g. "Cast Grizzly Bears", "Hill Giant attacks you", "Grizzly Bears blocks Hill Giant" */
        public String text;
        /** the card or permanent it's about */
        public UUID objectId;
        /** what it targets, or the attacker a blocker blocks, or the player or planeswalker an attacker attacks */
        public List<UUID> targetIds = new ArrayList<>();

        public AdviceStep() {
        }

        public AdviceStep(String text, UUID objectId) {
            this.text = text;
            this.objectId = objectId;
        }
    }

    public AdviceKind kind = AdviceKind.NONE;
    /** one line to show the player */
    public String text;
    public List<AdviceStep> steps = new ArrayList<>();
    /** the turn and step the advice is for, e.g. "Turn 4, precombat main" */
    public String when;
    /** how long the advisor thought, in milliseconds */
    public long millis;
    /** why, in one line: "Removes their best blocker", "Holding mana up beats any play here" */
    public String reason;
    /** hints left in this game, where hints are counted (Career); null where they aren't */
    public Integer hintsLeft;

    public static Advice none(String why) {
        Advice advice = new Advice();
        advice.kind = AdviceKind.NONE;
        advice.text = why;
        return advice;
    }
}
