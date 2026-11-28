//! Multiversal Incursion: "For each nontoken creature you control, create a token that's a copy of that creature,
//! except it isn't legendary."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MultiversalIncursion.MultiversalIncursionEffect", apply)];

/// The nontoken creatures you control as this resolves, in battlefield order, each copied once
/// (`CreateTokenCopyTargetEffect.setSavedPermanent(…).setIsntLegendary(true)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let yours: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && !game.obj(*o).token && game.has_type(*o, CardType::Creature))
        .collect();
    for o in yours {
        token_copy_modified(game, d, o, you, 1, &|def: &mut CardDef| {
            def.supertypes.retain(|s| s != "LEGENDARY");
        });
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
