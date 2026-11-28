//! Voldaren Bloodcaster: "Flying. Whenever Voldaren Bloodcaster or another nontoken creature you control dies, create
//! a Blood token. Whenever you create a Blood token, if you control five or more Blood tokens, transform Voldaren
//! Bloodcaster."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "VoldarenBloodcaster.VoldarenBloodcasterTriggeredAbility",
    "new TokenCreatedTriggeredAbility(new TransformSourceEffect(), filter2, TargetController.YOU)",
    five_blood,
)];

/// `CREATED_TOKEN` of a Blood token you control (the base trigger), and its intervening "if": you control five or
/// more Blood tokens
fn five_blood(game: &Game, t: &StackItem) -> bool {
    game.battlefield
        .iter()
        .filter(|o| {
            let x = game.obj(**o);
            x.token && x.controller == t.controller && game.has_subtype(**o, "Blood")
        })
        .count()
        >= 5
}
