//! Divine Gambit: "Exile target artifact, creature, or enchantment an opponent controls. That player may put a
//! permanent card from their hand onto the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DivineGambit.DivineGambitEffect", apply)];

/// the target's controller (as it last was) is asked, then picks a permanent card from their hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let p = game.obj(o).controller;
    if !choose_use(game, d, p, "Put a permanent card from your hand onto the battlefield?")? {
        return Some(());
    }
    let cards: Vec<ObjId> = game
        .player(p)
        .hand
        .iter()
        .copied()
        .filter(|c| !game.has_type(*c, CardType::Instant) && !game.has_type(*c, CardType::Sorcery))
        .collect();
    if cards.is_empty() {
        return Some(());
    }
    loop {
        match ask(game, d, p, &Query::Cards { objs: cards.clone(), required: true }) {
            Answer::Obj(c) if cards.contains(&c) => {
                put_cards(game, d, p, &[c], crate::card::PutCards::Battlefield);
                return Some(());
            }
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    }
}
