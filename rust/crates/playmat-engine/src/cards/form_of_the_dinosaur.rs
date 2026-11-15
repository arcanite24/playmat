//! Form of the Dinosaur: "When Form of the Dinosaur enters, your life total becomes 15. At the beginning of your
//! upkeep, Form of the Dinosaur deals 15 damage to target creature an opponent controls and that creature deals
//! damage equal to its power to you."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FormOfTheDinosaur.FormOfTheDinosaurEffect", apply)];

/// 15 damage to the target creature, then its power (as it is after that damage) to you, from it
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(c) = cx.obj(0) else { return Some(()) };
    if game.obj(c).zone != Zone::Battlefield {
        return Some(());
    }
    deal_damage(game, cx.source_id(), Target::Obj(c), 15, false);
    let power = game.power_toughness(c).0;
    deal_damage(game, c, Target::Player(cx.controller()), power, false);
    Some(())
}
