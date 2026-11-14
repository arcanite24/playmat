//! Champion of the Path: "Whenever another Elemental you control enters, it deals damage equal to its power to each
//! opponent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ChampionOfThePath.ChampionOfThePathEffect", apply)];

/// The Elemental that entered (`permanentEnteringBattlefield`), its power as it is now (or as it last was): with
/// power 1 or more, that much damage from it to each opponent
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let it = cx.item.entering?;
    let power = if game.obj(it).zone == Zone::Battlefield {
        game.power_toughness(it).0
    } else {
        game.obj(it).power_lki?
    };
    if power < 1 {
        return Some(());
    }
    let you = cx.controller();
    let opponents: Vec<Seat> = (0..game.seats()).filter(|p| *p != you && !game.player(*p).lost).collect();
    for p in opponents {
        deal_damage(game, it, Target::Player(p), power, false);
    }
    Some(())
}
