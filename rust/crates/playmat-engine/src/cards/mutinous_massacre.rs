//! Mutinous Massacre: "Choose odd or even. Destroy each creature with mana value of the chosen quality. Then gain
//! control of all creatures until end of turn. Untap them. They gain haste until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MutinousMassacre.MutinousMassacreEffect", apply)];

/// "Odd or even?" (`chooseUse` with Odd as yes), then each creature whose mana value has that parity destroyed, in
/// battlefield order (`getActivePermanents`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let odd = choose_use(game, d, cx.controller(), "Odd or even?")?;
    let doomed: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.has_type(*o, CardType::Creature) && (game.obj(*o).def.mana_value() % 2 == 1) == odd)
        .collect();
    for o in doomed {
        destroy(game, o);
    }
    Some(())
}
