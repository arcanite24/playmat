//! Avatar of Burgeoning Echoes: "Landfall — Whenever a land you control enters, empower Jace 2. Planeswalkers you
//! control have '−10: Put a +1/+1 counter on target creature for each land you control.'"

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("AvatarOfBurgeoningEchoes.AvatarOfBurgeoningEchoesEffect", apply)];

/// a +1/+1 counter on the target for each land you control (none at all with no land)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let you = cx.controller();
    let lands = game
        .battlefield
        .iter()
        .filter(|l| game.obj(**l).controller == you && game.has_type(**l, CardType::Land))
        .count() as i32;
    if lands > 0 {
        add_counters_by(game, o, "+1/+1", lands, Some(you));
    }
    Some(())
}
