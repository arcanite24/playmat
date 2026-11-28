//! Ulamog's Despoiler: "As Ulamog's Despoiler enters, you may put two cards your opponents own from exile into their
//! owners' graveyards. If you do, Ulamog's Despoiler enters with four +1/+1 counters on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("UlamogsDespoiler.UlamogsDespoilerEffect", apply)
    .effects(&["new AddCountersSourceEffect(CounterType.P1P1.createInstance(4))"])];

/// `TargetCardInExile(2, 2, cards your opponents own)`: only when two can be chosen, chosen (`chooseTarget`), put
/// into their owners' graveyards, then the four counters
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = game.exile.iter().copied().filter(|c| game.obj(*c).owner != you).collect();
    if possible.len() < 2 {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &possible, &possible, 2, 2, &rule)?;
    if chosen.len() < 2 {
        return Some(());
    }
    for c in chosen {
        game.move_to(c, Zone::Graveyard);
    }
    cx.run(game, d, &cx.effects[..1]);
    Some(())
}
