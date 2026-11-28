//! Prishe's Wanderings: "Search your library for a basic land card or Town card, put it onto the battlefield
//! tapped, then shuffle. When you search your library this way, put a +1/+1 counter on target creature you
//! control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PrishesWanderings.PrishesWanderingsEffect", apply)
    .filters(&["filter"])
    .effects(&["new AddCountersTargetEffect(CounterType.P1P1.createInstance())"])];

/// The search (`TargetCardInLibrary(filter)`: one); having searched, the reflexive trigger (`ReflexiveTriggeredAbility`
/// with `TargetControlledCreaturePermanent`) waits for the stack; the card found is revealed and put onto the
/// battlefield tapped, then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let found = search_library(game, d, you, &cx.filters[0], 1, 1, cx.source_id())?;
    let target: crate::card::TargetDef = serde_json::from_value(serde_json::json!({
        "class": "TargetPermanent", "filter": [{"p": "type", "t": "CREATURE"}, {"p": "controller", "who": "you"}]
    }))
    .ok()?;
    let delayed = crate::state::Delayed {
        rule: "put a +1/+1 counter on target creature you control.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects.to_vec(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    if let Some(card) = found.first().copied() {
        let zcc = game.obj(cx.source_id()).zcc;
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: vec![(card, Zone::Library)] });
        if !put_cards(game, d, you, &[card], crate::card::PutCards::BattlefieldTapped) {
            return None;
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
