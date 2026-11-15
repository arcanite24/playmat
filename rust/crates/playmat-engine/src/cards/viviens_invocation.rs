//! Vivien's Invocation: "Look at the top seven cards of your library. You may put a creature card from among them
//! onto the battlefield. Put the rest on the bottom of your library in a random order. When a creature is put onto
//! the battlefield this way, it deals damage equal to its power to target creature an opponent controls."

use super::*;
use crate::card::TargetDef;

const DAMAGE: &str = "ViviensInvocation.ViviensInvocationDamageEffect";

pub const HANDLERS: &[Handler] = &[
    Handler::effect("ViviensInvocation.ViviensInvocationEffect", apply)
        .filters(&["StaticFilters.FILTER_OPPONENTS_PERMANENT_CREATURE"]),
    Handler::effect(DAMAGE, damage),
];

/// The top seven: a creature card among them chosen (`TargetCard`, not targeted, required while there's one) and
/// put onto the battlefield; with it there, the reflexive trigger waits; then the rest go to the bottom in a random
/// order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards: Vec<ObjId> = game.player(you).library.iter().take(7).copied().collect();
    if cards.is_empty() {
        return Some(());
    }
    let creatures: Vec<ObjId> = cards.iter().copied().filter(|c| game.obj(*c).is(CardType::Creature)).collect();
    let card = if creatures.is_empty() {
        None
    } else {
        choose_cards(game, d, you, &cards, &creatures, 1, 1, "creature card to put on the battlefield")?.first().copied()
    };
    let Some(card) = card else {
        put_on_library(game, d, you, &cards, false, false);
        return (!d.stopped()).then_some(());
    };
    put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield);
    if d.stopped() {
        return None;
    }
    if game.obj(card).zone == Zone::Battlefield {
        cards.retain(|c| *c != card);
        let effect: EffectDef =
            serde_json::from_value(serde_json::json!({"class": "CardEffect", "handler": DAMAGE})).ok()?;
        let delayed = Delayed {
            rule: "It deals damage equals to its power to target creature an opponent controls.".into(),
            sacrifice: Vec::new(),
            warp_exile: Vec::new(),
            effects: vec![effect],
            source_zcc: cx.source.1,
            fixed: vec![(card, game.obj(card).zcc)],
            return_tapped: Vec::new(),
            targets: vec![TargetDef::TargetPermanent {
                filter: cx.filters.first()?.clone(),
                min: 1,
                max: 1,
                distribute: None,
                x_count: false,
                count: None,
            }],
            condition: None,
        };
        game.pending
            .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    }
    let rest: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    put_on_library(game, d, you, &rest, false, false);
    (!d.stopped()).then_some(())
}

/// The creature put onto the battlefield (still that permanent) deals damage equal to its power to the target
fn damage(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let target = cx.obj(0)?;
    let &(creature, zcc) = cx.item.delayed.as_ref()?.fixed.first()?;
    if game.obj(target).zone != Zone::Battlefield
        || game.obj(creature).zone != Zone::Battlefield
        || game.obj(creature).zcc != zcc
    {
        return None;
    }
    let power = game.power_toughness(creature).0;
    deal_damage(game, creature, Target::Obj(target), power, false);
    Some(())
}
