//! Mythos of Brokkos: "If {U}{B} was spent to cast this spell, search your library for a card, put that card into
//! your graveyard, then shuffle. Return up to two permanent cards from your graveyard to your hand."

use super::*;
use crate::card::{Color, Filter};

pub const HANDLERS: &[Handler] = &[Handler::effect("MythosOfBrokkos.MythosOfBrokkosEffect", apply)];

/// With blue and black spent (`ManaWasSpentCondition.BLUE` and `BLACK`), `TargetCardInLibrary()` searched for, put
/// into your graveyard, then the shuffle; then up to two permanent cards chosen from your graveyard (not targeted)
/// go to your hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let spell = game.obj(cx.item.card);
    let spent = cx.item.ability.is_none()
        && spell.zone == Zone::Stack
        && spell.spent_colors[Color::U.index()] > 0
        && spell.spent_colors[Color::B.index()] > 0;
    if spent {
        let found = search_library(game, d, you, &Filter::default(), 1, 1, cx.source_id())?;
        if let Some(&card) = found.first() {
            game.move_to(card, Zone::Graveyard);
        }
        cx.run(game, d, &[EffectDef::ShuffleLibrarySourceEffect]);
        if d.stopped() {
            return None;
        }
    }
    let shown = game.player(you).graveyard.clone();
    let possible: Vec<ObjId> = shown.iter().copied().filter(|c| game.obj(*c).def.is_permanent_card()).collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &shown, &possible, 0, 2, &rule)?;
    for c in chosen {
        game.move_to(c, Zone::Hand);
    }
    Some(())
}
