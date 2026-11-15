//! `BlightControllerEffect(n)`: "blight N" (Chaos Spewer, Dose of Dawnglow, Shadow Urchin): `BlightCost.doBlight`,
//! a creature you control chosen (not targeted) gets N -1/-1 counters; nothing without a creature.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BlightControllerEffect", apply).args(&["int"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = cx.arg_int(0)?;
    let creatures = controlled_creatures(game, you);
    if n < 1 || creatures.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    if let Some(&c) = choose_cards(game, d, you, &creatures, &creatures, 1, 1, &rule)?.first() {
        put_counters(game, c, "-1/-1", n, you);
    }
    Some(())
}
