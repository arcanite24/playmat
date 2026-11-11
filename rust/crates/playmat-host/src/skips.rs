//! A human's priority stops and "pass until…" keys, as `HumanPlayer.priority` applies them: the engine asks the
//! human at every priority (`Player::every_priority`) and the host answers the ones the player's settings skip.

use playmat_engine::state::{Game, Seat, Step};
use serde_json::Value;

/// `SkipPrioritySteps`: the steps a player stops in, on one side of the turns
#[derive(Clone, Debug)]
pub struct Stops {
    upkeep: bool,
    draw: bool,
    main1: bool,
    before_combat: bool,
    end_of_combat: bool,
    main2: bool,
    end_of_turn: bool,
}

impl Default for Stops {
    fn default() -> Stops {
        Stops {
            upkeep: false,
            draw: false,
            main1: true,
            before_combat: false,
            end_of_combat: false,
            main2: true,
            end_of_turn: false,
        }
    }
}

impl Stops {
    fn parse(v: Option<&Value>) -> Stops {
        let d = Stops::default();
        let Some(v) = v else { return d };
        let b = |k: &str, d: bool| v.get(k).and_then(Value::as_bool).unwrap_or(d);
        Stops {
            upkeep: b("upkeep", d.upkeep),
            draw: b("draw", d.draw),
            main1: b("main1", d.main1),
            before_combat: b("beforeCombat", d.before_combat),
            end_of_combat: b("endOfCombat", d.end_of_combat),
            main2: b("main2", d.main2),
            end_of_turn: b("endOfTurn", d.end_of_turn),
        }
    }

    /// `isPhaseStepSet`: the other steps (combat's) always stop
    fn stops(&self, step: Step) -> bool {
        match step {
            Step::Upkeep => self.upkeep,
            Step::Draw => self.draw,
            Step::PrecombatMain => self.main1,
            Step::BeginCombat => self.before_combat,
            Step::EndCombat => self.end_of_combat,
            Step::PostcombatMain => self.main2,
            Step::EndTurn => self.end_of_turn,
            _ => true,
        }
    }
}

/// The parts of a player's `UserData` the host uses
#[derive(Clone, Debug)]
pub struct Settings {
    your_turn: Stops,
    opponent_turn: Stops,
    stop_on_all_end_phases: bool,
    stop_on_stack_new_objects: bool,
    stop_on_blockers_any: bool,
    stop_on_blockers_zero: bool,
    pub pass_after_cast: bool,
    pub pass_after_activation: bool,
    pub confirm_empty_mana_pool: bool,
    /// `manaPoolAutomatic`: the pool pays by itself
    pub pool_automatic: bool,
    /// `manaPoolAutomaticRestricted`: only once mana was made for the payment
    pub pool_restricted: bool,
    /// `useFirstManaAbility`: a land with several mana abilities uses its first
    pub use_first_mana_ability: bool,
}

impl Settings {
    /// A `MANA_AUTO_PAYMENT_*` or `USE_FIRST_MANA_ABILITY_*` action; false for another
    pub fn press(&mut self, action: &str) -> bool {
        match action {
            "MANA_AUTO_PAYMENT_ON" => self.pool_automatic = true,
            "MANA_AUTO_PAYMENT_OFF" => self.pool_automatic = false,
            "MANA_AUTO_PAYMENT_RESTRICTED_ON" => self.pool_restricted = true,
            "MANA_AUTO_PAYMENT_RESTRICTED_OFF" => self.pool_restricted = false,
            "USE_FIRST_MANA_ABILITY_ON" => self.use_first_mana_ability = true,
            "USE_FIRST_MANA_ABILITY_OFF" => self.use_first_mana_ability = false,
            _ => return false,
        }
        true
    }

    pub fn parse(user_data: Option<&Value>) -> Settings {
        let u = user_data.cloned().unwrap_or(Value::Null);
        let skips = u.get("userSkipPrioritySteps");
        let b = |v: Option<&Value>, k: &str, d: bool| v.and_then(|v| v.get(k)).and_then(Value::as_bool).unwrap_or(d);
        Settings {
            your_turn: Stops::parse(skips.and_then(|s| s.get("yourTurn"))),
            opponent_turn: Stops::parse(skips.and_then(|s| s.get("opponentTurn"))),
            stop_on_all_end_phases: b(skips, "stopOnAllEndPhases", true),
            stop_on_stack_new_objects: b(skips, "stopOnStackNewObjects", true),
            stop_on_blockers_any: b(skips, "stopOnDeclareBlockersWithAnyPermanents", true),
            stop_on_blockers_zero: b(skips, "stopOnDeclareBlockersWithZeroPermanents", false),
            pass_after_cast: b(Some(&u), "passPriorityCast", false),
            pass_after_activation: b(Some(&u), "passPriorityActivation", false),
            confirm_empty_mana_pool: b(Some(&u), "confirmEmptyManaPool", true),
            pool_automatic: b(Some(&u), "manaPoolAutomatic", true),
            pool_restricted: b(Some(&u), "manaPoolAutomaticRestricted", true),
            use_first_mana_ability: b(Some(&u), "useFirstManaAbility", false),
        }
    }
}

/// The "pass until…" keys the player pressed (`PlayerImpl`'s skip flags)
#[derive(Clone, Debug, Default)]
pub struct Passing {
    /// F4: until the turn's end step
    pub until_end_of_turn: bool,
    /// F5: until the next turn
    pub turn: bool,
    /// F9: until the player's own next turn
    pub all_turns: bool,
    /// F10: until the stack resolves
    pub until_stack_resolved: bool,
    skipped_at_least_once: bool,
    /// the stack's size when it was last seen, to notice new objects
    stack_seen: usize,
    /// the turn F5 was pressed in
    turn_pressed: u32,
    /// HOLD_PRIORITY: no pass after casting
    pub holding: bool,
}

impl Passing {
    /// A `PASS_PRIORITY_*` action; true when it also passes the priority now (`skip()`)
    pub fn press(&mut self, action: &str, game: &Game) -> bool {
        self.skipped_at_least_once = false;
        match action {
            "PASS_PRIORITY_UNTIL_TURN_END_STEP" => self.until_end_of_turn = true,
            "PASS_PRIORITY_UNTIL_NEXT_TURN" => {
                self.turn = true;
                self.turn_pressed = game.turn;
            }
            "PASS_PRIORITY_UNTIL_MY_NEXT_TURN" => self.all_turns = true,
            "PASS_PRIORITY_UNTIL_STACK_RESOLVED" => {
                self.until_stack_resolved = true;
                self.stack_seen = game.stack.len();
            }
            "PASS_PRIORITY_CANCEL_ALL_ACTIONS" => {
                *self = Passing { holding: self.holding, ..Passing::default() };
                return false;
            }
            "HOLD_PRIORITY" => {
                self.holding = true;
                return false;
            }
            "UNHOLD_PRIORITY" => {
                self.holding = false;
                return false;
            }
            _ => return false,
        }
        true
    }

    /// Whether the player passes this priority without being asked
    pub fn passes(&mut self, game: &Game, seat: Seat, settings: &Settings) -> bool {
        let step = game.step.unwrap_or(Step::Upkeep);
        let active = game.active == Some(seat);
        // the F-keys end where they say
        if self.all_turns && active && game.turn > 1 && step != Step::Untap {
            // (the player's own turn: stops again)
            self.all_turns = false;
        }
        if self.turn && game.turn != self.turn_pressed {
            self.turn = false;
        }
        // attacked: stop before blocks (`quickStop`)
        if step == Step::DeclareAttackers && !active {
            let blockers = game
                .battlefield
                .iter()
                .filter(|id| {
                    let o = game.obj(**id);
                    o.controller == seat && game.has_type(**id, playmat_engine::card::CardType::Creature) && !o.tapped
                })
                .count();
            let attacked = !playmat_engine::rules::attackers(game).is_empty();
            if attacked
                && ((blockers > 0 && settings.stop_on_blockers_any)
                    || (blockers == 0 && settings.stop_on_blockers_zero))
            {
                return false;
            }
        }
        if self.all_turns {
            return true;
        }
        if game.stack.is_empty() {
            let mut check_step = true;
            if self.until_stack_resolved {
                self.until_stack_resolved = false;
                check_step = false;
            }
            if self.turn {
                return true;
            }
            if self.until_end_of_turn {
                if step == Step::EndTurn {
                    if !self.skipped_at_least_once || (active && !settings.stop_on_all_end_phases) {
                        self.skipped_at_least_once = true;
                        return true;
                    }
                    self.until_end_of_turn = false;
                    check_step = false;
                } else {
                    self.skipped_at_least_once = true;
                    return true;
                }
            }
            if check_step {
                let stops = if active { &settings.your_turn } else { &settings.opponent_turn };
                return !stops.stops(step);
            }
            false
        } else {
            let new_objects = game.stack.len() > self.stack_seen;
            self.stack_seen = game.stack.len();
            if self.until_stack_resolved && new_objects && active && settings.stop_on_stack_new_objects {
                self.until_stack_resolved = false;
            }
            self.until_stack_resolved
        }
    }
}
