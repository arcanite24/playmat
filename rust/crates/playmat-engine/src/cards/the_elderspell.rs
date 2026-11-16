//! The Elderspell: "Destroy any number of target planeswalkers. Choose a planeswalker you control. Put two loyalty
//! counters on it for each planeswalker destroyed this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheElderspell.TheElderspellEffect", apply)];

/// Each target still there destroyed (counted when it left the battlefield), then a planeswalker you control chosen
/// (`TargetPermanent` with `Player.choose`; none, nothing more) gets two loyalty counters for each
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut count = 0;
    for p in cx.objs(0) {
        if game.obj(p).zone != Zone::Battlefield {
            continue;
        }
        destroy(game, p);
        if game.obj(p).zone != Zone::Battlefield {
            count += 1;
        }
    }
    let yours: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Planeswalker))
        .collect();
    if yours.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &yours, &yours, 1, 1, &rule)?;
    if let Some(&p) = chosen.first() {
        if count > 0 {
            add_counters_by(game, p, "loyalty", 2 * count, Some(you));
        }
    }
    Some(())
}
