//! Academy Loremaster: "At the beginning of each player's draw step, that player may draw an additional card. If
//! they do, spells they cast this turn cost {2} more to cast."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("AcademyLoremaster.AcademyLoremasterEffect", apply),
    Handler::cost("AcademyLoremaster.AcademyLoremasterCostEffect", CostKind::Increase, applies, increase),
];

/// The active player: "Draw an additional card?"; when a card was drawn, `SpellsCostIncreasingAllEffect(2,
/// FILTER_CARD, TargetController.ACTIVE)` until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = game.active else { return Some(()) };
    if !choose_use(game, d, p, "Draw an additional card?")? {
        return Some(());
    }
    let before = game.player(p).drawn;
    draw(game, p, 1);
    if game.player(p).drawn <= before {
        return Some(());
    }
    let handler = serde_json::from_value(serde_json::json!("AcademyLoremaster.AcademyLoremasterCostEffect")).ok()?;
    let e = EffectDef::CardStatic {
        handler,
        args: Vec::new(),
        effects: Vec::new(),
        filters: Vec::new(),
        abilities: Vec::new(),
    };
    cx.run_on(game, d, &[e], Vec::new());
    Some(())
}

/// a spell cast by the active player
fn applies(game: &Game, _: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none() && game.active == Some(of.seat)
}

/// {2} more
fn increase(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic += 2;
}
