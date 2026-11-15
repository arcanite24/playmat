//! Solstice Revelations: "Exile cards from the top of your library until you exile a nonland card. You may cast that
//! card without paying its mana cost if the spell's mana value is less than the number of Mountains you control. If
//! you don't cast that card this way, put it into your hand. Flashback {6}{R}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SolsticeRevelations.SolsticeRevelationsEffect", apply)];

/// Exiled one at a time down to the first nonland card; `castSpellWithAttributesForFree` with the filter "mana value
/// less than the Mountains you control" (asked only when it's castable); not cast, it goes to your hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut hit = None;
    for c in game.player(you).library.clone() {
        game.move_to(c, Zone::Exile);
        if !game.has_type(c, CardType::Land) {
            hit = Some(c);
            break;
        }
    }
    let Some(card) = hit else { return Some(()) };
    let fits = |g: &Game, c: ObjId| {
        let mountains =
            g.battlefield.iter().filter(|p| g.obj(**p).controller == you && g.has_subtype(**p, "Mountain")).count();
        (g.obj(c).def.mana_value() as usize) < mountains
    };
    let cast = super::breaching_dragonstorm::cast_for_free(game, d, you, card, fits)?;
    if !cast {
        game.move_to(card, Zone::Hand);
    }
    Some(())
}
