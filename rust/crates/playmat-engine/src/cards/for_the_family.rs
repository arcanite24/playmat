//! For the Family: "Target creature gets +2/+2 until end of turn. If you control four or more creatures, that
//! creature gets +4/+4 until end of turn instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ForTheFamily.ForTheFamilyEffect", apply)];

/// The creatures you control as it resolves: four or more, +4/+4
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Creature))
        .count();
    let k = if n >= 4 { 4 } else { 2 };
    let e = EffectDef::BoostTargetEffect {
        power: crate::card::Amount::Fixed(k),
        toughness: crate::card::Amount::Fixed(k),
        duration: crate::card::Duration::EndOfTurn,
    };
    cx.run(game, d, &[e]);
    Some(())
}
