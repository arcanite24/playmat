//! Fall of the Impostor: "(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)
//! I, II — Put a +1/+1 counter on up to one target creature. III — Exile a creature with the greatest power among
//! creatures target opponent controls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FallOfTheImpostor.FallOfTheImpostorEffect", apply)];

/// The opponent's creatures, in battlefield order: none, nothing; one, that one; more, one with the greatest power
/// chosen (`chooseTarget`, not targeted); exiled
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
        .collect();
    let it = match creatures.len() {
        0 => return Some(()),
        1 => creatures[0],
        _ => {
            let most = creatures.iter().map(|c| game.power_toughness(*c).0).max().unwrap_or(i32::MIN);
            let possible: Vec<ObjId> =
                creatures.iter().copied().filter(|c| game.power_toughness(*c).0 >= most).collect();
            let rule = item_rule(game, cx.item);
            match choose_cards(game, d, you, &possible, &possible, 1, 1, &rule)?.first() {
                Some(c) => *c,
                None => return Some(()),
            }
        }
    };
    if game.obj(it).zone == Zone::Battlefield {
        game.move_to(it, Zone::Exile);
    }
    Some(())
}
