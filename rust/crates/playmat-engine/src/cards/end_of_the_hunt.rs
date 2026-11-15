//! End of the Hunt: "Target opponent exiles a creature or planeswalker they control with the greatest mana value among
//! creatures and planeswalkers they control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EndOfTheHunt.EndOfTheHuntEffect", apply)];

/// `opponent.choose` of one (not targeted, picked for them when only one fits), then the exile
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let theirs: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            game.obj(*o).controller == p
                && (game.has_type(*o, CardType::Creature) || game.has_type(*o, CardType::Planeswalker))
        })
        .collect();
    let Some(most) = theirs.iter().map(|o| game.obj(*o).def.mana_value()).max() else { return Some(()) };
    let possible: Vec<ObjId> = theirs.into_iter().filter(|o| game.obj(*o).def.mana_value() >= most).collect();
    let rule = item_rule(game, cx.item);
    let Some(&o) = choose_cards(game, d, p, &possible, &possible, 1, 1, &rule)?.first() else { return Some(()) };
    if game.obj(o).zone == Zone::Battlefield {
        game.move_to(o, Zone::Exile);
    }
    Some(())
}
