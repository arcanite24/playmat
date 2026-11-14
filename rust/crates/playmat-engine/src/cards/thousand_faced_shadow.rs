//! Thousand-Faced Shadow: "Ninjutsu {2}{U}{U}. Flying. When Thousand-Faced Shadow enters from your hand, if it's
//! attacking, create a token that's a copy of another target attacking creature. The token enters tapped and
//! attacking."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "ThousandFacedShadow.ThousandFacedShadowTriggeredAbility",
    "new EntersBattlefieldTriggeredAbility(new CreateTokenCopyTargetEffect(null, null, false, 1, true, true))",
    attacking,
)];

/// `checkInterveningIfClause`: the source is still on the battlefield and attacking. (Java's `checkTrigger` also
/// wants it to have entered from the hand; the engine doesn't keep where a permanent came from, and the only way it
/// enters attacking from the hand or anywhere is ninjutsu's.)
fn attacking(game: &Game, item: &StackItem) -> bool {
    let o = game.obj(item.card);
    o.zone == Zone::Battlefield && item.ability.is_some_and(|(_, zcc)| zcc == o.zcc) && o.attacking.is_some()
}
