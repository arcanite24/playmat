//! Whip of Erebos: "{2}{B}{B}, {T}: Return target creature card from your graveyard to the battlefield. It gains
//! haste. Exile it at the beginning of the next end step. If it would leave the battlefield, exile it instead of
//! putting it anywhere else. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WhipOfErebos.WhipOfErebosEffect", apply)];

/// The card onto the battlefield (`moveCards`); the permanent gains haste for as long as it stays (`Duration.Custom`)
/// and is exiled at the beginning of the next end step (`ExileTargetEffect` with a `FixedTarget`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(card) = cx.obj(0) else { return Some(()) };
    if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    if game.obj(card).zone != Zone::Battlefield {
        return Some(());
    }
    let haste: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "GainAbilityTargetEffect",
        "ability": {"class": "HasteAbility"},
        "duration": "WhileOnBattlefield",
    }))
    .ok()?;
    cx.run_on(game, d, &[haste], vec![Target::Obj(card)]);
    let delayed = Delayed {
        rule: "At the beginning of the next end step, exile that permanent.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::ExileTargetEffect],
        source_zcc: cx.source.1,
        fixed: vec![(card, game.obj(card).zcc)],
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
