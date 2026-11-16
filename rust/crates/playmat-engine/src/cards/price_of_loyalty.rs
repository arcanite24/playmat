//! Price of Loyalty: "Gain control of target creature until end of turn. Untap that creature. It gains haste until end
//! of turn. If mana from a Treasure was spent to cast this spell, that creature gets +2/+0 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PriceOfLoyalty.PriceOfLoyaltyEffect", apply)
    .effects(&["new BoostTargetEffect(2, 0)"])];

/// `ManaPaidSourceWatcher.getTreasurePaid` of the spell: +2/+0 on the target
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if game.obj(cx.source_id()).treasure_spent > 0 {
        cx.run(game, d, cx.effects);
    }
    (!d.stopped()).then_some(())
}
