package mage.server.career;

import mage.abilities.Ability;
import mage.abilities.common.SimpleStaticAbility;
import mage.abilities.common.delayed.AtTheBeginOfNextEndStepDelayedTriggeredAbility;
import mage.abilities.common.delayed.AtTheBeginOfYourNextUpkeepDelayedTriggeredAbility;
import mage.abilities.effects.OneShotEffect;
import mage.abilities.effects.common.InfoEffect;
import mage.abilities.effects.common.LoseGameTargetPlayerEffect;
import mage.abilities.effects.common.WinGameSourceControllerEffect;
import mage.cards.Card;
import mage.cards.repository.CardInfo;
import mage.cards.repository.CardRepository;
import mage.constants.Outcome;
import mage.constants.TargetController;
import mage.constants.Zone;
import mage.game.Game;
import mage.game.GameSetup;
import mage.game.PutToBattlefieldInfo;
import mage.game.command.emblems.EmblemOfCard;
import mage.players.Player;
import mage.target.targetpointer.FixedTarget;
import org.apache.log4j.Logger;

import java.io.Serializable;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.UUID;

/**
 * How a Career game starts when it isn't a plain duel: an opponent with a twist (more life, extra cards, a permanent
 * or an emblem in play), or a puzzle that starts from a fixed position and must be won this turn.
 */
public final class CareerSetup implements GameSetup {

    private static final long serialVersionUID = 1L;
    static final Logger logger = Logger.getLogger(CareerSetup.class);

    /** A boss's edge over a normal opponent. Every field is optional. */
    public static class CareerTwists implements Serializable {
        private static final long serialVersionUID = 1L;
        /** life the opponent starts with */
        public Integer startingLife;
        /** life added to (or, negative, taken from) the starting life: relics, wounds, ascension */
        public int lifeBonus;
        /** cards the opponent draws on top of the opening hand */
        public int extraCards;
        /** cards the opponent starts with in play; "Name" or "Name|tapped" */
        public List<String> permanents;
        /** a card whose abilities the opponent has as an emblem */
        public String emblem;
        /** edges that come in during the game */
        public List<TimedTwist> timed;
    }

    /**
     * An edge that comes in during the game, once: at the beginning of a turn ({@code when} "turn", {@code turn} N:
     * the game's turn number), when a player's life is at or below a threshold ("life", {@code life} N, {@code who}
     * "opponent" by default) or when a player casts a card ("cast", {@code card} name, {@code who} "you" by default).
     * The opponent then gets {@code then} (as a starting twist: life set, cards drawn, permanents, an emblem) and
     * {@code gainLife} life (negative: loses it); {@code message} goes in the game log.
     */
    public static class TimedTwist implements Serializable {
        private static final long serialVersionUID = 1L;
        public static final String TURN = "turn";
        public static final String LIFE = "life";
        public static final String CAST = "cast";

        public String when;
        public Integer turn;
        public Integer life;
        public String card;
        /** "you" (the player) or "opponent" */
        public String who;
        public CareerTwists then;
        public int gainLife;
        public String message;

        /** null when it's well formed, else what's wrong */
        public String problem() {
            if (when == null) {
                return "a timed twist needs when: turn, life or cast";
            }
            switch (when) {
                case TURN:
                    if (turn == null || turn < 1) {
                        return "a turn twist needs a turn of 1 or more";
                    }
                    break;
                case LIFE:
                    if (life == null) {
                        return "a life twist needs a life threshold";
                    }
                    break;
                case CAST:
                    if (card == null || card.trim().isEmpty()) {
                        return "a cast twist needs a card name";
                    }
                    break;
                default:
                    return "a timed twist's when is turn, life or cast, not " + when;
            }
            if (who != null && !who.equalsIgnoreCase("you") && !who.equalsIgnoreCase("opponent")) {
                return "a timed twist's who is you or opponent";
            }
            if (then != null && then.timed != null && !then.timed.isEmpty()) {
                return "a timed twist can't start more timed twists";
            }
            if (then == null && gainLife == 0 && message == null) {
                return "a timed twist needs something to happen";
            }
            return null;
        }
    }

    /** One player's side of a puzzle. Card lists hold names; battlefield entries may end in "|tapped". */
    public static class CareerPuzzleSide implements Serializable {
        private static final long serialVersionUID = 1L;
        public Integer life;
        public List<String> hand;
        public List<String> battlefield;
        public List<String> graveyard;
        /** the library, top card first */
        public List<String> library;
    }

    /**
     * A fixed position: the player must win from it this turn, or (a campaign trial's "survive") live through the
     * opponent's turn.
     */
    public static class Puzzle implements Serializable {
        private static final long serialVersionUID = 1L;
        public String id;
        public String name;
        public String text;
        /** a nudge for a player who's stuck */
        public String hint;
        public CareerPuzzleSide you;
        public CareerPuzzleSide opponent;
        /** "win" (the default), "survive", or "protect": the permanent named {@link #protect} must still be yours */
        public String objective;
        /** for "protect": the name of the player's permanent that must last until their next upkeep */
        public String protect;
        /** who takes the first turn: "you" or "opponent"; by default the opponent in a survive or protect trial */
        public String firstTurn;
        /** puzzles of a series open in order among themselves, like "Stack and priority" */
        public String series;

        /** the objective is met at the player's next upkeep rather than this turn */
        boolean survive() {
            return "survive".equals(objective) || protecting();
        }

        boolean protecting() {
            return "protect".equals(objective);
        }

        boolean opponentFirst() {
            return firstTurn == null ? survive() : "opponent".equals(firstTurn);
        }

        /** what's wrong with the puzzle's objective, or null */
        public String problem() {
            if (objective != null && !"win".equals(objective) && !"survive".equals(objective) && !"protect".equals(objective)) {
                return "an objective is win, survive or protect";
            }
            if (protecting() && (protect == null || you == null || you.battlefield == null
                    || you.battlefield.stream().noneMatch(entry -> name(entry).equals(protect)))) {
                return "a protect objective names a permanent on your battlefield";
            }
            if (firstTurn != null && !"you".equals(firstTurn) && !"opponent".equals(firstTurn)) {
                return "the first turn is you or opponent";
            }
            if (!survive() && "opponent".equals(firstTurn)) {
                return "a puzzle won this turn starts on your turn";
            }
            return null;
        }
    }

    /** the player's next upkeep after the first turn, so a trial the player starts runs through the opponent's turn */
    static class LaterUpkeepTrigger extends AtTheBeginOfYourNextUpkeepDelayedTriggeredAbility {

        LaterUpkeepTrigger(mage.abilities.effects.Effect effect) {
            super(effect);
        }

        private LaterUpkeepTrigger(final LaterUpkeepTrigger ability) {
            super(ability);
        }

        @Override
        public LaterUpkeepTrigger copy() {
            return new LaterUpkeepTrigger(this);
        }

        @Override
        public boolean checkTrigger(mage.game.events.GameEvent event, Game game) {
            return game.getTurnNum() > 1 && super.checkTrigger(event, game);
        }
    }

    /** a protect trial at the player's upkeep: the permanent is still theirs, and they win; it's gone, and they lose */
    static class ProtectedEffect extends OneShotEffect {

        private final String name;

        ProtectedEffect(String name) {
            super(Outcome.Benefit);
            this.name = name;
            staticText = "if you still control " + name + ", you win the game; otherwise you lose it";
        }

        private ProtectedEffect(final ProtectedEffect effect) {
            super(effect);
            this.name = effect.name;
        }

        @Override
        public ProtectedEffect copy() {
            return new ProtectedEffect(this);
        }

        @Override
        public boolean apply(Game game, Ability source) {
            Player player = game.getPlayer(source.getControllerId());
            if (player == null) {
                return false;
            }
            boolean kept = game.getBattlefield().getAllActivePermanents(source.getControllerId()).stream()
                    .anyMatch(permanent -> name.equals(permanent.getName()));
            if (kept) {
                player.won(game);
            } else {
                game.informPlayers(name + " didn't make it.");
                player.lost(game);
            }
            return true;
        }
    }

    private final CareerTwists twists;
    private final Puzzle puzzle;
    /** a table of more than two: the game ends when the player is out */
    private final boolean pod;
    /** the player's own edges (a gauntlet run's relics and wounds), or null */
    private final CareerTwists you;

    private CareerSetup(CareerTwists twists, Puzzle puzzle, boolean pod, CareerTwists you) {
        this.twists = twists;
        this.puzzle = puzzle;
        this.pod = pod;
        this.you = you;
    }

    private CareerSetup(CareerTwists twists, Puzzle puzzle, boolean pod) {
        this(twists, puzzle, pod, null);
    }

    private CareerSetup(CareerTwists twists, Puzzle puzzle) {
        this(twists, puzzle, false);
    }

    /** a pod's setup: the given one (or none) that also ends the game once the player is out */
    public static CareerSetup pod(CareerSetup setup) {
        return setup == null ? new CareerSetup(null, null, true) : new CareerSetup(setup.twists, setup.puzzle, true, setup.you);
    }

    @Override
    public boolean endsWhenHumansAreOut() {
        return pod;
    }

    /** whether twists change nothing */
    static boolean empty(CareerTwists twists) {
        return twists == null || (twists.startingLife == null && twists.lifeBonus == 0 && twists.extraCards <= 0
                && (twists.permanents == null || twists.permanents.isEmpty()) && twists.emblem == null
                && (twists.timed == null || twists.timed.isEmpty()));
    }

    /** null when the twists change nothing */
    public static CareerSetup ofTwists(CareerTwists twists) {
        if (empty(twists)) {
            return null;
        }
        return new CareerSetup(twists, null);
    }

    /** the opponent's twists and the player's own edges; null when neither changes anything */
    public static CareerSetup ofRun(CareerTwists opponent, CareerTwists you) {
        if (empty(opponent) && empty(you)) {
            return null;
        }
        return new CareerSetup(empty(opponent) ? null : opponent, null, false, empty(you) ? null : you);
    }

    /** the player's own edges, or null */
    public CareerTwists yours() {
        return you;
    }

    /** two sets of twists as one: lives and cards add up, permanents and timed twists join, the later emblem wins */
    public static CareerTwists combine(CareerTwists a, CareerTwists b) {
        if (a == null) {
            return b;
        }
        if (b == null) {
            return a;
        }
        CareerTwists both = new CareerTwists();
        both.startingLife = b.startingLife != null ? b.startingLife : a.startingLife;
        both.lifeBonus = a.lifeBonus + b.lifeBonus;
        both.extraCards = a.extraCards + b.extraCards;
        if (a.permanents != null || b.permanents != null) {
            both.permanents = new ArrayList<>();
            if (a.permanents != null) {
                both.permanents.addAll(a.permanents);
            }
            if (b.permanents != null) {
                both.permanents.addAll(b.permanents);
            }
        }
        both.emblem = b.emblem != null ? b.emblem : a.emblem;
        if (a.timed != null || b.timed != null) {
            both.timed = new ArrayList<>();
            if (a.timed != null) {
                both.timed.addAll(a.timed);
            }
            if (b.timed != null) {
                both.timed.addAll(b.timed);
            }
        }
        return both;
    }

    public static CareerSetup ofPuzzle(Puzzle puzzle) {
        return new CareerSetup(null, puzzle);
    }

    @Override
    public boolean replacesOpeningHands() {
        return puzzle != null;
    }

    @Override
    public UUID startingPlayer(Game game) {
        if (puzzle == null) {
            return null;
        }
        UUID humanId = human(game);
        if (puzzle.opponentFirst()) {
            // the opponent takes the turn the player has to live through
            for (Player player : game.getPlayers().values()) {
                if (!player.getId().equals(humanId)) {
                    return player.getId();
                }
            }
        }
        return humanId;
    }

    private static UUID human(Game game) {
        for (Player player : game.getPlayers().values()) {
            if (player.isHuman()) {
                return player.getId();
            }
        }
        return null;
    }

    @Override
    public void apply(Game game, UUID startingPlayerId) {
        UUID humanId = human(game);
        for (Player player : game.getPlayers().values()) {
            if (player.getId().equals(humanId)) {
                if (puzzle != null) {
                    side(game, player, puzzle.you);
                } else if (you != null) {
                    edge(game, player, you, " starts with");
                    if (you.timed != null && !you.timed.isEmpty()) {
                        CareerTimedTwists.add(game, player, humanId, you.timed);
                    }
                }
            } else if (puzzle != null) {
                side(game, player, puzzle.opponent);
            } else if (twists != null) {
                edge(game, player, twists, " starts with an edge");
                if (twists.timed != null && !twists.timed.isEmpty()) {
                    CareerTimedTwists.add(game, player, humanId, twists.timed);
                }
            }
        }
        if (puzzle != null && humanId != null && puzzle.protecting()) {
            Ability source = fakeSource(humanId);
            game.addDelayedTriggeredAbility(new LaterUpkeepTrigger(new ProtectedEffect(puzzle.protect)), source);
            game.informPlayers("Trial: " + (puzzle.name == null ? "" : puzzle.name + ". ")
                    + (puzzle.text == null ? "Keep " + puzzle.protect + " until your next turn." : puzzle.text));
        } else if (puzzle != null && humanId != null && puzzle.survive()) {
            // alive when your turn comes round: you win
            Ability source = fakeSource(humanId);
            game.addDelayedTriggeredAbility(new LaterUpkeepTrigger(new WinGameSourceControllerEffect()), source);
            game.informPlayers("Trial: " + (puzzle.name == null ? "" : puzzle.name + ". ")
                    + (puzzle.text == null ? "Survive their turn." : puzzle.text));
        } else if (puzzle != null && humanId != null) {
            // win this turn, or lose at its end
            Ability source = fakeSource(humanId);
            game.addDelayedTriggeredAbility(new AtTheBeginOfNextEndStepDelayedTriggeredAbility(
                    new LoseGameTargetPlayerEffect().setTargetPointer(new FixedTarget(humanId)), TargetController.ANY), source);
            game.informPlayers("Puzzle: " + (puzzle.name == null ? "" : puzzle.name + ". ")
                    + (puzzle.text == null ? "Win this turn." : puzzle.text));
        }
        game.applyEffects();
    }

    private static Ability fakeSource(UUID controllerId) {
        Ability source = new SimpleStaticAbility(Zone.OUTSIDE, new InfoEffect("Career"));
        source.setControllerId(controllerId);
        return source;
    }

    /** gives a player an edge: life set, cards drawn, permanents in play, an emblem; announced as {@code what} */
    static void edge(Game game, Player player, CareerTwists twists, String what) {
        if (twists.startingLife != null) {
            player.setLife(twists.startingLife, game, null);
        }
        if (twists.lifeBonus != 0) {
            player.setLife(Math.max(1, player.getLife() + twists.lifeBonus), game, null);
        }
        if (twists.extraCards > 0) {
            player.drawCards(twists.extraCards, null, game);
        }
        if (twists.permanents != null && !twists.permanents.isEmpty()) {
            List<PutToBattlefieldInfo> battlefield = new ArrayList<>();
            for (String entry : twists.permanents) {
                Card card = create(entry);
                if (card != null) {
                    battlefield.add(new PutToBattlefieldInfo(card, entry.endsWith("|tapped")));
                }
            }
            game.cheat(player.getId(), Collections.emptyList(), Collections.emptyList(), battlefield,
                    Collections.emptyList(), Collections.emptyList(), Collections.emptyList());
        }
        if (twists.emblem != null) {
            Card card = create(twists.emblem);
            if (card != null) {
                EmblemOfCard emblem = new EmblemOfCard(card);
                game.addEmblem(emblem, card, player.getId());
                for (Ability ability : emblem.getAbilities()) {
                    game.getState().addAbility(ability, null, emblem);
                }
            }
        }
        String described = describe(twists);
        if (!described.isEmpty()) {
            game.informPlayers(player.getLogName() + what + described + ".");
        }
    }

    private static String describe(CareerTwists twists) {
        List<String> parts = new ArrayList<>();
        if (twists.startingLife != null) {
            parts.add(twists.startingLife + " life");
        }
        if (twists.lifeBonus != 0) {
            parts.add((twists.lifeBonus > 0 ? "+" : "") + twists.lifeBonus + " life");
        }
        if (twists.extraCards > 0) {
            parts.add(twists.extraCards + (twists.extraCards == 1 ? " extra card" : " extra cards"));
        }
        if (twists.permanents != null) {
            for (String permanent : twists.permanents) {
                parts.add(name(permanent));
            }
        }
        if (twists.emblem != null) {
            parts.add("an emblem of " + twists.emblem);
        }
        return parts.isEmpty() ? "" : ": " + String.join(", ", parts);
    }

    private static void side(Game game, Player player, CareerPuzzleSide side) {
        if (side == null) {
            return;
        }
        player.getHand().clear();
        player.getLibrary().clear();
        if (side.life != null) {
            player.setLife(side.life, game, null);
        }
        List<Card> library = cards(side.library);
        // cheat puts each library card on top, so the last one put ends up first
        Collections.reverse(library);
        List<PutToBattlefieldInfo> battlefield = new ArrayList<>();
        if (side.battlefield != null) {
            for (String entry : side.battlefield) {
                Card card = create(entry);
                if (card != null) {
                    battlefield.add(new PutToBattlefieldInfo(card, entry.endsWith("|tapped")));
                }
            }
        }
        game.cheat(player.getId(), library, cards(side.hand), battlefield, cards(side.graveyard),
                Collections.emptyList(), Collections.emptyList());
    }

    private static List<Card> cards(List<String> names) {
        List<Card> cards = new ArrayList<>();
        if (names != null) {
            for (String name : names) {
                Card card = create(name);
                if (card != null) {
                    cards.add(card);
                }
            }
        }
        return cards;
    }

    static String name(String entry) {
        int bar = entry.indexOf('|');
        return (bar < 0 ? entry : entry.substring(0, bar)).trim();
    }

    private static Card create(String entry) {
        CardInfo info = CardRepository.instance.findPreferredCoreExpansionCard(name(entry));
        if (info == null) {
            logger.warn("Career setup: no card " + name(entry));
            return null;
        }
        return info.createCard();
    }

    public CareerTwists twists() {
        return twists;
    }

    public Puzzle puzzle() {
        return puzzle;
    }
}
