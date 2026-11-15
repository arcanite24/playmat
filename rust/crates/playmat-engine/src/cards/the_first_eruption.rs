//! The First Eruption's chapter III: "Sacrifice a Mountain. If you do, The First Eruption deals 3 damage to each
//! creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheFirstEruption.TheFirstEruptionEffect", apply)];

/// `TargetSacrifice` of one Mountain you control (picked for you when only one fits); if it was sacrificed, 3
/// damage to each creature (`DamageAllEffect`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mountains: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_subtype(*o, "Mountain"))
        .collect();
    if mountains.is_empty() {
        return Some(());
    }
    let &o = choose_cards(game, d, you, &mountains, &mountains, 1, 1, "sacrifice")?.first()?;
    sacrifice(game, o);
    if game.obj(o).zone == Zone::Battlefield {
        return Some(());
    }
    let filter = Filter(vec![crate::card::Pred::Type { t: CardType::Creature }]);
    let e = EffectDef::DamageAllEffect { amount: crate::card::Amount::Fixed(3), filter };
    cx.run(game, d, &[e]);
    Some(())
}
