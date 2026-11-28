//! Renegade Reaper: "When Renegade Reaper enters the battlefield, mill four cards. If at least one Angel card is
//! milled this way, you gain 4 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RenegadeReaper.RenegadeReaperEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(4).copied().collect();
    let mill = EffectDef::MillCardsControllerEffect { amount: crate::card::Amount::Fixed(4) };
    cx.run(game, d, &[mill]);
    if top.iter().any(|c| game.obj(*c).zone == Zone::Graveyard && game.has_subtype(*c, "Angel")) {
        gain_life(game, you, 4);
    }
    Some(())
}
