//! Archfiend of Depravity: "At the beginning of each opponent's end step, that player chooses up to two creatures
//! they control, then sacrifices the rest."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ArchfiendOfDepravity.ArchfiendOfDepravityEffect", apply)];

/// `HumanPlayer.chooseTarget` of up to two creatures to keep (not asked with none). Java sacrifices only when that
/// returns true, which needs at least one creature chosen: keeping none sacrifices nothing, as in Java.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    // "that player": the active player, whose end step it is
    let Some(p) = cx.player(0).or(game.active) else { return Some(()) };
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
        .collect();
    if creatures.is_empty() {
        return Some(());
    }
    let keep = choose_cards(game, d, p, &creatures, &creatures, 0, 2, "creatures to keep")?;
    if keep.is_empty() {
        return Some(());
    }
    let rest: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature) && !keep.contains(o))
        .collect();
    for o in rest {
        if game.obj(o).zone == Zone::Battlefield {
            sacrifice(game, o);
        }
    }
    Some(())
}
