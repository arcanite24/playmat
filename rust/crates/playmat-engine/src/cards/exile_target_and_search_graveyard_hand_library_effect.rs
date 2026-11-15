//! `ExileTargetAndSearchGraveyardHandLibraryEffect(true, …)` and `CounterTargetAndSearchGraveyardHandLibraryEffect(true,
//! …, drawForEachHandCard)`: "exile target nonbasic land (counter target spell). Search its controller's graveyard,
//! hand, and library for any number of cards with the same name and exile them. That player shuffles (then draws a
//! card for each card exiled from their hand this way)" (Crumble to Dust, Test of Talents)

use super::search_for_card_name_and_exile::search_and_exile_named;
use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("ExileTargetAndSearchGraveyardHandLibraryEffect", exile_and_search)
        .args(&["one_of:true", "text", "text"]),
    Handler::effect("CounterTargetAndSearchGraveyardHandLibraryEffect", counter_and_search)
        .args(&["one_of:true", "text", "text", "bool"]),
];

/// the target permanent exiled by you, then its name searched for among its controller's cards (the graveyard's
/// optional: the constructors written here take `true`)
fn exile_and_search(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let name = game.obj(o).def.name.clone();
    let controller = game.obj(o).controller;
    cx.run(game, d, &[EffectDef::ExileTargetEffect]);
    search_and_exile_named(game, d, cx.controller(), controller, &name, usize::MAX, false)
}

/// the target spell countered (or left there, if it can't be), then its name searched for among its controller's
/// cards, drawing for each card exiled from the hand with `drawForEachHandCard`
fn counter_and_search(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if cx.args.len() != 4 {
        return Some(());
    }
    let Some(o) = cx.obj(0) else { return Some(()) };
    let Some(spell) = game.stack.iter().find(|i| i.card == o && i.ability.is_none() && i.delayed.is_none()) else {
        return Some(());
    };
    let controller = spell.controller;
    let name = game.obj(o).def.name.clone();
    cx.run(game, d, &[EffectDef::CounterTargetEffect]);
    search_and_exile_named(game, d, cx.controller(), controller, &name, usize::MAX, cx.args[3] == "true")
}
