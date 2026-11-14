//! Doomfall: "Choose one — • Target opponent exiles a creature they control. • Target opponent reveals their hand.
//! You choose a nonland card from it. Exile that card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Doomfall.DoomfallEffect", apply)];

/// the first mode: the opponent chooses a creature they control (not targeted), exiled
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let can: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
        .collect();
    if can.is_empty() {
        return Some(());
    }
    loop {
        match ask(game, d, p, &Query::Cards { objs: can.clone(), required: true }) {
            Answer::Obj(o) if can.contains(&o) => {
                game.move_to(o, Zone::Exile);
                return Some(());
            }
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    }
}
