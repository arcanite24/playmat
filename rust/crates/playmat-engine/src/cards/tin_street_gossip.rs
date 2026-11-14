//! Tin Street Gossip: "{T}: Add {R}{G}. Spend this mana only to cast face-down spells or to turn creatures face up."

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] = &[Handler::spend("TinStreetGossip.TinStreetGossipManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    if cx.spell() {
        return game.obj(cx.on).face_down;
    }
    matches!(cx.ability_def(game), Some(AbilityDef::TurnFaceUpAbility { .. }))
}
