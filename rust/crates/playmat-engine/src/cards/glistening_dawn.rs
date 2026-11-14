//! Glistening Dawn: "Incubate X twice, where X is the number of lands you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("GlisteningDawn.GlisteningDawnEffect", apply).effects(&["new IncubateEffect(0)"])];

/// the lands counted once, then two incubates of that many
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let lands =
        game.battlefield.iter().filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Land)).count();
    let mut e = cx.effects[0].clone();
    if let EffectDef::IncubateEffect { amount, .. } = &mut e {
        *amount = lands as i32;
    }
    cx.run(game, d, &[e.clone()]);
    if d.stopped() {
        return None;
    }
    cx.run(game, d, &[e]);
    Some(())
}
