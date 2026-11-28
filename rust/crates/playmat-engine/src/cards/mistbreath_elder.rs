//! Mistbreath Elder: "At the beginning of your upkeep, return another creature you control to its owner's hand. If
//! you do, put a +1/+1 counter on Mistbreath Elder. Otherwise, you may return Mistbreath Elder to its owner's hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MistbreathElder.MistbreathElderEffect", apply)];

/// The "otherwise": with the Elder still on the battlefield, you may (`chooseUse`) return it to its owner's hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let you = cx.controller();
    let src = cx.source_id();
    let msg = format!("Return {} to its owner's hand?", game.obj(src).name());
    if choose_use(game, d, you, &msg)? {
        put_cards(game, d, you, &[src], crate::card::PutCards::Hand);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
