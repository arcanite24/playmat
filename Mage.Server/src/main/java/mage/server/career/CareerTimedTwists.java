package mage.server.career;

import mage.abilities.Ability;
import mage.abilities.StateTriggeredAbility;
import mage.abilities.TriggeredAbilityImpl;
import mage.abilities.effects.OneShotEffect;
import mage.constants.Outcome;
import mage.constants.Zone;
import mage.game.Game;
import mage.game.command.Emblem;
import mage.game.events.GameEvent;
import mage.game.stack.Spell;
import mage.players.Player;

import java.util.List;
import java.util.Locale;
import java.util.UUID;

/**
 * Twists that come in during a game (S008): at a turn, when a player's life falls to a threshold, or when a player
 * casts a named card, the opponent gets an edge (cards, permanents, an emblem, life). Each fires once. They ride on
 * an emblem the opponent holds, as triggered abilities, so they're part of the game state: they copy with the game,
 * save with it and show in its log.
 */
final class CareerTimedTwists {

    private CareerTimedTwists() {
    }

    /** gives the opponent an emblem holding their timed twists */
    static void add(Game game, Player opponent, UUID humanId, List<CareerSetup.TimedTwist> twists) {
        TwistsEmblem emblem = new TwistsEmblem();
        int n = 0;
        for (CareerSetup.TimedTwist twist : twists) {
            String problem = twist.problem();
            if (problem != null) {
                CareerSetup.logger.warn("Career: timed twist skipped: " + problem);
                continue;
            }
            n++;
            UUID whoId = "you".equals(who(twist)) ? humanId : opponent.getId();
            switch (twist.when) {
                case CareerSetup.TimedTwist.TURN:
                    emblem.getAbilities().add(new AtTurn(twist));
                    break;
                case CareerSetup.TimedTwist.LIFE:
                    emblem.getAbilities().add(new AtLife(twist, whoId));
                    break;
                default:
                    emblem.getAbilities().add(new OnCast(twist, whoId));
            }
        }
        if (n > 0) {
            game.addEmblem(emblem, null, opponent.getId());
        }
    }

    static String who(CareerSetup.TimedTwist twist) {
        if (twist.who != null) {
            return twist.who.toLowerCase(Locale.ENGLISH);
        }
        // a life threshold watches the opponent's life by default; a cast watches the player
        return CareerSetup.TimedTwist.LIFE.equals(twist.when) ? "opponent" : "you";
    }

    public static final class TwistsEmblem extends Emblem {

        public TwistsEmblem() {
            super("Emblem Twist");
        }

        private TwistsEmblem(final TwistsEmblem emblem) {
            super(emblem);
        }

        @Override
        public void setSourceObjectAndInitImage(mage.MageObject sourceObject) {
            // no printed emblem looks like this one: skip the token image lookup (it throws for unknown emblems)
            this.sourceObject = sourceObject;
        }

        @Override
        public TwistsEmblem copy() {
            return new TwistsEmblem(this);
        }
    }

    /** at the beginning of turn N's upkeep */
    static final class AtTurn extends TriggeredAbilityImpl {
        private final int turn;

        AtTurn(CareerSetup.TimedTwist twist) {
            super(Zone.COMMAND, new TwistEffect(twist));
            this.turn = twist.turn;
            setTriggersLimitEachGame(1);
            setTriggerPhrase("At the beginning of turn " + turn + ", ");
        }

        private AtTurn(final AtTurn ability) {
            super(ability);
            this.turn = ability.turn;
        }

        @Override
        public boolean checkEventType(GameEvent event, Game game) {
            return event.getType() == GameEvent.EventType.UPKEEP_STEP_PRE;
        }

        @Override
        public boolean checkTrigger(GameEvent event, Game game) {
            return game.getTurnNum() >= turn;
        }

        @Override
        public AtTurn copy() {
            return new AtTurn(this);
        }
    }

    /** when a player's life is at or below N */
    static final class AtLife extends StateTriggeredAbility {
        private final int life;
        private final UUID playerId;

        AtLife(CareerSetup.TimedTwist twist, UUID playerId) {
            super(Zone.COMMAND, new TwistEffect(twist));
            this.life = twist.life;
            this.playerId = playerId;
            setTriggersLimitEachGame(1);
            setTriggerPhrase("When " + ("you".equals(who(twist)) ? "your opponent's" : "this player's") + " life is " + life + " or less, ");
        }

        private AtLife(final AtLife ability) {
            super(ability);
            this.life = ability.life;
            this.playerId = ability.playerId;
        }

        @Override
        public boolean checkTrigger(GameEvent event, Game game) {
            Player player = game.getPlayer(playerId);
            return player != null && player.isInGame() && player.getLife() <= life;
        }

        @Override
        public AtLife copy() {
            return new AtLife(this);
        }
    }

    /** when a player casts a card with this name */
    static final class OnCast extends TriggeredAbilityImpl {
        private final String card;
        private final UUID playerId;

        OnCast(CareerSetup.TimedTwist twist, UUID playerId) {
            super(Zone.COMMAND, new TwistEffect(twist));
            this.card = twist.card;
            this.playerId = playerId;
            setTriggersLimitEachGame(1);
            setTriggerPhrase("When " + ("you".equals(who(twist)) ? "your opponent casts " : "this player casts ") + card + ", ");
        }

        private OnCast(final OnCast ability) {
            super(ability);
            this.card = ability.card;
            this.playerId = ability.playerId;
        }

        @Override
        public boolean checkEventType(GameEvent event, Game game) {
            return event.getType() == GameEvent.EventType.SPELL_CAST;
        }

        @Override
        public boolean checkTrigger(GameEvent event, Game game) {
            if (!event.getPlayerId().equals(playerId)) {
                return false;
            }
            Spell spell = game.getSpellOrLKIStack(event.getTargetId());
            return spell != null && spell.getName().equalsIgnoreCase(card);
        }

        @Override
        public OnCast copy() {
            return new OnCast(this);
        }
    }

    /** the edge itself: the same as a starting twist, plus life and a line in the log */
    static final class TwistEffect extends OneShotEffect {
        private final CareerSetup.TimedTwist twist;

        TwistEffect(CareerSetup.TimedTwist twist) {
            super(Outcome.Benefit);
            this.twist = twist;
            this.staticText = twist.message != null ? twist.message : "the opponent gets an edge";
        }

        private TwistEffect(final TwistEffect effect) {
            super(effect);
            this.twist = effect.twist;
        }

        @Override
        public boolean apply(Game game, Ability source) {
            Player opponent = game.getPlayer(source.getControllerId());
            if (opponent == null) {
                return false;
            }
            if (twist.message != null) {
                game.informPlayers(twist.message);
            }
            if (twist.then != null) {
                CareerSetup.edge(game, opponent, twist.then, " gets an edge");
            }
            if (twist.gainLife != 0) {
                if (twist.gainLife > 0) {
                    opponent.gainLife(twist.gainLife, game, source);
                } else {
                    opponent.loseLife(-twist.gainLife, game, source, false);
                }
            }
            return true;
        }

        @Override
        public TwistEffect copy() {
            return new TwistEffect(this);
        }
    }
}
