//! Exquisite Archangel: "Flying. If you would lose the game, instead exile Exquisite Archangel and your life total
//! becomes equal to your starting life total."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::replacement("ExquisiteArchangel.ExquisiteArchangelEffect", &[EventType::Loses], applies, replace)];

/// `LOSES` of the Archangel's controller
fn applies(_game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.player == cx.controller
}

/// While it's on the battlefield: `ExileSourceEffect`, then `setLife(startingLife)` (a gain or a loss of the
/// difference); the loss is replaced
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    if game.obj(cx.source).zone != Zone::Battlefield {
        return false;
    }
    game.move_to(cx.source, Zone::Exile);
    let to = game.starting_life;
    super::set_player_life_source_effect::set_life(game, ev.player, to);
    true
}
