//! Academic Probation: "Choose one — Choose a nonland card name. Opponents can't cast spells with the chosen name
//! until your next turn. / Choose target nonland permanent. Until your next turn, it can't attack or block, and its
//! activated abilities can't be activated."
//! (the first mode is `OpponentsCantCastChosenUntilNextTurnEffect`'s handler; the second as Linvala, Shield of Sea
//! Gate's restriction: its mana abilities aren't stopped)

use super::*;

const CANT_ATTACK_OR_BLOCK: &str = "AcademicProbation.AcademicProbationCantAttackOrBlock";
const CANT_ACTIVATE: &str = "AcademicProbation.AcademicProbationCantActivate";

pub const HANDLERS: &[Handler] = &[
    Handler::effect("AcademicProbation.AcademicProbationRestrictionEffect", restrict),
    Handler::restriction(
        "AcademicProbation.AcademicProbationCantAttackOrBlock",
        the_target,
        &[Restrict::CantAttack(never_attack), Restrict::CantBlock(never_block)],
    )
    .until_your_next_turn(),
    Handler::rule("AcademicProbation.AcademicProbationCantActivate", Rule::CantActivate(cant_activate))
        .until_your_next_turn(),
];

/// the restriction and the can't-activate rule, both pointing at the spell's target, until your next turn
fn restrict(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let mut effects = Vec::new();
    for name in [CANT_ATTACK_OR_BLOCK, CANT_ACTIVATE] {
        let handler = serde_json::from_value(serde_json::json!(name)).ok()?;
        effects.push(EffectDef::CardStatic {
            handler,
            args: Vec::new(),
            effects: Vec::new(),
            filters: Vec::new(),
            abilities: Vec::new(),
        });
    }
    cx.run(game, d, &effects);
    Some(())
}

/// the target, while it's still that permanent
fn the_target(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    cx.target_obj(game, 0) == Some(id)
}

fn never_attack(_: &Game, _: &StaticCx, _: ObjId) -> bool {
    true
}

fn never_block(_: &Game, _: &StaticCx, _: ObjId, _: ObjId) -> bool {
    true
}

fn cant_activate(game: &Game, cx: &StaticCx, _: Seat, id: ObjId, _: usize) -> bool {
    cx.target_obj(game, 0) == Some(id)
}
