//! Extinguish the Light: "Destroy target creature or planeswalker. If its mana value was 3 or less, you gain 3 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ExtinguishTheLight.ExtinguishTheLightEffect", apply)];

/// Its mana value as it was, the destroy, then the life (whether or not it was destroyed)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    let mv = game.obj(o).def.mana_value();
    destroy(game, o);
    if mv <= 3 {
        gain_life(game, cx.controller(), 3);
    }
    Some(())
}
