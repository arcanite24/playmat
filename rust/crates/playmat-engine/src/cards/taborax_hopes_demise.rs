//! Taborax, Hope's Demise: "Flying. Taborax has lifelink as long as it has five or more +1/+1 counters on it.
//! Whenever another nontoken creature you control dies, put a +1/+1 counter on Taborax. If that creature was a
//! Cleric, you may draw a card. If you do, you lose 1 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TaboraxHopesDemise.TaboraxHopesDemiseEffect", apply)];

/// the creature that died, as it last was: a Cleric, then "Draw a card and lose 1 life?"; a card drawn, 1 life lost
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some((died, _)) = cx.item.died else { return Some(()) };
    let cleric = match game.watch.left_battlefield.iter().rev().find(|l| l.id == died) {
        Some(l) => l.subtypes.iter().any(|s| s == "Cleric") || game.obj(died).def.all_creature_types,
        None => game.has_subtype(died, "Cleric"),
    };
    if !cleric {
        return Some(());
    }
    let you = cx.controller();
    if !choose_use(game, d, you, "Draw a card and lose 1 life?")? {
        return Some(());
    }
    let before = game.player(you).drawn;
    draw(game, you, 1);
    if game.player(you).drawn > before {
        lose_life(game, you, 1);
    }
    Some(())
}
