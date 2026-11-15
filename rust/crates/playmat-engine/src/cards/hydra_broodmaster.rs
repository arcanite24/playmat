//! Hydra Broodmaster: "{X}{X}{G}: Monstrosity X. When Hydra Broodmaster becomes monstrous, create X X/X green Hydra
//! creature tokens."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("HydraBroodmaster.HydraBroodmasterEffect", apply).tokens(&["HydraBroodmasterToken"])];

/// X is the monstrosity's (the trigger's X): `CreateTokenEffect(new HydraBroodmasterToken(x, x), x)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x;
    let mut token = cx.tokens[0].clone();
    token.power = x;
    token.toughness = x;
    cx.create_tokens(game, d, &token, x);
    if d.stopped() {
        return None;
    }
    Some(())
}
