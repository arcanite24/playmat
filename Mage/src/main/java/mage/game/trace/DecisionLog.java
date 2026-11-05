package mage.game.trace;

import mage.MageObject;
import mage.abilities.Ability;
import mage.abilities.Mode;
import mage.abilities.Modes;
import mage.cards.Card;
import mage.constants.ManaType;
import mage.game.Game;
import mage.game.permanent.Permanent;
import mage.game.stack.StackObject;
import mage.players.Player;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.UUID;
import java.util.function.BiPredicate;

/**
 * Every answer the players gave, in order (Rust migration phase 0). With the decks and the seed it replays a game:
 * {@link mage.game.GameOptions#decisions} set to {@link #recording()} keeps the answers, set to {@link #replaying(List)} gives
 * them back instead of waiting for a client. A game replayed this way gives the recorded trace again.
 * <p>
 * Object ids change from game to game, so an answer names an object by where it is when the answer is given: a seat,
 * a zone and a position ({@code P1}, {@code H0.3}, {@code B5}, {@code S0}), an ability of one ({@code B5a2}) or a mode
 * of that ability ({@code H0.3a0m1}).
 *
 * @see mage.game.GameOptions#decisions
 */
public final class DecisionLog {

    public static final class Decision {
        public final int seat;
        /** an object reference, or null */
        public final String target;
        public final Boolean bool;
        public final Integer integer;
        public final String string;
        public final String manaType;
        /** a player reference, or null */
        public final String manaPlayer;

        public Decision(int seat, String target, Boolean bool, Integer integer, String string, String manaType, String manaPlayer) {
            this.seat = seat;
            this.target = target;
            this.bool = bool;
            this.integer = integer;
            this.string = string;
            this.manaType = manaType;
            this.manaPlayer = manaPlayer;
        }

        @Override
        public String toString() {
            StringBuilder out = new StringBuilder("seat ").append(seat);
            if (target != null) out.append(" target ").append(target);
            if (bool != null) out.append(" bool ").append(bool);
            if (integer != null) out.append(" int ").append(integer);
            if (string != null) out.append(" string ").append(string);
            if (manaType != null) out.append(" mana ").append(manaType).append(' ').append(manaPlayer);
            return out.toString();
        }
    }

    /** An answer given back, with the references turned into this game's ids. */
    public static final class Answer {
        public UUID target;
        public Boolean bool;
        public Integer integer;
        public String string;
        public ManaType manaType;
        public UUID manaPlayer;
    }

    private final boolean replay;
    private final List<Decision> decisions;
    private int next = 0;
    /** set when a replay can't go on: what went wrong */
    private String failure;

    private DecisionLog(boolean replay, List<Decision> decisions) {
        this.replay = replay;
        this.decisions = decisions;
    }

    public static DecisionLog recording() {
        return new DecisionLog(false, new ArrayList<>());
    }

    public static DecisionLog replaying(List<Decision> decisions) {
        return new DecisionLog(true, new ArrayList<>(decisions));
    }

    public boolean isReplay() {
        return replay;
    }

    public synchronized List<Decision> getDecisions() {
        return Collections.unmodifiableList(new ArrayList<>(decisions));
    }

    /** answers given back so far (replay) */
    public synchronized int used() {
        return next;
    }

    public synchronized String getFailure() {
        return failure;
    }

    public synchronized void record(Game game, UUID playerId, UUID target, Boolean bool, Integer integer, String string,
                                    ManaType manaType, UUID manaPlayer) {
        if (replay) {
            return;
        }
        decisions.add(new Decision(seat(game, playerId), ref(game, target), bool, integer, string,
                manaType == null ? null : manaType.name(), ref(game, manaPlayer)));
    }

    /**
     * The next answer, for this player. Null when the log is used up or names another player or an object that isn't
     * there: the replay has gone off the recorded game ({@link #getFailure()}).
     */
    public synchronized Answer next(Game game, UUID playerId) {
        if (failure != null) {
            return null;
        }
        if (next >= decisions.size()) {
            failure = "the log ends after " + decisions.size() + " answers";
            return null;
        }
        Decision decision = decisions.get(next);
        int seat = seat(game, playerId);
        if (decision.seat != seat) {
            failure = "answer " + next + " is seat " + decision.seat + "'s, but seat " + seat + " is asked";
            return null;
        }
        Answer answer = new Answer();
        answer.target = resolve(game, decision.target);
        answer.manaPlayer = resolve(game, decision.manaPlayer);
        if ((decision.target != null && answer.target == null) || (decision.manaPlayer != null && answer.manaPlayer == null)) {
            failure = "answer " + next + " names " + decision + ", which isn't in this game";
            return null;
        }
        answer.bool = decision.bool;
        answer.integer = decision.integer;
        answer.string = decision.string;
        answer.manaType = decision.manaType == null ? null : ManaType.valueOf(decision.manaType);
        next++;
        return answer;
    }

    private static int seat(Game game, UUID playerId) {
        return StateFingerprint.seats(game).indexOf(playerId);
    }

    /** where the object is now, or {@code ?<id>} when it can't be placed */
    public static String ref(Game game, UUID id) {
        if (id == null) {
            return null;
        }
        // CHOOSE_MODE's Done and Cancel
        if (id.equals(Modes.CHOOSE_OPTION_DONE_ID)) {
            return "MD";
        }
        if (id.equals(Modes.CHOOSE_OPTION_CANCEL_ID)) {
            return "MC";
        }
        String[] found = {null};
        visit(game, (ref, objectId) -> {
            if (id.equals(objectId)) {
                found[0] = ref;
                return true;
            }
            return false;
        });
        return found[0] != null ? found[0] : "?" + id;
    }

    public static UUID resolve(Game game, String ref) {
        if (ref == null) {
            return null;
        }
        if (ref.equals("MD")) {
            return Modes.CHOOSE_OPTION_DONE_ID;
        }
        if (ref.equals("MC")) {
            return Modes.CHOOSE_OPTION_CANCEL_ID;
        }
        UUID[] found = {null};
        visit(game, (candidate, objectId) -> {
            if (ref.equals(candidate)) {
                found[0] = objectId;
                return true;
            }
            return false;
        });
        return found[0];
    }

    /**
     * Every object an answer can name, with its reference, in a fixed order; stops when {@code visitor} says so.
     */
    private static boolean visit(Game game, BiPredicate<String, UUID> visitor) {
        List<UUID> seats = StateFingerprint.seats(game);
        for (int s = 0; s < seats.size(); s++) {
            if (visitor.test("P" + s, seats.get(s))) {
                return true;
            }
        }
        for (int s = 0; s < seats.size(); s++) {
            Player player = game.getPlayer(seats.get(s));
            if (player == null) {
                continue;
            }
            if (cards(game, "H" + s + ".", player.getHand(), visitor)
                    || cards(game, "L" + s + ".", player.getLibrary().getCardList(), visitor)
                    || cards(game, "G" + s + ".", player.getGraveyard(), visitor)) {
                return true;
            }
        }
        int i = 0;
        for (Permanent permanent : game.getState().getBattlefield().getAllPermanents()) {
            if (object(game, "B" + i++, permanent, visitor)) {
                return true;
            }
        }
        i = 0;
        for (StackObject object : game.getState().getStack()) {
            if (object(game, "S" + i++, object, visitor)) {
                return true;
            }
        }
        i = 0;
        for (Card card : game.getState().getExile().getAllCards(game)) {
            if (object(game, "E" + i++, card, visitor)) {
                return true;
            }
        }
        i = 0;
        for (MageObject object : game.getState().getCommand()) {
            if (object(game, "M" + i++, object, visitor)) {
                return true;
            }
        }
        i = 0;
        for (Ability ability : game.getState().getSpecialActions()) {
            if (ability(ability, "X" + i++, visitor)) {
                return true;
            }
        }
        for (int s = 0; s < seats.size(); s++) {
            i = 0;
            for (Ability ability : game.getState().getTriggered(seats.get(s))) {
                if (ability(ability, "T" + s + "." + i++, visitor)) {
                    return true;
                }
            }
        }
        return false;
    }

    private static boolean cards(Game game, String prefix, Iterable<UUID> ids, BiPredicate<String, UUID> visitor) {
        int i = 0;
        for (UUID id : ids) {
            if (object(game, prefix + i++, game.getCard(id), visitor)) {
                return true;
            }
        }
        return false;
    }

    private static boolean cards(Game game, String prefix, List<Card> cards, BiPredicate<String, UUID> visitor) {
        int i = 0;
        for (Card card : cards) {
            if (object(game, prefix + i++, card, visitor)) {
                return true;
            }
        }
        return false;
    }

    /** the object, then each of its abilities and their modes */
    private static boolean object(Game game, String ref, MageObject object, BiPredicate<String, UUID> visitor) {
        if (object == null) {
            return false;
        }
        if (visitor.test(ref, object.getId())) {
            return true;
        }
        if (object instanceof StackObject && ((StackObject) object).getStackAbility() != null
                && ability(((StackObject) object).getStackAbility(), ref + "s", visitor)) {
            return true;
        }
        Iterable<? extends Ability> abilities = object instanceof Card ? ((Card) object).getAbilities(game) : object.getAbilities();
        int j = 0;
        for (Ability ability : abilities) {
            if (ability(ability, ref + "a" + j++, visitor)) {
                return true;
            }
        }
        return false;
    }

    private static boolean ability(Ability ability, String ref, BiPredicate<String, UUID> visitor) {
        if (visitor.test(ref, ability.getId())) {
            return true;
        }
        int k = 0;
        for (Mode mode : ability.getModes().values()) {
            if (visitor.test(ref + "m" + k++, mode.getId())) {
                return true;
            }
        }
        return false;
    }
}
