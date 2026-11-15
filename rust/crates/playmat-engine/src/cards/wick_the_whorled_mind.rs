//! Wick, the Whorled Mind: "Whenever Wick or another Rat you control enters, create a 1/1 black Snail creature token
//! if you don't control a Snail. Otherwise, put a +1/+1 counter on a Snail you control. {U}{B}{R}, Sacrifice a
//! Snail: Wick deals damage equal to the sacrificed creature's power to each opponent. Then draw cards equal to the
//! sacrificed creature's power."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("WickTheWhorledMind.WickTheWhorledMindEffect", apply).tokens(&["SnailToken"])];

/// With a Snail you control: one chosen (`TargetPermanent`, not targeted) gets a +1/+1 counter; with none, a Snail
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let snails: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_subtype(*o, "Snail"))
        .collect();
    if snails.is_empty() {
        cx.create_tokens(game, d, &cx.tokens[0], 1);
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &snails, &snails, 1, 1, &rule)?;
    if let Some(&o) = chosen.first() {
        if game.obj(o).zone == Zone::Battlefield {
            put_counters(game, o, "+1/+1", 1, you);
        }
    }
    Some(())
}
