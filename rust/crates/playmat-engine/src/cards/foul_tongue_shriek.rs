//! Foul-Tongue Shriek: "Target opponent loses 1 life for each attacking creature you control. You gain that much
//! life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FoulTongueShriek.FoulTongueShriekEffect", apply)];

/// `AttackingCreatureCount` of the creatures you control; with any, the life loss then the gain
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let n = game
        .battlefield
        .iter()
        .filter(|o| {
            let ob = game.obj(**o);
            ob.controller == you && ob.attacking.is_some() && game.has_type(**o, CardType::Creature)
        })
        .count() as i32;
    if n > 0 {
        lose_life_by(game, p, n, Some(cx.source_id()), false);
        gain_life(game, you, n);
    }
    Some(())
}
