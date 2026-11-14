//! Sphere of Annihilation: "This artifact enters with X void counters on it. {T}, sacrifice ... : Exile Sphere of
//! Annihilation, all creatures and planeswalkers with mana value less than or equal to the number of void counters on
//! it, and all creature and planeswalker cards in graveyards with mana value less than or equal to the number of void
//! counters on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SphereOfAnnihilation.SphereOfAnnihilationEffect", apply)];

/// The void counters as the Sphere was last seen; it, the creatures and planeswalkers (battlefield order), then the
/// creature and planeswalker cards of each graveyard (players from you in turn order) are exiled together
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let counters = game.obj(src).counter("void");
    let small = |g: &Game, o: ObjId| {
        (g.has_type(o, CardType::Creature) || g.has_type(o, CardType::Planeswalker))
            && g.obj(o).def.mana_value() as i32 <= counters
    };
    let mut cards = Vec::new();
    if cx.source_on_battlefield(game) {
        cards.push(src);
    }
    cards.extend(game.battlefield.iter().copied().filter(|o| *o != src && small(game, *o)));
    for k in 0..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let yard = game.player(p).graveyard.clone();
        cards.extend(yard.into_iter().filter(|c| {
            let def = &game.obj(*c).def;
            (def.is(CardType::Creature) || def.is(CardType::Planeswalker)) && def.mana_value() as i32 <= counters
        }));
    }
    for c in cards {
        game.move_to(c, Zone::Exile);
    }
    Some(())
}
