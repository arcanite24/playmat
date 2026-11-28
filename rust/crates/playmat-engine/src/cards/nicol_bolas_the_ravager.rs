//! Nicol Bolas, the Ravager: "Flying. When Nicol Bolas, the Ravager enters the battlefield, each opponent discards a
//! card. {4}{U}{B}{R}: Exile Nicol Bolas, the Ravager, then return him to the battlefield transformed under his
//! owner's control. Activate only as a sorcery." Nicol Bolas, the Arisen: "+2: Draw two cards. −3: Nicol Bolas, the
//! Arisen deals 10 damage to target creature or planeswalker. −4: Put target creature or planeswalker card from a
//! graveyard onto the battlefield under your control. −12: Exile all but the bottom card of target player's
//! library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NicolBolasTheRavager.NicolBolasTheArisenEffect", apply)];

/// the target player's library but its bottom card exiled together (`controller.moveCards`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let library = game.player(p).library.clone();
    let n = library.len().saturating_sub(1);
    if n == 0 {
        return Some(());
    }
    if !put_cards(game, d, cx.controller(), &library[..n], crate::card::PutCards::Exiled) {
        return None;
    }
    Some(())
}
