//! Rhystic Study: "Whenever an opponent casts a spell, you may draw a card unless that player pays {1}."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[Handler::effect("RhysticStudy.RhysticStudyDrawEffect", apply)];

/// You're asked first ("Draw a card (…)"); then the caster (the trigger's pointer) is asked "Pay {1}?" and pays; you
/// draw unless they paid
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(opponent) = cx.player(0) else { return Some(()) };
    let name = log::object(game, cx.source_id());
    if !choose_use(game, d, you, &format!("Draw a card ({name})"))? {
        return Some(());
    }
    if choose_use(game, d, opponent, "Pay {1}?")? {
        let paid = pay_mana(game, d, opponent, &ManaCost::parse("{1}").ok()?);
        if d.stopped() {
            return None;
        }
        if paid {
            return Some(());
        }
    }
    draw(game, you, 1);
    Some(())
}
