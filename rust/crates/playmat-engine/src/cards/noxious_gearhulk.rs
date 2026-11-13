//! Noxious Gearhulk: "When this creature enters, you may destroy another target creature. If a creature is
//! destroyed this way, you gain life equal to its toughness."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NoxiousGearhulk.NoxiousGearhulkEffect", apply)];

/// "Destroy creature?" (asked only with the target still there); its toughness as it was, once it's gone
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    if !choose_use(game, d, cx.controller(), "Destroy creature?")? {
        return Some(());
    }
    let toughness = game.power_toughness(o).1;
    let zcc = game.obj(o).zcc;
    destroy(game, o);
    if game.obj(o).zcc != zcc && game.obj(o).zone != Zone::Battlefield {
        gain_life(game, cx.controller(), toughness);
    }
    Some(())
}
