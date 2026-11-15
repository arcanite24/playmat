//! Discovery // Dispersal: Dispersal's "Each opponent returns a nonland permanent they control with the highest mana
//! value among permanents they control to its owner's hand, then discards a card."

use super::*;
use crate::card::Amount;

pub const HANDLERS: &[Handler] = &[Handler::effect("DiscoveryDispersal.DispersalEffect", dispersal)];

/// each opponent chooses (not targeted) a nonland permanent of theirs whose mana value is the highest among all
/// their permanents (lands too); those return to their owners' hands together, in battlefield order
/// (`ReturnToHandFromBattlefieldAllEffect`), then each opponent discards a card (`DiscardEachPlayerEffect`)
fn dispersal(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    let mut chosen_all = Vec::new();
    for k in 1..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let theirs: Vec<ObjId> = game.battlefield.iter().copied().filter(|o| game.obj(*o).controller == p).collect();
        let highest = theirs.iter().map(|o| game.obj(*o).def.mana_value() as i32).fold(0, i32::max);
        let possible: Vec<ObjId> = theirs
            .into_iter()
            .filter(|o| !game.has_type(*o, CardType::Land) && game.obj(*o).def.mana_value() as i32 == highest)
            .collect();
        // (one with none isn't asked)
        if possible.is_empty() {
            continue;
        }
        chosen_all.extend(choose_cards(game, d, p, &possible, &possible, 1, 1, &rule)?);
    }
    let back: Vec<ObjId> = game.battlefield.iter().copied().filter(|o| chosen_all.contains(o)).collect();
    for o in back {
        game.move_to(o, Zone::Hand);
    }
    let discard = EffectDef::DiscardEachPlayerEffect { amount: Amount::Fixed(1), who: "opponent".into() };
    cx.run(game, d, &[discard]);
    Some(())
}
