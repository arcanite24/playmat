//! Glowspore Shaman: "When Glowspore Shaman enters, mill three cards. You may put a land card from your graveyard on
//! top of your library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GlowsporeShaman.GlowsporeShamanEffect", apply)];

/// "Put a land card on top of your library?", then up to one land card of your graveyard (`TargetCardInYourGraveyard(0,
/// 1, filter, true)`, `Player.choose`) on top
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if !choose_use(game, d, you, "Put a land card on top of your library?")? {
        return Some(());
    }
    let lands: Vec<ObjId> =
        game.player(you).graveyard.iter().copied().filter(|c| game.obj(*c).def.is(CardType::Land)).collect();
    if lands.is_empty() {
        return Some(());
    }
    let chosen = choose_cards_rounds(game, d, you, &lands, 0, 1)?;
    if let Some(&c) = chosen.first() {
        to_library(game, c, true);
    }
    Some(())
}
