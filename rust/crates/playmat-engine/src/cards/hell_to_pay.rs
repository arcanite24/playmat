//! Hell to Pay: "Hell to Pay deals X damage to target creature. Create a number of tapped Treasure tokens equal to
//! the amount of excess damage dealt to that creature this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HellToPay.HellToPayEffect", apply)
    .effects(&["new CreateTokenEffect(new TreasureToken(), 1, true)"])];

/// X damage with excess (`damageWithExcess`), then that many tapped Treasures
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let excess = super::goblin_negotiation::damage_with_excess(game, cx.source_id(), o, cx.item.x.max(0));
    if excess > 0 {
        let mut e = cx.effects[0].clone();
        if let EffectDef::CreateTokenEffect { amount, .. } = &mut e {
            *amount = Amount::Fixed(excess);
        }
        cx.run_on(game, d, &[e], Vec::new());
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
