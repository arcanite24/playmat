//! `BlightTargetEffect(n)`: "target opponent blights N" (Champion of the Weird): `BlightCost.doBlight` for the
//! target player, a creature they control chosen (not targeted) gets N -1/-1 counters; nothing without a creature.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BlightTargetEffect", apply).args(&["int"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(who) = cx.player(0) else { return Some(()) };
    let n = cx.arg_int(0)?;
    let creatures = controlled_creatures(game, who);
    if n < 1 || creatures.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    if let Some(&c) = choose_cards(game, d, who, &creatures, &creatures, 1, 1, &rule)?.first() {
        put_counters(game, c, "-1/-1", n, who);
    }
    Some(())
}
