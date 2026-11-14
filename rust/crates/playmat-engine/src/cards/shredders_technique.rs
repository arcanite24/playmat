//! Shredder's Technique: "Destroy target creature or enchantment. If an enchantment was destroyed this way, you lose
//! 2 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ShreddersTechnique.ShreddersTechniqueEffect", apply)
    .effects(&["new LoseLifeSourceControllerEffect(2)"])];

/// The destroy; then, when it was destroyed and was an enchantment, you lose 2
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    let enchantment = game.has_type(o, CardType::Enchantment);
    let zcc = game.obj(o).zcc;
    destroy(game, o);
    if game.obj(o).zcc == zcc {
        return None;
    }
    if enchantment {
        cx.run(game, d, &cx.effects[..1]);
    }
    Some(())
}
