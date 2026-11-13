//! Tribute to the World Tree: "Whenever a creature you control enters, draw a card if its power is 3 or greater.
//! Otherwise, put two +1/+1 counters on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TributeToTheWorldTree.TributeToTheWorldTreeEffect", apply)];

/// Its power now (or as it last was on the battlefield)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.item.entering else { return Some(()) };
    let mut power = game.power_toughness(o).0;
    if game.obj(o).zone != Zone::Battlefield {
        // gone: as it last was, with its counters (`getPermanentOrLKIBattlefield`)
        let lki = |name: &str| game.obj(o).lki_counters.iter().filter(|(n, _)| n == name).map(|(_, k)| *k).sum::<i32>();
        power += lki("+1/+1") - lki("-1/-1");
    }
    if power < 3 {
        if game.obj(o).zone == Zone::Battlefield {
            add_counters_by(game, o, "+1/+1", 2, Some(cx.controller()));
        }
    } else {
        draw(game, cx.controller(), 1);
    }
    Some(())
}
