//! Liliana's Indignation: "Mill X cards. Target player loses 2 life for each creature card put into your graveyard
//! this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LilianasIndignation.LilianasIndignationEffect", apply)];

/// the top X milled, then 2 life for each creature card of them in your graveyard
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x;
    if x < 1 {
        return Some(());
    }
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(x as usize).copied().collect();
    let mill: EffectDef =
        serde_json::from_value(serde_json::json!({"class": "MillCardsControllerEffect", "amount": x})).ok()?;
    cx.run(game, d, &[mill]);
    let n = top
        .iter()
        .filter(|c| game.obj(**c).zone == Zone::Graveyard && game.has_type(**c, CardType::Creature))
        .count() as i32
        * 2;
    if n > 0 {
        if let Some(p) = cx.player(0) {
            lose_life(game, p, n);
        }
    }
    Some(())
}
