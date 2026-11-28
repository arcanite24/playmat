//! Wonder Man, Hollywood Hero: "Each power-up ability of permanents you control can be activated an additional
//! time."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("WonderManHollywoodHero.WonderManHollywoodHeroEffect", Rule::MaxActivations(twice))];

/// `MAX_ACTIVATIONS` (per game) of a `PowerUpAbility` activated by Wonder Man's controller: one more than once
fn twice(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, index: usize) -> Option<u32> {
    let power_up = matches!(game.obj(id).def.abilities.get(index), Some(AbilityDef::PowerUpAbility { .. }));
    (power_up && seat == cx.controller).then_some(2)
}
