//! Garna, the Bloodflame: "When Garna, the Bloodflame enters the battlefield, return to your hand all creature cards
//! in your graveyard that were put there from anywhere this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GarnaTheBloodflame.GarnaTheBloodflameEffect", apply)];

/// `GarnaTheBloodflameWatcher`: the creature cards put into a graveyard this turn (`CardsPutIntoGraveyardWatcher`
/// has them, from anywhere); those you own that are in a graveyard now go to your hand. Java keeps the card's id, so
/// one that left the graveyard and came back still counts.
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut back: Vec<ObjId> = Vec::new();
    for (c, _, _) in game.watch.to_graveyard.clone() {
        let o = game.obj(c);
        if o.owner == you && o.zone == Zone::Graveyard && o.def.is(CardType::Creature) && !back.contains(&c) {
            back.push(c);
        }
    }
    for c in back {
        game.move_to(c, Zone::Hand);
    }
    Some(())
}
