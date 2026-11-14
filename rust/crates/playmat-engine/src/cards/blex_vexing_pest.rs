//! Blex, Vexing Pest // Search for Blex: "Look at the top five cards of your library. You may put any number of them
//! into your hand and the rest into your graveyard. You lose 3 life for each card you put into your hand this way."

use super::*;
use crate::card::{Filter, PutCards};

pub const HANDLERS: &[Handler] = &[Handler::effect("BlexVexingPest.SearchForBlexEffect", apply)];

/// `LookLibraryAndPickControllerEffect(5, Integer.MAX_VALUE, PutCards.HAND, PutCards.GRAVEYARD)` made optional
/// (asked first; not "up to", so it's all of them or none), then 3 life lost for each card put into the hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(5).copied().collect();
    let e = EffectDef::LookLibraryAndPickControllerEffect {
        look: 5,
        look_count: None,
        pick: i32::MAX,
        filter: Filter::default(),
        picked: PutCards::Hand,
        looked: PutCards::Graveyard,
        up_to: false,
        optional: true,
        pick_x: false,
    };
    cx.run(game, d, &[e]);
    if d.stopped() {
        return None;
    }
    let picked = top.iter().filter(|c| game.obj(**c).zone == Zone::Hand).count() as i32;
    if picked > 0 {
        lose_life(game, you, picked * 3);
    }
    Some(())
}
