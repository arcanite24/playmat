//! Chandra, Pyromaster: "+1: Chandra, Pyromaster deals 1 damage to target player or planeswalker and 1 damage to up
//! to one target creature that player or that planeswalker's controller controls. That creature can't block this
//! turn. 0: Exile the top card of your library. You may play it this turn. −7: Exile the top ten cards of your
//! library. Choose an instant or sorcery card exiled this way and copy it three times. You may cast the copies
//! without paying their mana costs."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::target(
        "ChandraPyromaster.ChandraPyromasterTarget",
        "new TargetPermanent(0, 1, StaticFilters.FILTER_PERMANENT_CREATURE, false)",
        that_players,
    ),
    Handler::effect("ChandraPyromaster.ChandraPyromasterEffect1", plus_one)
        .effects(&["new CantBlockTargetEffect(Duration.EndOfTurn)"]),
    Handler::effect("ChandraPyromaster.ChandraPyromasterEffect2", zero),
    Handler::effect("ChandraPyromaster.ChandraPyromasterEffect3", minus_seven),
];

/// `possibleTargets`: with the first target chosen, a creature its player (a planeswalker's controller) controls,
/// not that planeswalker; any creature before
fn that_players(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    match game.chosen_before.first() {
        Some(Target::Player(p)) => game.obj(id).controller == *p,
        Some(Target::Obj(pw)) => id != *pw && game.obj(id).controller == game.obj(*pw).controller,
        None => true,
    }
}

/// 1 damage to the first target, then 1 to the creature, which can't block this turn
fn plus_one(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    if let Some(to) = cx.target(0) {
        deal_damage(game, src, to, 1, false);
    }
    let Some(creature) = cx.obj(1) else { return Some(()) };
    if game.obj(creature).zone != Zone::Battlefield {
        return Some(());
    }
    deal_damage(game, src, Target::Obj(creature), 1, false);
    cx.run_on(game, d, cx.effects, vec![Target::Obj(creature)]);
    Some(())
}

/// The top card exiled (no zone of the source's), playable this turn (`PlayFromNotOwnHandZoneTargetEffect`)
fn zero(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(&card) = game.player(you).library.first() else { return Some(()) };
    game.move_to(card, Zone::Exile);
    if game.obj(card).zone == Zone::Exile {
        let turn = game.turn;
        game.obj_mut(card).may_play = Some((you, turn));
    }
    Some(())
}

/// The top ten into the source's exile zone; with an instant or sorcery among them, one chosen (`chooseTarget`),
/// then three times "Cast copy N of …?": a copy (`game.copyCard`) cast without paying its mana cost
fn minus_seven(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let (src, zcc) = cx.source;
    let cards: Vec<ObjId> = game.player(you).library.iter().take(10).copied().collect();
    for c in &cards {
        game.move_to(*c, Zone::Exile);
        if game.obj(*c).zone == Zone::Exile {
            game.obj_mut(*c).exiled_with = Some((src, zcc));
        }
    }
    let spells: Vec<ObjId> = cards
        .iter()
        .copied()
        .filter(|c| {
            let o = game.obj(*c);
            o.zone == Zone::Exile && (o.is(CardType::Instant) || o.is(CardType::Sorcery))
        })
        .collect();
    if spells.is_empty() {
        return Some(());
    }
    let shown: Vec<ObjId> = cards.iter().copied().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
    let rule = item_rule(game, cx.item);
    let Some(&card) = choose_cards(game, d, you, &shown, &spells, 1, 1, &rule)?.first() else { return Some(()) };
    for i in 1..=3 {
        let message = format!("Cast copy {} of {}", i, game.obj(card).def.name);
        if !choose_use(game, d, you, &message)? {
            continue;
        }
        let def = game.obj(card).def.clone();
        let copy = game.create_obj(def, you, Zone::Exile);
        Arc::make_mut(&mut game.exile).push(copy);
        game.obj_mut(copy).token = true;
        cast_free_face(game, d, you, copy);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
