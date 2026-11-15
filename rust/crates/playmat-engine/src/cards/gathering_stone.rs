//! Gathering Stone: "As this artifact enters, choose a creature type. Spells you cast of the chosen type cost {1}
//! less to cast. When this artifact enters and at the beginning of your upkeep, look at the top card of your
//! library. If it's a card of the chosen type, you may reveal it and put it into your hand. If you don't put the card
//! into your hand, you may put it into your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GatheringStone.GatheringStoneEffect", apply)];

/// The top card looked at ("Top card of library"); of the chosen type, "Reveal <it> and put it into your hand?";
/// otherwise "Put <it> into your graveyard?"
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let me = cx.source_id();
    let card = game.player(you).library.first().copied()?;
    let chosen = game.obj(me).chosen_type.clone();
    let zcc = game.obj(me).zcc;
    game.looked_at.push(crate::state::LookedAt { viewer: you, source: me, zcc, bare: false, cards: vec![card] });
    let name = game.obj(card).def.name.clone();
    if chosen.is_some_and(|t| game.has_subtype(card, &t))
        && choose_use(game, d, you, &format!("Reveal {name} and put it into your hand?"))?
    {
        game.revealed.push(crate::state::Revealed { source: me, zcc, paren: false, cards: vec![(card, Zone::Library)] });
        game.move_to(card, Zone::Hand);
        return Some(());
    }
    if choose_use(game, d, you, &format!("Put {name} into your graveyard?"))? {
        game.move_to(card, Zone::Graveyard);
    }
    Some(())
}
