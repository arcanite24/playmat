//! Deadly Brew: "Each player sacrifices a creature or planeswalker of their choice. If you sacrificed a permanent
//! this way, you may return another permanent card from your graveyard to your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DeadlyBrew.DeadlyBrewEffect", apply)];

/// each player in range from you chooses one of theirs; the others' are sacrificed first; the permanent cards in
/// your graveyard are noted, then yours is sacrificed and you may return one of those
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    let mut others = Vec::new();
    let mut yours = None;
    for k in 0..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let possible: Vec<ObjId> = game
            .battlefield
            .iter()
            .copied()
            .filter(|o| {
                game.obj(*o).controller == p
                    && (game.has_type(*o, CardType::Creature) || game.has_type(*o, CardType::Planeswalker))
            })
            .collect();
        if possible.is_empty() {
            continue;
        }
        let chosen = choose_cards(game, d, p, &possible, &possible, 1, 1, &rule)?;
        let Some(&o) = chosen.first() else { continue };
        if game.obj(o).controller == you {
            yours = Some(o);
        } else {
            others.push(o);
        }
    }
    for o in others {
        if game.obj(o).zone == Zone::Battlefield {
            sacrifice(game, o);
        }
    }
    let grave: Vec<ObjId> =
        game.player(you).graveyard.iter().copied().filter(|c| game.obj(*c).def.is_permanent_card()).collect();
    let Some(o) = yours.filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    sacrifice(game, o);
    let possible: Vec<ObjId> = grave.iter().copied().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
    let chosen = choose_cards(game, d, you, &possible, &possible, 0, 1, &rule)?;
    if let Some(&c) = chosen.first() {
        game.move_to(c, Zone::Hand);
    }
    Some(())
}
