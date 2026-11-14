//! Skulking Killer: "When Skulking Killer enters the battlefield, target creature an opponent controls gets -2/-2
//! until end of turn if that opponent controls no other creatures."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SkulkingKiller.SkulkingKillerEffect", apply)];

/// Only while its controller controls no other creature
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    let p = game.obj(o).controller;
    let n = game
        .battlefield
        .iter()
        .filter(|c| game.obj(**c).controller == p && game.has_type(**c, CardType::Creature))
        .count();
    if n > 1 {
        return None;
    }
    boost(game, o, -2, -2, crate::card::Duration::EndOfTurn);
    Some(())
}
