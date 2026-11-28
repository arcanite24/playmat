//! Lavabrink Venturer: "As Lavabrink Venturer enters, choose odd or even. Lavabrink Venturer has protection from
//! each mana value of the chosen quality. (Zero is even.)"

use super::*;
use crate::card::{AbilityDef, Filter, Pred};

pub const HANDLERS: &[Handler] = &[Handler::continuous(
    "LavabrinkVenturer.LavabrinkVenturerEffect",
    itself,
    Layer::Ability(protection),
)];

fn itself(_: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source
}

/// `ManaValueParityPredicate.ODD` / `.EVEN`, on the object's mana value (0 for an ability on the stack, as its
/// targets are chosen: `Game::protected_from_ability_object`)
fn mana_value(game: &Game, id: ObjId) -> u32 {
    if crate::state::ability_object() {
        0
    } else {
        game.obj(id).def.mana_value()
    }
}

fn odd(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    mana_value(game, id) % 2 == 1
}

fn even(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    mana_value(game, id).is_multiple_of(2)
}

/// `new ProtectionAbility(oddFilter)` or `(evenFilter)` by the mode chosen as it entered (`ModeChoice.checkMode`); no
/// ability before one is chosen (`apply` returns false)
fn protection(game: &Game, cx: &StaticCx, _: ObjId) -> Vec<AbilityDef> {
    let marks = &game.obj(cx.source).marks;
    let handler = if marks.iter().any(|m| m == "mode:odd") {
        PredicateHandler("ManaValueParityPredicate.ODD", odd)
    } else if marks.iter().any(|m| m == "mode:even") {
        PredicateHandler("ManaValueParityPredicate.EVEN", even)
    } else {
        return Vec::new();
    };
    vec![AbilityDef::ProtectionAbility { filter: Filter(vec![Pred::Card { handler }]) }]
}
