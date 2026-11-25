//! Yuriko, Blade of the Mighty: "During combat, players can't cast spells or activate abilities that aren't mana
//! abilities. Whenever a creature you control attacks a player alone, it gains double strike until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::rule(
    "YurikoBladeOfTheMighty.YurikoBladeOfTheMightyEffect",
    Rule::Several(&[Rule::CantCast(cast_in_combat), Rule::CantActivate(activate_in_combat)]),
)];

/// `game.getTurnPhaseType() == TurnPhase.COMBAT`: from the beginning of combat to its end step
fn in_combat(game: &Game) -> bool {
    matches!(
        game.step,
        Some(
            Step::BeginCombat
                | Step::DeclareAttackers
                | Step::DeclareBlockers
                | Step::FirstCombatDamage
                | Step::CombatDamage
                | Step::EndCombat
        )
    )
}

/// `CAST_SPELL` during combat: any spell, by any player
fn cast_in_combat(game: &Game, _cx: &StaticCx, _seat: Seat, _id: ObjId, _face: &CardDef) -> bool {
    in_combat(game)
}

/// `ACTIVATE_ABILITY` during combat of an ability that isn't a mana ability (the hook asks for those only)
fn activate_in_combat(game: &Game, _cx: &StaticCx, _seat: Seat, _id: ObjId, _index: usize) -> bool {
    in_combat(game)
}
