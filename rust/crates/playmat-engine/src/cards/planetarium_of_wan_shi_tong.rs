//! Planetarium of Wan Shi Tong: "{1}, {T}: Scry 2. Whenever you scry or surveil, look at the top card of your
//! library. You may cast that card without paying its mana cost. Do this only once each turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PlanetariumOfWanShiTong.PlanetariumOfWanShiTongEffect", apply)];

/// The top card, looked at ("Top of library"); then `CardUtil.castSpellWithAttributesForFree` on it: its parts that
/// aren't lands, "cast it without paying its mana cost?", the part if more than one, the free cast (undone if it's
/// cancelled). The trigger's own "do this only once each turn" is the engine's (`once_each_turn`).
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(&card) = game.player(you).library.first() else { return Some(()) };
    let zcc = game.obj(cx.source_id()).zcc;
    game.looked_at.push(crate::state::LookedAt {
        viewer: you,
        source: cx.source_id(),
        zcc,
        bare: false,
        cards: vec![card],
    });
    let full = game.obj(card).def.clone();
    let parts: Vec<(usize, Arc<CardDef>)> =
        castable_parts(&full).into_iter().filter(|(_, p)| !p.is(CardType::Land)).collect();
    if parts.is_empty() {
        return Some(());
    }
    let names: Vec<String> = parts.iter().map(|(_, p)| p.name.clone()).collect();
    let message = format!("Cast spell without paying its mana cost ({})?", names.join(" or "));
    if !choose_use(game, d, you, &message)? {
        return Some(());
    }
    let pick = if parts.len() == 1 {
        parts[0].0
    } else {
        let options: Vec<usize> = parts.iter().map(|(k, _)| *k).collect();
        match ask(game, d, you, &Query::ChooseAbility { source: card, abilities: options.clone() }) {
            Answer::Ability(o, k) if o == card && options.contains(&k) => k,
            Answer::Stop => return None,
            _ => return Some(()),
        }
    };
    let before = game.clone();
    let o = game.obj_mut(card);
    if !full.room.is_empty() {
        o.adventurer = Some(full.clone());
        o.def = full.room[pick].clone();
        o.room_cast = Some(pick);
    } else if let Some(back) = full.back.clone().filter(|_| pick == 1) {
        o.front = Some(full.clone());
        o.def = back;
    } else if let Some(adventure) = full.adventure.clone().filter(|_| pick == 0) {
        o.adventurer = Some(full.clone());
        o.def = adventure;
    }
    if !cast_free(game, d, you, card) {
        *game = before;
    }
    if d.stopped() {
        return None;
    }
    Some(())
}
