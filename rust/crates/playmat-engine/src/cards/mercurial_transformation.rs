//! Mercurial Transformation: "Until end of turn, target nonland permanent loses all abilities and becomes your
//! choice of a blue Frog creature with base power and toughness 1/1 or a blue Octopus creature with base power and
//! toughness 4/4."

use super::*;
use crate::card::{Color, Duration, TokenDef};

pub const HANDLERS: &[Handler] = &[Handler::effect("MercurialTransformation.MercurialTransformationEffect", apply)];

/// "1/1 Frog or 4/4 Octopus?" (yes: the Frog), then `BecomesCreatureTargetEffect(token, true, false, EndOfTurn)`
/// with `withLoseOtherCardTypes(true)` on the target: a creature and nothing else until end of turn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let frog = choose_use(game, d, you, "1/1 Frog or 4/4 Octopus?")?;
    let (subtype, pt) = if frog { ("Frog", 1) } else { ("Octopus", 4) };
    let token = TokenDef {
        name: String::new(),
        class: String::new(),
        types: vec![CardType::Creature],
        colors: vec![Color::U],
        subtypes: vec![subtype.into()],
        supertypes: Vec::new(),
        power: pt,
        toughness: pt,
        abilities: Vec::new(),
        all_creature_types: false,
        loyalty: None,
        images: Default::default(),
        back: None,
    };
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    let e = EffectDef::BecomesCreatureTargetEffect {
        token,
        duration: Duration::EndOfTurn,
        lose_abilities: true,
        still_land: false,
        named: None,
    };
    cx.run_on(game, d, &[e], vec![Target::Obj(t)]);
    // (`loseOtherCardTypes`: its other card types go too, and with them the subtypes it can't have then, every
    // subtype that isn't a creature type: `SubType.canGain`)
    if game.obj(t).zone == Zone::Battlefield && game.obj(t).def.types.iter().any(|c| *c != CardType::Creature) {
        let mut def = (*game.obj(t).def).clone();
        def.types.retain(|c| *c == CardType::Creature);
        def.subtypes.retain(|s| !crate::card::NON_CREATURE_TYPES.contains(&s.as_str()));
        game.obj_mut(t).def = std::sync::Arc::new(def);
    }
    Some(())
}
