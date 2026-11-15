//! Covert Technician: "Ninjutsu {1}{U}. Whenever this creature deals combat damage to a player, you may put an
//! artifact card with mana value less than or equal to that damage from your hand onto the battlefield."

use super::*;
use crate::card::{Cmp, Filter, Pred};

pub const HANDLERS: &[Handler] = &[Handler::effect("CovertTechnician.CovertTechnicianEffect", apply)];

/// `PutCardFromHandOntoBattlefieldEffect` for an artifact card with mana value at most the damage (the trigger's X)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let filter =
        Filter::of(vec![Pred::Type { t: CardType::Artifact }, Pred::Mv { cmp: Cmp::FewerThan, n: cx.item.x + 1 }]);
    let e = EffectDef::PutCardFromHandOntoBattlefieldEffect { filter, tapped: false, attacking: false };
    cx.run(game, d, &[e]);
    if d.stopped() {
        return None;
    }
    Some(())
}
