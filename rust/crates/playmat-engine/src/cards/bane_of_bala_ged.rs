//! Bane of Bala Ged: "Whenever Bane of Bala Ged attacks, defending player exiles two permanents they control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BaneOfBalaGed.BaneOfBalaGedEffect", apply)];

/// The defending player (the trigger's target pointer) chooses two permanents they control (`TargetControlledPermanent(2)`,
/// `chooseTarget`) and exiles them together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let theirs: Vec<ObjId> = game.battlefield.iter().copied().filter(|o| game.obj(*o).controller == p).collect();
    let rule = item_rule(game, cx.item);
    if theirs.is_empty() {
        return Some(());
    }
    // (with one, it's picked and the player is asked again for the second, as `TargetControlledPermanent(2)` is)
    let chosen = choose_cards(game, d, p, &theirs, &theirs, 2, 2, &rule)?;
    for o in chosen {
        if game.obj(o).zone == Zone::Battlefield {
            game.move_to(o, Zone::Exile);
        }
    }
    Some(())
}
