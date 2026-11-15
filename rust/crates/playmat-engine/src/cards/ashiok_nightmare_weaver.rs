//! Ashiok, Nightmare Weaver: "+2: Exile the top three cards of target opponent's library.", "−X: Put a creature card
//! with mana value X exiled with Ashiok, Nightmare Weaver onto the battlefield under your control. That creature is a
//! Nightmare in addition to its other types." and "−10: Exile all cards from all opponents' hands and graveyards."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("AshiokNightmareWeaver.AshiokNightmareWeaverExileEffect", exile_top),
    Handler::effect("AshiokNightmareWeaver.AshiokNightmareWeaverPutIntoPlayEffect", put_into_play)
        .effects(&["new AddCardSubTypeTargetEffect(SubType.NIGHTMARE, Duration.EndOfTurn)"]),
    Handler::effect("AshiokNightmareWeaver.AshiokNightmareWeaverExileAllEffect", exile_all),
];

/// `CardUtil.getExileZoneId(game, source)`: the source as it is now
fn zone(game: &Game, cx: &EffectCx) -> (ObjId, u32) {
    (cx.source_id(), game.obj(cx.source_id()).zcc)
}

/// the cards moved to exile together, those now in exile in this object's exile zone
fn exile_into(game: &mut Game, cards: &[ObjId], zone: (ObjId, u32)) {
    for c in cards {
        game.move_to(*c, Zone::Exile);
    }
    for c in cards {
        if game.obj(*c).zone == Zone::Exile {
            game.obj_mut(*c).exiled_with = Some(zone);
        }
    }
}

/// the target opponent's top three cards, exiled with this
fn exile_top(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let top: Vec<ObjId> = game.player(p).library.iter().take(3).copied().collect();
    let z = zone(game, cx);
    exile_into(game, &top, z);
    Some(())
}

/// a creature card with mana value X (the X paid) from this object's exile zone (`chooseTarget`, required when one
/// can be chosen) onto the battlefield under your control, then a Nightmare until end of turn (as Java has it)
fn put_into_play(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = cx.item.x;
    let z = zone(game, cx);
    let possible: Vec<ObjId> = game
        .exile
        .iter()
        .copied()
        .filter(|c| {
            let o = game.obj(*c);
            o.exiled_with == Some(z) && o.is(CardType::Creature) && o.def.mana_value() as i32 == x
        })
        .collect();
    if possible.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(card) = choose_cards(game, d, you, &possible, &possible, 1, 1, &rule)?.first().copied() else {
        return Some(());
    };
    if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    if game.obj(card).zone == Zone::Battlefield {
        cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(card)]);
    }
    Some(())
}

/// every opponent's hand and graveyard, exiled together with this
fn exile_all(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards = Vec::new();
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        cards.extend(game.player(p).hand.iter().copied());
        cards.extend(game.player(p).graveyard.iter().copied());
    }
    let z = zone(game, cx);
    exile_into(game, &cards, z);
    Some(())
}
