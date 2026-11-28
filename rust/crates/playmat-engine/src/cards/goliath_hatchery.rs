//! Goliath Hatchery: "When Goliath Hatchery enters, create two 3/3 green Phyrexian Beast creature tokens with
//! toxic 1. Corrupted — At the beginning of your upkeep, if an opponent has three or more poison counters, choose
//! a creature you control, then draw cards equal to its total toxic value."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GoliathHatchery.GoliathHatcheryEffect", apply)];

/// `choose(TargetControlledCreaturePermanent)`, not targeted (nothing with no creature); then its toxic abilities'
/// amounts (its own and granted ones) in cards drawn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mine: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    if mine.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(&o) = choose_cards(game, d, you, &mine, &mine, 1, 1, &rule)?.first() else { return Some(()) };
    let toxic = |a: &AbilityDef| match a {
        AbilityDef::ToxicAbility { amount, .. } => *amount,
        _ => 0,
    };
    let mut n: i32 = game.obj(o).def.abilities.iter().map(toxic).sum();
    n += game.granted_instances(o).iter().map(toxic).sum::<i32>();
    if n > 0 {
        draw(game, you, n as usize);
    }
    Some(())
}
