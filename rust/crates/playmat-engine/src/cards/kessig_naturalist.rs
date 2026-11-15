//! Kessig Naturalist // Lord of the Ulvenwald: "Whenever this creature attacks, add {R} or {G}. Until end of turn,
//! you don't lose this mana as steps and phases end. Daybound"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KessigNaturalist.KessigNaturalistEffect", apply)];

/// `chooseUse("Choose red or green", "Red", "Green")`: yes is red; the mana stays in the pool until the turn ends
/// (`addMana(…, emptyOnTurnsEnd)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let red = choose_use(game, d, you, "Choose red or green")?;
    let k = if red { Color::R } else { Color::G }.index();
    let p = game.player_mut(you);
    p.mana_pool.colored[k] += 1;
    p.eot_mana.colored[k] += 1;
    Some(())
}
