//! Karfell Harbinger: "{T}: Add {U}. Spend this mana only to foretell a card from your hand or cast an instant or
//! sorcery spell."

use super::*;
use crate::card::{AbilityDef, CardType};

pub const HANDLERS: &[Handler] = &[Handler::spend("KarfellHarbinger.KarfellHarbingerManaBuilder", spend)];

fn spend(game: &Game, cx: &SpendCx) -> bool {
    if cx.spell() {
        return game.has_type(cx.on, CardType::Instant) || game.has_type(cx.on, CardType::Sorcery);
    }
    matches!(cx.ability_def(game), Some(AbilityDef::ForetellAbility { .. }))
}
