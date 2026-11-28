//! Madcap Experiment: "Reveal cards from the top of your library until you reveal an artifact card. Put that card
//! onto the battlefield and the rest on the bottom of your library in a random order. Madcap Experiment deals damage
//! to you equal to the number of cards revealed this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MadcapExperiment.MadcapExperimentEffect", apply)];

/// The cards down to the first artifact card (all of them when there's none) revealed; the artifact onto the
/// battlefield, the rest to the bottom in a random order, then damage to you for each card revealed
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut revealed = Vec::new();
    let mut artifact = None;
    for c in game.player(you).library.iter().copied() {
        revealed.push(c);
        if game.has_type(c, CardType::Artifact) {
            artifact = Some(c);
            break;
        }
    }
    if !revealed.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let shown = revealed.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: shown });
    }
    if let Some(a) = artifact {
        if !put_cards(game, d, you, &[a], crate::card::PutCards::Battlefield) {
            return None;
        }
    }
    let damage = revealed.len() as i32;
    let rest: Vec<ObjId> = revealed.into_iter().filter(|c| Some(*c) != artifact).collect();
    if !put_cards(game, d, you, &rest, crate::card::PutCards::BottomRandom) {
        return None;
    }
    deal_damage(game, cx.source_id(), Target::Player(you), damage, false);
    Some(())
}
