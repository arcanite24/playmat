//! Deadlock Trap: "This artifact enters tapped. When it enters, you get {E}{E}. {T}, Pay {E}: Tap target creature or
//! planeswalker. Its activated abilities can't be activated this turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("DeadlockTrap.DeadlockTrapCantActivateEffect", Rule::CantActivate(cant))];

/// the target, while it's still that permanent (`canUseActivatedAbilities` false until end of turn; its mana
/// abilities aren't stopped here, as Koma's)
fn cant(game: &Game, cx: &StaticCx, _: Seat, id: ObjId, _: usize) -> bool {
    cx.target_obj(game, 0) == Some(id)
}
