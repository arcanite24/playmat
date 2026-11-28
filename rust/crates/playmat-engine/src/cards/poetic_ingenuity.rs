//! Poetic Ingenuity: "Whenever one or more Dinosaurs you control attack, create that many Treasure tokens."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PoeticIngenuity.PoeticIngenuityEffect", apply)
    .effects(&["new CreateTokenEffect(new TreasureToken())"])];

/// `VALUEKEY_NUMBER_ATTACKERS`: the Dinosaurs you control that are attacking, that many Treasures
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game
        .battlefield
        .iter()
        .filter(|o| {
            let ob = game.obj(**o);
            ob.controller == you && ob.attacking.is_some() && game.has_subtype(**o, "Dinosaur")
        })
        .count() as i32;
    if n < 1 {
        return Some(());
    }
    let mut e = cx.effects[0].clone();
    if let EffectDef::CreateTokenEffect { amount, .. } = &mut e {
        *amount = crate::card::Amount::Fixed(n);
    }
    cx.run_on(game, d, &[e], Vec::new());
    if d.stopped() {
        return None;
    }
    Some(())
}
