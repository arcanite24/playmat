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
    /** a horde game (S036): the opponent's deck plays itself */
    private HordeRules horde;
    /** an Archenemy game (Phase 5): either the player or the boss flips a Scheme every upkeep */
    private ArchenemyRules archenemy;
    /** a Two-Headed Giant solo match (Phase 5.4): You + AI Partner vs 2 AI */
    private TwoHeadedGiantSpec thg;
    /** how the opponent AI leans (S042), or null for its plain self */
    private Personality personality;

    /** a setup that only carries a horde, archenemy, two-headed giant or a personality */
    public static CareerSetup plain() {
        return new CareerSetup(null, null);
    }

    public CareerSetup withHorde(HordeRules horde) {
        this.horde = horde;
        return this;
    }

    public HordeRules horde() {
        return horde;
    }

    public CareerSetup withArchenemy(ArchenemyRules archenemy) {
        this.archenemy = archenemy;
        return this;
    }

    public ArchenemyRules archenemy() {
        return archenemy;
    }

    public CareerSetup withTwoHeadedGiant(TwoHeadedGiantSpec thg) {
        this.thg = thg;
        return this;
    }

    public static CareerSetup twoHeadedGiant(TwoHeadedGiantSpec thg, Personality personality) {
        CareerSetup setup = pod(plain()).withTwoHeadedGiant(thg);
        return withPersonality(setup, personality);
    }

    /** the setup (or a plain one) with the opponent's personality; null personality leaves it as it was */
    public static CareerSetup withPersonality(CareerSetup setup, Personality personality) {
        if (personality == null || personality.neutral()) {
            return setup;
        }
        CareerSetup with = setup == null ? plain() : setup;
        with.personality = personality;
        return with;
    }

    public Personality personality() {
        return personality;
    }

    /**
     * An opponent's leanings (S042), each -100 to 100 with 0 the AI's plain self: aggression (attacks into worse
     * trades), risk (blocks and attacks that can lose a creature) and patience (holds back to build a bigger board).
     * The game keeps them as a state value the AI reads: {@value #PERSONALITY_KEY} and the player's id.
     */
    public static class Personality implements Serializable {
        private static final long serialVersionUID = 1L;
        public int aggression;
        public int risk;
        public int patience;

        boolean neutral() {
            return aggression == 0 && risk == 0 && patience == 0;
        }
    }

    /** the game state key prefix the AI reads its personality from (ComputerPlayer6) */
    public static final String PERSONALITY_KEY = "ai.personality.";

    /** A horde (S036 / 2.0): what the player must live through (survive <= 0 means Endless Waves). */
    public static class HordeRules implements Serializable {
        private static final long serialVersionUID = 1L;
        public String name;
        /** the horde's turns to live through (0 or negative = Endless Waves) */
        public int survive = 8;
        /** cards the horde flips on its first turn */
        public int flip = 1;
        /** one more card flipped every this many horde turns */
        public int rampEvery = 2;
        /** the horde's life; null for 20 */
        public Integer life;
        /** optional player starting life override (e.g. 40 for Commander Horde) */
        public Integer playerLife;
    }

    /** A single Archenemy Scheme set in motion on the Archenemy's upkeep. */
    public static class SchemeSpec implements Serializable {
        private static final long serialVersionUID = 1L;
        public String id;
        public String name;
        public String text;
        public int damageOpponents;
        public int lifeLossOpponents;
        public int gainLife;
        public int drawCards;
        public int opponentsDiscard;
        public List<String> permanents = new ArrayList<>();
        public String emblem;
    }

    /** Archenemy rules: who holds the Scheme deck (player or boss), starting life, and randomized Scheme list. */
    public static class ArchenemyRules implements Serializable {
        private static final long serialVersionUID = 1L;
        public String name;
        /** true = "Become the Archenemy" (human vs 3 AI); false = "Hunt the Archenemy" (boss flips schemes) */
        public boolean playerIsArchenemy;
        public int archenemyLife = 60;
        public int challengerLife = 20;
        public List<SchemeSpec> schemes = new ArrayList<>();
    }

    /** Two-Headed Giant solo rules (Phase 5.4): team tags so allies never attack each other, shared win condition. */
    public static class TwoHeadedGiantSpec implements Serializable {
        private static final long serialVersionUID = 1L;
        public String name;
        public String partnerName;
        public int teamLife = 30;
        public boolean commander;
    }

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
        if (horde != null && humanId != null) {
            if (horde.playerLife != null) {
                Player humanPlayer = game.getPlayer(humanId);
                if (humanPlayer != null) {
                    humanPlayer.setLife(horde.playerLife, game, null);
                }
            }
            for (Player player : game.getPlayers().values()) {
                if (!player.getId().equals(humanId)) {
                    if (horde.life != null) {
                        player.setLife(horde.life, game, null);
                    }
                    player.getHand().clear();
                    game.addEmblem(new HordeEmblem(horde), null, player.getId());
                    String goal = horde.survive > 0
                            ? "Live through " + horde.survive + " of its turns."
                            : "Endless Waves: survive as many waves as you can.";
                    game.informPlayers("Horde: " + (horde.name == null ? "the horde" : horde.name) + " flips its cards onto the battlefield "
                            + "each turn, and its creatures have haste and attack every combat. " + goal);
                }
            }
        }
        if (archenemy != null && humanId != null && archenemy.schemes != null && !archenemy.schemes.isEmpty()) {
            UUID holderId = null;
            for (Player player : game.getPlayers().values()) {
                boolean isHolder = archenemy.playerIsArchenemy
                        ? player.getId().equals(humanId)
                        : (!player.getId().equals(humanId) && holderId == null);
                if (isHolder) {
                    holderId = player.getId();
                    player.setLife(archenemy.archenemyLife, game, null);
                    game.addEmblem(new ArchenemyEmblem(archenemy), null, player.getId());
                } else {
                    player.setLife(archenemy.challengerLife, game, null);
                }
            }
            game.informPlayers("Archenemy: " + (archenemy.name == null ? "The Scheme Deck" : archenemy.name)
                    + " is active. A Scheme is set in motion at the start of the Archenemy's upkeep.");
        }
        if (thg != null && humanId != null) {
            UUID partnerId = null;
            List<String> enemyNames = new ArrayList<>();
            for (Player player : game.getPlayers().values()) {
                player.setLife(thg.teamLife, game, null);
                if (player.getId().equals(humanId)) {
                    game.getState().setValue("career.thg.team." + player.getId(), "allies");
                } else if (partnerId == null && (thg.partnerName == null || thg.partnerName.equals(player.getName()))) {
                    partnerId = player.getId();
                    game.getState().setValue("career.thg.team." + player.getId(), "allies");
                } else {
                    game.getState().setValue("career.thg.team." + player.getId(), "enemies");
                    enemyNames.add(player.getName());
                }
            }
            game.addEmblem(new TwoHeadedGiantEmblem(thg.partnerName != null ? thg.partnerName : "AI Partner"), null, humanId);
            game.informPlayers("Two-Headed Giant: You and " + (thg.partnerName != null ? thg.partnerName : "your AI partner")
                    + " battle " + String.join(" & ", enemyNames) + " (" + thg.teamLife + " life each).");
        }
        if (personality != null && humanId != null) {
            for (Player player : game.getPlayers().values()) {
                if (!player.getId().equals(humanId)) {
                    java.util.Map<String, Integer> leanings = new java.util.HashMap<>();
                    leanings.put("aggression", personality.aggression);
                    leanings.put("risk", personality.risk);
                    leanings.put("patience", personality.patience);
                    game.getState().setValue(PERSONALITY_KEY + player.getId(), leanings);
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

    /** Two-Headed Giant team emblem: grants victory immediately once both opposing heads fall. */
    public static class TwoHeadedGiantEmblem extends mage.game.command.Emblem {

        public TwoHeadedGiantEmblem(String partnerName) {
            super("Two-Headed Giant (" + partnerName + ")");
            this.getAbilities().add(new mage.abilities.triggers.BeginningOfUpkeepTriggeredAbility(
                    Zone.COMMAND, TargetController.ANY, new TwoHeadedGiantCheckEffect(), false));
        }

        private TwoHeadedGiantEmblem(final TwoHeadedGiantEmblem emblem) {
            super(emblem);
        }

        @Override
        public void setSourceObjectAndInitImage(mage.MageObject sourceObject) {
            this.sourceObject = sourceObject;
        }

        @Override
        public TwoHeadedGiantEmblem copy() {
            return new TwoHeadedGiantEmblem(this);
        }
    }

    static class TwoHeadedGiantCheckEffect extends OneShotEffect {

        TwoHeadedGiantCheckEffect() {
            super(Outcome.Benefit);
            staticText = "When both opposing heads are eliminated, your team wins the game.";
        }

        private TwoHeadedGiantCheckEffect(final TwoHeadedGiantCheckEffect effect) {
            super(effect);
        }

        @Override
        public TwoHeadedGiantCheckEffect copy() {
            return new TwoHeadedGiantCheckEffect(this);
        }

        @Override
        public boolean apply(Game game, Ability source) {
            Player human = game.getPlayer(source.getControllerId());
            if (human == null || !human.isInGame()) {
                return false;
            }
            boolean enemyAlive = false;
            for (Player player : game.getPlayers().values()) {
                if ("enemies".equals(game.getState().getValue("career.thg.team." + player.getId())) && player.isInGame()) {
                    enemyAlive = true;
                    break;
                }
            }
            if (!enemyAlive) {
                game.informPlayers("Two-Headed Giant: Both opposing heads have fallen — your team wins!");
                human.won(game);
            }
            return true;
        }
    }

    /** the horde's engine: flip cards each upkeep; creatures with haste that attack each combat */
    public static class HordeEmblem extends mage.game.command.Emblem {

        public HordeEmblem(HordeRules rules) {
            super("Horde");
            this.getAbilities().add(new mage.abilities.triggers.BeginningOfUpkeepTriggeredAbility(Zone.COMMAND, TargetController.YOU,
                    new HordeFlipEffect(rules.survive, rules.flip, rules.rampEvery), false));
            this.getAbilities().add(new SimpleStaticAbility(Zone.COMMAND, new mage.abilities.effects.common.continuous.GainAbilityControlledEffect(
                    mage.abilities.keyword.HasteAbility.getInstance(), mage.constants.Duration.EndOfGame,
                    mage.filter.StaticFilters.FILTER_PERMANENT_CREATURES)));
            this.getAbilities().add(new SimpleStaticAbility(Zone.COMMAND, new mage.abilities.effects.common.continuous.GainAbilityControlledEffect(
                    new mage.abilities.common.AttacksEachCombatStaticAbility(), mage.constants.Duration.EndOfGame,
                    mage.filter.StaticFilters.FILTER_PERMANENT_CREATURES)));
        }

        private HordeEmblem(final HordeEmblem emblem) {
            super(emblem);
        }

        @Override
        public void setSourceObjectAndInitImage(mage.MageObject sourceObject) {
            // no printed emblem looks like this one: skip the token image lookup
            this.sourceObject = sourceObject;
        }

        @Override
        public HordeEmblem copy() {
            return new HordeEmblem(this);
        }
    }

    /** the horde's upkeep: count the turn (past the last, the horde is beaten), then flip and put its permanents in play */
    static class HordeFlipEffect extends OneShotEffect {
        static final String TURNS = "career.horde.turns";
        private final int survive;
        private final int flip;
        private final int rampEvery;

        HordeFlipEffect(int survive, int flip, int rampEvery) {
            super(Outcome.PutCardInPlay);
            this.survive = survive;
            this.flip = flip;
            this.rampEvery = Math.max(1, rampEvery);
            staticText = "flip the horde's cards";
        }

        private HordeFlipEffect(final HordeFlipEffect effect) {
            super(effect);
            this.survive = effect.survive;
            this.flip = effect.flip;
            this.rampEvery = effect.rampEvery;
        }

        @Override
        public HordeFlipEffect copy() {
            return new HordeFlipEffect(this);
        }

        @Override
        public boolean apply(Game game, Ability source) {
            Player horde = game.getPlayer(source.getControllerId());
            if (horde == null) {
                return false;
            }
            Object counted = game.getState().getValue(TURNS);
            int turn = (counted instanceof Integer ? (Integer) counted : 0) + 1;
            game.getState().setValue(TURNS, turn);
            if (survive > 0 && turn > survive) {
                game.informPlayers("The horde is spent: you lived through " + survive + " of its turns.");
                horde.lost(game);
                return true;
            }
            int count = flip + (turn - 1) / rampEvery;
            if (survive <= 0 && horde.getLibrary().size() < count && !horde.getGraveyard().isEmpty()) {
                horde.moveCards(horde.getGraveyard(), Zone.LIBRARY, source, game);
                horde.shuffleLibrary(source, game);
                game.informPlayers("Endless Horde: wave " + turn + " reshuffles the fallen back into the tide.");
            }
            mage.cards.Cards cards = new mage.cards.CardsImpl(horde.getLibrary().getTopCards(game, count));
            if (cards.isEmpty()) {
                return true;
            }
            horde.revealCards(source, "Horde turn " + turn, cards, game);
            java.util.Set<Card> permanents = new java.util.LinkedHashSet<>();
            java.util.Set<Card> rest = new java.util.LinkedHashSet<>();
            for (Card card : cards.getCards(game)) {
                (card.isPermanent(game) ? permanents : rest).add(card);
            }
            horde.moveCards(permanents, Zone.BATTLEFIELD, source, game);
            horde.moveCards(rest, Zone.GRAVEYARD, source, game);
            return true;
        }
    }

    /** Archenemy engine: sets the next Scheme in motion on the Archenemy's upkeep. */
    public static class ArchenemyEmblem extends mage.game.command.Emblem {

        public ArchenemyEmblem(ArchenemyRules rules) {
            super(rules.name == null ? "Archenemy Scheme Deck" : rules.name);
            this.getAbilities().add(new mage.abilities.triggers.BeginningOfUpkeepTriggeredAbility(
                    Zone.COMMAND, TargetController.YOU, new SchemeTriggerEffect(rules.schemes), false));
        }

        private ArchenemyEmblem(final ArchenemyEmblem emblem) {
            super(emblem);
        }

        @Override
        public void setSourceObjectAndInitImage(mage.MageObject sourceObject) {
            this.sourceObject = sourceObject;
        }

        @Override
        public ArchenemyEmblem copy() {
            return new ArchenemyEmblem(this);
        }
    }

    static class SchemeTriggerEffect extends OneShotEffect {
        static final String INDEX_KEY = "career.archenemy.index";
        private final List<SchemeSpec> schemes;

        SchemeTriggerEffect(List<SchemeSpec> schemes) {
            super(Outcome.Benefit);
            this.schemes = new ArrayList<>(schemes);
            staticText = "set the next Scheme in motion";
        }

        private SchemeTriggerEffect(final SchemeTriggerEffect effect) {
            super(effect);
            this.schemes = new ArrayList<>(effect.schemes);
        }

        @Override
        public SchemeTriggerEffect copy() {
            return new SchemeTriggerEffect(this);
        }

        @Override
        public boolean apply(Game game, Ability source) {
            Player holder = game.getPlayer(source.getControllerId());
            if (holder == null || schemes.isEmpty()) {
                return false;
            }
            Object raw = game.getState().getValue(INDEX_KEY);
            int idx = raw instanceof Integer ? (Integer) raw : 0;
            SchemeSpec scheme = schemes.get(idx % schemes.size());
            game.getState().setValue(INDEX_KEY, idx + 1);
            game.informPlayers("Scheme in motion — " + scheme.name + ": " + scheme.text);

            if (scheme.gainLife > 0) {
                holder.gainLife(scheme.gainLife, game, source);
            }
            if (scheme.drawCards > 0) {
                holder.drawCards(scheme.drawCards, source, game);
            }
            for (UUID oppId : game.getOpponents(holder.getId())) {
                Player opp = game.getPlayer(oppId);
                if (opp == null) {
                    continue;
                }
                if (scheme.damageOpponents > 0) {
                    opp.damage(scheme.damageOpponents, source.getSourceId(), source, game);
                }
                if (scheme.lifeLossOpponents > 0) {
                    opp.loseLife(scheme.lifeLossOpponents, game, source, false);
                }
                if (scheme.opponentsDiscard > 0) {
                    opp.discard(scheme.opponentsDiscard, false, false, source, game);
                }
            }
            if (scheme.permanents != null && !scheme.permanents.isEmpty()) {
                List<PutToBattlefieldInfo> battlefield = new ArrayList<>();
                for (String entry : scheme.permanents) {
                    Card card = create(entry);
                    if (card != null) {
                        battlefield.add(new PutToBattlefieldInfo(card, entry.endsWith("|tapped")));
                    }
                }
                if (!battlefield.isEmpty()) {
                    game.cheat(holder.getId(), Collections.emptyList(), Collections.emptyList(), battlefield,
                            Collections.emptyList(), Collections.emptyList(), Collections.emptyList());
                }
            }
            if (scheme.emblem != null && !scheme.emblem.isEmpty()) {
                Card emblemCard = create(scheme.emblem);
                if (emblemCard != null) {
                    mage.game.command.Emblem emblem = new EmblemOfCard(emblemCard);
                    game.addEmblem(emblem, emblemCard, holder.getId());
                    for (Ability ability : emblem.getAbilities()) {
                        game.getState().addAbility(ability, null, emblem);
                    }
                }
            }
            return true;
        }
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

    /**
     * The emblems the Rust host plays (an {@code EmblemOfCard} with the card's static abilities, checked there by
     * playmat-host's tests): another card's emblem keeps the game on Java.
     */
    static final java.util.Set<String> HOST_EMBLEMS = java.util.Collections.unmodifiableSet(
            new java.util.HashSet<>(Collections.singletonList("Glorious Anthem")));

    /**
     * What keeps the Rust host from playing this setup (docs/rust/GameHost.md, Career tables), or null when it plays
     * it: life, extra cards, permanents, an emblem of {@link #HOST_EMBLEMS} and timed twists for either side, and the
     * opponent's personality.
     */
    public String hostProblem() {
        if (puzzle != null) {
            return "a Career puzzle or trial";
        }
        if (horde != null) {
            return "a horde";
        }
        if (pod) {
            return "a pod";
        }
        for (CareerTwists t : new CareerTwists[]{twists, you}) {
            if (t == null) {
                continue;
            }
            String emblem = hostEmblemProblem(t);
            if (emblem != null) {
                return emblem;
            }
            for (TimedTwist timed : timed(t)) {
                emblem = timed.then == null ? null : hostEmblemProblem(timed.then);
                if (emblem != null) {
                    return emblem;
                }
            }
        }
        return null;
    }

    private static String hostEmblemProblem(CareerTwists t) {
        if (t.emblem != null && !HOST_EMBLEMS.contains(t.emblem)) {
            return "a Career emblem the host lacks (" + t.emblem + ")";
        }
        return null;
    }

    /** the timed twists that play: Java skips one that isn't well formed ({@code CareerTimedTwists.add}) */
    private static List<TimedTwist> timed(CareerTwists t) {
        List<TimedTwist> list = new ArrayList<>();
        if (t.timed != null) {
            for (TimedTwist timed : t.timed) {
                if (timed != null && timed.problem() == null) {
                    list.add(timed);
                }
            }
        }
        return list;
    }

    /** the cards the setup puts in play or makes emblems of, then or later: the engine must have verified them too */
    public java.util.List<String> hostCards() {
        java.util.List<String> names = new ArrayList<>();
        for (CareerTwists t : new CareerTwists[]{twists, you}) {
            if (t != null) {
                cards(t, names);
                for (TimedTwist timed : timed(t)) {
                    if (timed.then != null) {
                        cards(timed.then, names);
                    }
                }
            }
        }
        return names;
    }

    private static void cards(CareerTwists t, List<String> names) {
        if (t.permanents != null) {
            t.permanents.forEach(entry -> names.add(name(entry)));
        }
        if (t.emblem != null) {
            names.add(t.emblem);
        }
    }

    /**
     * The edges as the host plays them ({@code apply} after the mulligans): {@code you} for the human, {@code opponent}
     * for the other seat, each with its life, life bonus, extra cards, permanents, emblem, the log line's text after
     * the player's name and its timed twists; and the opponent's {@code personality}.
     */
    public com.google.gson.JsonObject hostSetup() {
        com.google.gson.JsonObject json = new com.google.gson.JsonObject();
        if (you != null) {
            json.add("you", hostEdge(you, " starts with"));
        }
        if (twists != null) {
            json.add("opponent", hostEdge(twists, " starts with an edge"));
        }
        if (personality != null) {
            com.google.gson.JsonObject p = new com.google.gson.JsonObject();
            p.addProperty("aggression", personality.aggression);
            p.addProperty("risk", personality.risk);
            p.addProperty("patience", personality.patience);
            json.add("personality", p);
        }
        return json;
    }

    private static com.google.gson.JsonObject hostEdge(CareerTwists t, String what) {
        com.google.gson.JsonObject edge = new com.google.gson.JsonObject();
        if (t.startingLife != null) {
            edge.addProperty("life", t.startingLife);
        }
        edge.addProperty("lifeBonus", t.lifeBonus);
        edge.addProperty("cards", Math.max(0, t.extraCards));
        com.google.gson.JsonArray permanents = new com.google.gson.JsonArray();
        if (t.permanents != null) {
            for (String entry : t.permanents) {
                com.google.gson.JsonObject card = new com.google.gson.JsonObject();
                card.addProperty("name", name(entry));
                card.addProperty("tapped", entry.endsWith("|tapped"));
                permanents.add(card);
            }
        }
        edge.add("permanents", permanents);
        if (t.emblem != null) {
            edge.addProperty("emblem", t.emblem);
        }
        String described = describe(t);
        if (!described.isEmpty()) {
            edge.addProperty("log", what + described + ".");
        }
        List<TimedTwist> timed = timed(t);
        if (!timed.isEmpty()) {
            com.google.gson.JsonArray list = new com.google.gson.JsonArray();
            timed.forEach(twist -> list.add(hostTimed(twist)));
            edge.add("timed", list);
        }
        return edge;
    }

    /**
     * A timed twist as the host plays it: when it triggers, whose life or spells it watches ({@code who}: "you", the
     * human, or "opponent", the twist's holder), its rule text as the emblem shows it, and its message, life and edge
     */
    private static com.google.gson.JsonObject hostTimed(TimedTwist twist) {
        com.google.gson.JsonObject json = new com.google.gson.JsonObject();
        json.addProperty("when", twist.when);
        Ability ability;
        switch (twist.when) {
            case TimedTwist.TURN:
                json.addProperty("turn", twist.turn);
                ability = new CareerTimedTwists.AtTurn(twist);
                break;
            case TimedTwist.LIFE:
                json.addProperty("life", twist.life);
                ability = new CareerTimedTwists.AtLife(twist, null);
                break;
            default:
                json.addProperty("card", twist.card);
                ability = new CareerTimedTwists.OnCast(twist, null);
        }
        json.addProperty("who", CareerTimedTwists.who(twist));
        json.addProperty("rule", ability.getRule());
        if (twist.message != null) {
            json.addProperty("message", twist.message);
        }
        json.addProperty("gainLife", twist.gainLife);
        if (twist.then != null) {
            json.add("then", hostEdge(twist.then, " gets an edge"));
        }
        return json;
    }

    public CareerTwists twists() {
        return twists;
    }

    public Puzzle puzzle() {
        return puzzle;
    }
}
