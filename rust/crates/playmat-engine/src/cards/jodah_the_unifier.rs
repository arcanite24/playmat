//! Jodah, the Unifier: "Whenever you cast a legendary spell from your hand, exile cards from the top of your
//! library until you exile a legendary nonland card with lesser mana value. You may cast that card without paying
//! its mana cost. Put the rest on the bottom of your library in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("JodahTheUnifier.JodahTheUnifierEffect", apply)];

/// The spell's mana value (the cast trigger's X), then cards off the top one at a time into exile until a
/// legendary nonland one with less; that one may be cast for free (`castSpellWithAttributesForFree`), and the
/// exiled cards still in exile go to the bottom in a random order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mv = cx.item.x;
    let mut exiled = Vec::new();
    let mut hit = None;
    for c in game.player(you).library.clone() {
        exiled.push(c);
        game.move_to(c, Zone::Exile);
        let o = game.obj(c);
        if o.def.supertypes.iter().any(|s| s == "LEGENDARY") && !o.is(CardType::Land) && (o.def.mana_value() as i32) < mv
        {
            hit = Some(c);
            break;
        }
    }
    if let Some(card) = hit {
        let message = format!("Cast spell without paying its mana cost ({})?", log::object(game, card));
        if choose_use(game, d, you, &message)? {
            cast_free(game, d, you, card);
            if d.stopped() {
                return None;
            }
        }
    }
    let rest: Vec<ObjId> = exiled.into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
    if !put_cards(game, d, you, &rest, crate::card::PutCards::BottomRandom) {
        return None;
    }
    Some(())
}
