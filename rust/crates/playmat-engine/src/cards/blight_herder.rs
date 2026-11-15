//! Blight Herder: "When you cast this spell, you may put two cards your opponents own from exile into their owners'
//! graveyards. If you do, create three 1/1 colorless Eldrazi Scion creature tokens. They have \"Sacrifice this
//! creature: Add {C}.\""

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("BlightHerder.BlightHerderEffect", apply).tokens(&["EldraziScionToken"])];

/// `TargetCardInExile(2, 2, cards your opponents own)`, only when two can be chosen; the two into their owners'
/// graveyards, then three Scions
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = game
        .exile
        .iter()
        .copied()
        .filter(|c| {
            let o = game.obj(*c);
            o.owner != you && !o.token
        })
        .collect();
    if possible.len() < 2 {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &possible, &possible, 2, 2, &rule)?;
    for c in chosen {
        game.move_to(c, Zone::Graveyard);
    }
    cx.create_tokens(game, d, &cx.tokens[0], 3);
    Some(())
}
