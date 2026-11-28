//! Brain in a Jar: "{1}, {T}: Put a charge counter on Brain in a Jar, then you may cast an instant or sorcery spell
//! with mana value equal to the number of charge counters on Brain in a Jar from your hand without paying its mana
//! cost."

use super::*;
use crate::card::{Cmp, Filter, Pred};

pub const HANDLERS: &[Handler] = &[Handler::effect("BrainInAJar.BrainInAJarCastEffect", apply)];

/// `castSpellWithAttributesForFree` over the hand: an instant or sorcery card whose mana value is the charge counters
/// on the Brain now (`getSourcePermanentOrLKI`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let counters = game.obj(cx.source_id()).counter("charge");
    let filter = Filter::of(vec![
        Pred::Or { xs: vec![Pred::Type { t: CardType::Instant }, Pred::Type { t: CardType::Sorcery }] },
        Pred::Mv { cmp: Cmp::EqualTo, n: counters },
    ]);
    if !cast_one_for_free(game, d, you, cx.source_id(), &filter) {
        return None;
    }
    Some(())
}
