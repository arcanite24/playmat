//! Oblivion Sower: "When you cast this spell, target opponent exiles the top four cards of their library, then you
//! may put any number of land cards that player owns from exile onto the battlefield under your control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OblivionSower.OblivionSowerEffect", apply)];

/// The land cards in exile the target player owns (face up), if any: "Put lands into play?", then any number of
/// them (`TargetCard(0, n)`), onto the battlefield under you together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(p) = cx.player(0) else { return Some(()) };
    let lands: Vec<ObjId> = game
        .exile
        .iter()
        .copied()
        .filter(|c| game.obj(*c).owner == p && !game.obj(*c).face_down && game.has_type(*c, CardType::Land))
        .collect();
    if lands.is_empty() || !choose_use(game, d, you, "Put lands into play?")? {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &lands, &lands, 0, lands.len(), &rule)?;
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}
