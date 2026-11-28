//! Liliana of the Dark Realms: "−3: Target creature gets +X/+X or -X/-X until end of turn, where X is the number of
//! Swamps you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LilianaOfTheDarkRealms.LilianaOfTheDarkRealmsEffect", apply)];

/// Nothing with no Swamps; the yes is the plus
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let swamps = game
        .battlefield
        .iter()
        .filter(|b| game.obj(**b).controller == you && game.has_subtype(**b, "Swamp"))
        .count() as i32;
    if swamps < 1 {
        return Some(());
    }
    let message = format!("Give +{swamps}/+{swamps} or -{swamps}/-{swamps}?");
    let x = if choose_use(game, d, you, &message)? { swamps } else { -swamps };
    let e = EffectDef::BoostTargetEffect {
        power: Amount::Fixed(x),
        toughness: Amount::Fixed(x),
        duration: crate::card::Duration::EndOfTurn,
    };
    cx.run(game, d, &[e]);
    Some(())
}
