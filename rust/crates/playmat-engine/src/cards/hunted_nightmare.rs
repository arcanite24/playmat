//! Hunted Nightmare: "Menace. When Hunted Nightmare enters, target opponent puts a deathtouch counter on a creature
//! they control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HuntedNightmare.HuntedNightmareEffect", apply)];

/// With a creature, the opponent chooses one they control (not targeted) and puts the counter on it
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let theirs: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
        .collect();
    if theirs.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, p, &theirs, &theirs, 1, 1, &rule)?;
    if let Some(&c) = chosen.first() {
        add_counters_by(game, c, "deathtouch", 1, Some(p));
    }
    Some(())
}
