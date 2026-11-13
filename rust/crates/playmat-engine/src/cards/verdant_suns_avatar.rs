//! Verdant Sun's Avatar: "Whenever this creature or another creature you control enters, you gain life equal to
//! that creature's toughness."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "VerdantSunsAvatar.VerdantSunsAvatarTriggeredAbility",
        "new EntersBattlefieldAllTriggeredAbility(Zone.BATTLEFIELD, new VerdantSunsAvatarEffect(), StaticFilters.FILTER_CONTROLLED_CREATURE, false, SetTargetPointer.PERMANENT)",
        any,
    ),
    // the same effect as Trostani's
    Handler::effect("VerdantSunsAvatar.VerdantSunsAvatarEffect", super::trostani_selesnyas_voice::gain_toughness),
];

/// `checkTrigger` is the base trigger's: a creature you control entered, this one too
fn any(_: &Game, _: &StackItem) -> bool {
    true
}
