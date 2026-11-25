//! Rambling Possum: "Whenever this creature attacks while saddled, it gets +1/+2 until end of turn. Then you may
//! return any number of creatures that saddled it this turn to their owner's hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RamblingPossum.RamblingPossumEffect", apply)];

/// `TargetPermanent(0, MAX, creatures that saddled it this turn, true)` chosen (the saddle cost's marks, by the
/// Possum as the trigger was made: `SaddledSourceThisTurnPredicate`), returned to their owners' hands
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let (mount, zcc) = cx.source;
    let mark = format!("saddled mount {} {} {}", mount.0, zcc, game.turn);
    let saddlers: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.has_type(*o, CardType::Creature) && game.obj(*o).marks.contains(&mark))
        .collect();
    if saddlers.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &saddlers, &saddlers, 0, saddlers.len(), &rule)?;
    for o in chosen {
        if game.obj(o).zone == Zone::Battlefield {
            game.move_to(o, Zone::Hand);
        }
    }
    Some(())
}
