//! High Perfect Morcant: "Whenever High Perfect Morcant or another Elf you control enters, each opponent blights 1.
//! (They each put a -1/-1 counter on a creature they control.)"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HighPerfectMorcant.HighPerfectMorcantEffect", apply)];

/// `BlightCost.doBlight` for each opponent in turn: one creature they control chosen (`TargetPermanent`, not
/// targeted), a -1/-1 counter on it from them; nothing with no creature
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let creatures = controlled_creatures(game, p);
        if creatures.is_empty() {
            continue;
        }
        let chosen = choose_cards(game, d, p, &creatures, &creatures, 1, 1, &rule)?;
        if let Some(&c) = chosen.first() {
            if game.obj(c).zone == Zone::Battlefield {
                add_counters_by(game, c, "-1/-1", 1, Some(p));
            }
        }
    }
    Some(())
}
