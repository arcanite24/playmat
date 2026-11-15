//! The Tale of Tamiyo: "(As this Saga enters and after your draw step, add a lore counter.) I, II, III — Mill two
//! cards. If two cards that share a card type were milled this way, draw a card and repeat this process. IV — Exile
//! any number of target instant, sorcery, and/or Tamiyo planeswalker cards from your graveyard. Copy them. You may
//! cast any number of the copies."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("TheTaleOfTamiyo.TheTaleOfTamiyoEffect1", mill_and_draw),
    Handler::effect("TheTaleOfTamiyo.TheTaleOfTamiyoEffect2", copy_and_cast),
];

/// Mill two; with both milled (in the graveyard) sharing a card type, draw a card and go again
fn mill_and_draw(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    loop {
        let top: Vec<ObjId> = game.player(you).library.iter().take(2).copied().collect();
        if !top.is_empty() {
            note(game, EventKind::Milled, Some(you), None, None, top.len() as i32);
        }
        for c in &top {
            game.move_to(*c, Zone::Graveyard);
        }
        let milled: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
        if milled.len() < 2 {
            break;
        }
        let (a, b) = (&game.obj(milled[0]).def.types, &game.obj(milled[1]).def.types);
        if !b.iter().any(|t| a.contains(t)) {
            break;
        }
        draw(game, you, 1);
        if d.stopped() {
            return None;
        }
        if game.player(you).lost {
            break;
        }
    }
    Some(())
}

/// The targets still in your graveyard exiled together; then for each in exile, in target order, "Cast copy of
/// <name>?" and, if so, a copy (`game.copyCard`, leaving nothing behind) cast paying its costs
fn copy_and_cast(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = cx.objs(0).into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    for c in cards {
        if game.obj(c).zone != Zone::Exile {
            continue;
        }
        let def = game.obj(c).def.clone();
        if !choose_use(game, d, you, &format!("Cast copy of {}?", def.name))? {
            continue;
        }
        let copy = game.create_obj(def, you, Zone::Exile);
        Arc::make_mut(&mut game.exile).push(copy);
        game.obj_mut(copy).token = true;
        let before = game.clone();
        if !crate::rules::cast(game, d, you, copy) {
            if d.stopped() {
                return None;
            }
            *game = before;
        }
    }
    Some(())
}
