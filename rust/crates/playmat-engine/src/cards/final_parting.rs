//! Final Parting: "Search your library for two cards. Put one into your hand and the other into your graveyard. Then
//! shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FinalParting.FinalPartingEffect", apply)];

/// Two found: `choose(searched, TargetCard)` for the hand, the other to the graveyard; one found: to the hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let found = search_library(game, d, you, &Filter(Vec::new()), 2, 2, cx.source_id())?;
    match found.as_slice() {
        [one] => game.move_to(*one, Zone::Hand),
        [_, _] => {
            let rule = item_rule(game, cx.item);
            let chosen = choose_cards(game, d, you, &found, &found, 1, 1, &rule)?;
            if let Some(&card) = chosen.first() {
                game.move_to(card, Zone::Hand);
            }
            if let Some(&other) = found.iter().find(|c| !chosen.contains(c)) {
                game.move_to(other, Zone::Graveyard);
            }
        }
        _ => {}
    }
    cx.run(game, d, &[EffectDef::ShuffleLibrarySourceEffect]);
    Some(())
}
