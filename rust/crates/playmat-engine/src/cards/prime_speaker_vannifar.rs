//! Prime Speaker Vannifar: "{T}, Sacrifice another creature: Search your library for a creature card with mana value
//! equal to 1 plus the sacrificed creature's mana value, put that card onto the battlefield, then shuffle. Activate
//! only as a sorcery."

use super::*;
use crate::card::{Cmp, Pred};

pub const HANDLERS: &[Handler] = &[Handler::effect("PrimeSpeakerVannifar.PrimeSpeakerVannifarEffect", apply)];

/// The creature the cost sacrificed (`SacrificeTargetCost.getPermanents().get(0)`), its mana value as it was; a
/// creature card of one more from the library onto the battlefield, then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(sacrificed) = cx.item.sacrificed.first().copied() else { return Some(()) };
    let mv = game.obj(sacrificed).def.mana_value() as i32 + 1;
    let filter = Filter(vec![Pred::Mv { cmp: Cmp::EqualTo, n: mv }, Pred::Type { t: CardType::Creature }]);
    let chosen = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
    if let Some(card) = chosen.first().copied() {
        if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
            return None;
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
