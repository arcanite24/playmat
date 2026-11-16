//! Hidetsugu and Kairi: "When Hidetsugu and Kairi dies, exile the top card of your library. Target opponent loses
//! life equal to its mana value. If it's an instant or sorcery card, you may cast it without paying its mana cost."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HidetsuguAndKairi.HidetsuguAndKairiEffect", apply)];

/// the top card exiled, the life lost, then an instant or sorcery offered as `castSpellWithAttributesForFree` does
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let opponent = cx.player(0)?;
    let card = game.player(you).library.first().copied()?;
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone != Zone::Exile {
        return Some(());
    }
    let def = game.obj(card).def.clone();
    lose_life_by(game, opponent, def.mana_value() as i32, Some(cx.source_id()), false);
    if def.is(CardType::Instant) || def.is(CardType::Sorcery) {
        let message = format!("Cast spell without paying its mana cost ({})?", log::object(game, card));
        if choose_use(game, d, you, &message)? {
            cast_free(game, d, you, card);
            if d.stopped() {
                return None;
            }
        }
    }
    Some(())
}
