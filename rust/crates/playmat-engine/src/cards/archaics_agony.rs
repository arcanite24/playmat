//! Archaic's Agony: "Converge — Archaic's Agony deals X damage to target creature, where X is the number of colors
//! of mana spent to cast this spell. Exile cards from the top of your library equal to the excess damage dealt to
//! that creature this way. You may play those cards until the end of your next turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ArchaicsAgony.ArchaicsAgonyEffect", apply)];

/// X damage with excess (`damageWithExcess`), X the colors of mana spent to cast it (nothing at all below 1); then
/// `ExileTopXMayPlayUntilEffect(excess, UntilEndOfYourNextTurn)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let amount = game.obj(cx.source_id()).spent_colors.iter().filter(|n| **n > 0).count() as i32;
    if amount < 1 {
        return Some(());
    }
    let excess = super::goblin_negotiation::damage_with_excess(game, cx.source_id(), o, amount);
    if excess > 0 {
        let e: EffectDef = serde_json::from_value(serde_json::json!({
            "class": "ExileTopXMayPlayUntilEffect", "amount": excess, "duration": "UntilEndOfYourNextTurn"
        }))
        .ok()?;
        cx.run(game, d, &[e]);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
