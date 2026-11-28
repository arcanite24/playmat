//! Nahiri, the Harbinger: "−8: Search your library for an artifact or creature card, put it onto the battlefield,
//! then shuffle. It gains haste. Return it to your hand at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NahiriTheHarbinger.NahiriTheHarbingerEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_ARTIFACT_OR_CREATURE"])];

/// Up to one artifact or creature card (`TargetCardInLibrary(0, 1, …)`) onto the battlefield; the permanent gains
/// haste for as long as it stays (`Duration.Custom`) and returns to its owner's hand at the beginning of the next end
/// step (`AtTheBeginOfNextEndStepDelayedTriggeredAbility(ReturnToHandTargetEffect)`); then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter = cx.filters.first().cloned().unwrap_or_default();
    let found = search_library(game, d, you, &filter, 0, 1, cx.source_id())?;
    if let Some(&card) = found.first() {
        if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
            return None;
        }
        if game.obj(card).zone == Zone::Battlefield {
            let haste: EffectDef = serde_json::from_value(serde_json::json!({
                "class": "GainAbilityTargetEffect",
                "ability": {"class": "HasteAbility"},
                "duration": "EndOfTurn",
            }))
            .ok()?;
            cx.run_on(game, d, &[haste], vec![Target::Obj(card)]);
            let delayed = Delayed {
                rule: "At the beginning of the next end step, return that creature to its owner's hand.".into(),
                sacrifice: Vec::new(),
                warp_exile: Vec::new(),
                effects: vec![EffectDef::ReturnToHandTargetEffect],
                source_zcc: cx.source.1,
                fixed: vec![(card, game.obj(card).zcc)],
                return_tapped: Vec::new(),
                targets: Vec::new(),
                condition: None,
            };
            game.at_next_end_step
                .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
