//! Xenagos, the Reveler: "+1: Add X mana in any combination of {R} and/or {G}, where X is the number of creatures
//! you control. 0: Create a 2/2 red and green Satyr creature token with haste. -6: Exile the top seven cards of your
//! library. You may put any number of creature and/or land cards from among them onto the battlefield."

use super::*;
use crate::card::{CardType, Color};
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("XenagosTheReveler.XenagosManaEffect", mana),
    Handler::effect("XenagosTheReveler.XenagosExileEffect", exile),
];

/// X the creatures you control; none adds nothing. `getAmount(0, X, "How much RED mana …")`, the rest green
fn mana(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Creature))
        .count() as i32;
    if x == 0 {
        return Some(());
    }
    let red = loop {
        match ask(game, d, you, &Query::Amount { min: 0, max: x }) {
            Answer::Int(n) if (0..=x).contains(&n) => break n,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let pool = &mut game.player_mut(you).mana_pool;
    pool.colored[Color::R.index()] += red as u32;
    pool.colored[Color::G.index()] += (x - red).max(0) as u32;
    Some(())
}

/// the top seven exiled, then any number of the creature and land cards among them chosen (asked while any were
/// exiled) and moved onto the battlefield together under your control
fn exile(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(7).copied().collect();
    for c in &top {
        game.move_to(*c, Zone::Exile);
    }
    let exiled: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
    if exiled.is_empty() {
        return Some(());
    }
    let possible: Vec<ObjId> = exiled
        .iter()
        .copied()
        .filter(|c| game.obj(*c).def.is(CardType::Creature) || game.obj(*c).def.is(CardType::Land))
        .collect();
    let chosen = choose_cards(game, d, you, &exiled, &possible, 0, usize::MAX, &item_rule(game, cx.item))?;
    if chosen.is_empty() {
        return Some(());
    }
    game.entering_together = chosen.clone();
    for c in &chosen {
        enter_battlefield(game, d, *c, you, false);
    }
    game.entering_together.clear();
    Some(())
}
