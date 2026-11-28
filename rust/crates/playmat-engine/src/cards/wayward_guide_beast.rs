//! Wayward Guide-Beast: "Whenever Wayward Guide-Beast deals combat damage to a player, return a land you control to
//! its owner's hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WaywardGuideBeast.WaywardGuideBeastEffect", apply)];

/// a land you control, chosen (not targeted), to its owner's hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let lands: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Land))
        .collect();
    if lands.is_empty() {
        return Some(());
    }
    // one land: chosen without asking (Java asks no question then)
    if lands.len() == 1 {
        game.move_to(lands[0], Zone::Hand);
        return Some(());
    }
    loop {
        match ask(game, d, you, &Query::Cards { objs: lands.clone(), required: true }) {
            Answer::Obj(o) if lands.contains(&o) => {
                game.move_to(o, Zone::Hand);
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
