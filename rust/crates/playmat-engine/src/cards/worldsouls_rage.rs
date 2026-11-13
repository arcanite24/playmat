//! Worldsoul's Rage: "Put up to X land cards from your hand and/or graveyard onto the battlefield tapped."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WorldsoulsRage.WorldsoulsRageEffect", apply)];

/// The land cards in your hand, then those in your graveyard; with any and X above 0, up to X of them chosen
/// (`TargetCard(0, X, Zone.ALL)` with `chooseTarget`), onto the battlefield tapped together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let p = game.player(you);
    let lands: Vec<ObjId> =
        p.hand.iter().chain(p.graveyard.iter()).copied().filter(|c| game.has_type(*c, CardType::Land)).collect();
    let x = cx.item.x.max(0) as usize;
    if lands.is_empty() || x == 0 {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &lands, &lands, 0, x, &rule)?;
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, crate::card::PutCards::BattlefieldTapped) {
        return None;
    }
    Some(())
}
