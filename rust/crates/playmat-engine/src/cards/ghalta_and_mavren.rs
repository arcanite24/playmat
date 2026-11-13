//! Ghalta and Mavren: "Whenever you attack, choose one — • Create a tapped and attacking X/X green Dinosaur creature
//! token with trample, where X is the greatest power among other attacking creatures. • Create X 1/1 white Vampire
//! creature tokens with lifelink, where X is the number of other attacking creatures."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GhaltaAndMavren.GhaltaAndMavrenEffect", apply)
    .effects(&["new CreateTokenEffect(new DinosaurXXToken(), 1, true, true)"])];

/// `GreatestAmongPermanentsValue(Power, other attacking creatures)`, then the X/X token tapped and attacking
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    let x = attackers(game)
        .into_iter()
        .filter(|o| *o != src && game.has_type(*o, CardType::Creature))
        .map(|o| game.power_toughness(o).0)
        .max()
        .unwrap_or(0)
        .max(0);
    let mut e = cx.effects[0].clone();
    if let EffectDef::CreateTokenEffect { token, .. } = &mut e {
        token.power = x;
        token.toughness = x;
    }
    cx.run(game, d, &[e]);
    Some(())
}
