//! Balin, Loremaster: "Whenever Balin or another Dwarf you control enters, you may discard your hand. Draw X cards,
//! where X is the number of cards discarded this way. If you have an enduring story, Balin deals X damage to each
//! opponent."

use super::*;
use crate::card::Condition;

pub const HANDLERS: &[Handler] = &[Handler::effect("BalinLoremaster.BalinLoremasterEffect", apply)];

/// X is the hand's size before; `DiscardHandDrawSameNumberSourceEffect`, then with an enduring story
/// `DamagePlayersEffect(X, OPPONENT)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let amount = game.player(you).hand.len() as i32;
    cx.run(game, d, &[EffectDef::DiscardHandDrawSameNumberSourceEffect]);
    if d.stopped() {
        return None;
    }
    if game.condition_holds(Some(&Condition::EnduringStory), you, cx.source_id()) {
        let e = EffectDef::DamagePlayersEffect { amount: Amount::Fixed(amount), controller: "opponent".into() };
        cx.run(game, d, &[e]);
    }
    (!d.stopped()).then_some(())
}
