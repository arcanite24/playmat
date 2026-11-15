//! Auspicious Starrix: "Whenever this creature mutates, exile cards from the top of your library until you exile X
//! permanent cards, where X is the number of times this creature has mutated. Put those permanent cards onto the
//! battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AuspiciousStarrix.AuspiciousStarrixEffect", apply)];

/// X is `SourceMutatedCount` (the source as it last was); the cards down to the Xth permanent card are exiled
/// together, then the permanent ones put onto the battlefield together (`moveCards`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = game.obj(cx.source_id()).merged.len();
    let mut exiled = Vec::new();
    let mut permanents = Vec::new();
    for c in game.player(you).library.clone() {
        if game.obj(c).def.is_permanent_card() {
            permanents.push(c);
        }
        exiled.push(c);
        if permanents.len() >= x {
            break;
        }
    }
    if !exiled.is_empty() {
        put_cards(game, d, you, &exiled, crate::card::PutCards::Exiled);
    }
    if d.stopped() {
        return None;
    }
    let permanents: Vec<ObjId> = permanents.into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
    if !permanents.is_empty() {
        put_cards(game, d, you, &permanents, crate::card::PutCards::Battlefield);
    }
    if d.stopped() {
        return None;
    }
    Some(())
}
