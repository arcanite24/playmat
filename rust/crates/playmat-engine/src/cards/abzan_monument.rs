//! Abzan Monument: "When this artifact enters, search your library for a basic Plains, Swamp, or Forest card, reveal
//! it, put it into your hand, then shuffle. {1}{W}{B}{G}, {T}, Sacrifice this artifact: Create an X/X white Spirit
//! creature token, where X is the greatest toughness among creatures you control. Activate only as a sorcery."

use super::*;
use crate::card::Amount;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("AbzanMonument.AbzanMonumentEffect", apply).tokens(&["SpiritXXToken"])];

/// `GreatestAmongPermanentsValue.TOUGHNESS_CONTROLLED_CREATURES` (0 with none), then `CreateTokenEffect(new
/// SpiritXXToken(value))`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Creature))
        .map(|o| game.power_toughness(*o).1)
        .max()
        .unwrap_or(0)
        .max(0);
    let e = EffectDef::CreateXXTokenEffect { token: cx.tokens[0].clone(), amount: Amount::Fixed(n) };
    cx.run_on(game, d, &[e], Vec::new());
    Some(())
}
