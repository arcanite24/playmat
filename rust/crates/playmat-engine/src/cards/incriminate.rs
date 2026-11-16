//! Incriminate: "Choose two target creatures controlled by the same player. That player sacrifices one of them."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Incriminate.IncriminateEffect", apply)];

/// The controller of the last target still there chooses one of the targets still there ("creature to sacrifice",
/// `TargetSacrifice`, not targeted) and sacrifices it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let them: Vec<ObjId> = cx.objs(0).into_iter().filter(|o| game.obj(*o).zone == Zone::Battlefield).collect();
    let Some(&last) = them.last() else { return Some(()) };
    let player = game.obj(last).controller;
    let chosen = choose_cards(game, d, player, &them, &them, 1, 1, "creature to sacrifice")?;
    if let Some(&c) = chosen.first() {
        sacrifice(game, c);
    }
    Some(())
}
