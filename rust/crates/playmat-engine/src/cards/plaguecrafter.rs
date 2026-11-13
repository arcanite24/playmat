//! Plaguecrafter: "When this creature enters, each player sacrifices a creature or planeswalker of their choice.
//! Each player who can't discards a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Plaguecrafter.PlaguecrafterEffect", apply)
    .filters(&["StaticFilters.FILTER_CONTROLLED_PERMANENT_CREATURE_OR_PLANESWALKER"])];

/// Each player in turn order from you chooses (`TargetSacrifice`, one PICK_TARGET each, picked for them when only
/// one fits); then the chosen are sacrificed together, then each player who had none discards
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut chosen = Vec::new();
    let mut cant = Vec::new();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let possible: Vec<ObjId> = matching(game, &cx.filters[0], p, cx.source_id())
            .into_iter()
            .filter(|o| game.obj(*o).controller == p)
            .collect();
        if possible.is_empty() {
            cant.push(p);
            continue;
        }
        chosen.extend(choose_cards(game, d, p, &possible, &possible, 1, 1, "sacrifice")?);
    }
    for o in chosen {
        if game.obj(o).zone == Zone::Battlefield {
            sacrifice(game, o);
        }
    }
    for p in cant {
        discard(game, d, p, 1);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
