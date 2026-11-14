//! Sin Prodder: "At the beginning of your upkeep, reveal the top card of your library. Any opponent may have you put
//! that card into your graveyard. If a player does, Sin Prodder deals damage to that player equal to that card's mana
//! value. Otherwise, put that card into your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SinProdder.SinProdderEffect", apply)];

/// The reveal, each opponent asked in turn until one says yes (dealt the damage before the card moves)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(&card) = game.player(you).library.first() else { return Some(()) };
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc: cx.source.1,
        paren: false,
        cards: vec![(card, Zone::Library)],
    });
    let message = format!("Put {} in {}'s graveyard?", game.obj(card).def.name, log::seat_name(you));
    let mut to_graveyard = false;
    let seats = game.seats();
    for p in (1..seats).map(|k| (you + k) % seats) {
        if choose_use(game, d, p, &message)? {
            to_graveyard = true;
            let mv = game.obj(card).def.mana_value() as i32;
            deal_damage(game, cx.source_id(), Target::Player(p), mv, false);
            break;
        }
    }
    if game.obj(card).zone == Zone::Library {
        game.move_to(card, if to_graveyard { Zone::Graveyard } else { Zone::Hand });
    }
    Some(())
}
