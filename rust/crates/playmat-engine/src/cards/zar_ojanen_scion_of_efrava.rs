//! Zar Ojanen, Scion of Efrava: "Domain — Whenever Zar Ojanen becomes tapped, put a +1/+1 counter on each creature
//! you control with toughness less than the number of basic land types among lands you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("ZarOjanenScionOfEfrava.ZarOjanenScionOfEfravaEffect", apply)];

/// The domain count first, then each creature you control (battlefield order) under it gets a counter
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let domain = game.value(&crate::card::Amount::Of(crate::card::Value::Domain), you, cx.source_id(), 0);
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            game.obj(*o).controller == you && game.has_type(*o, CardType::Creature) && game.power_toughness(*o).1 < domain
        })
        .collect();
    for o in creatures {
        add_counters_by(game, o, "+1/+1", 1, Some(you));
    }
    Some(())
}
