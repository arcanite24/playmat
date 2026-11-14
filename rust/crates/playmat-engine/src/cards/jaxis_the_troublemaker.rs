//! Jaxis, the Troublemaker: "{R}, {T}, Discard a card: Create a token that's a copy of another target creature you
//! control. It gains haste and "When this creature dies, draw a card." Sacrifice it at the beginning of the next end
//! step. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("JaxisTheTroublemaker.JaxisTheTroublemakerEffect", apply)];

/// `CreateTokenCopyTargetEffect` with haste and the dies trigger added to the copy (`addAdditionalAbilities`), then
/// `sacrificeTokensCreatedAtNextEndStep`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(of) = cx.obj(0) else { return Some(()) };
    if game.obj(of).zone != Zone::Battlefield {
        return Some(());
    }
    let dies: AbilityDef = serde_json::from_value(serde_json::json!({
        "class": "TriggeredAbility",
        "effects": [{"amount": 1, "class": "DrawCardSourceControllerEffect"}],
        "on": {"e": "dies"},
        "optional": false,
        "rule": "When this creature dies, draw a card.",
    }))
    .ok()?;
    let mods = move |def: &mut CardDef| {
        if !def.abilities.contains(&AbilityDef::HasteAbility) {
            def.abilities.push(AbilityDef::HasteAbility);
        }
        def.abilities.push(dies.clone());
    };
    let made = token_copy_modified(game, d, of, you, 1, &mods);
    if d.stopped() {
        return None;
    }
    let delayed = Delayed {
        rule: "At the beginning of the next end step, sacrifice the token copies.".into(),
        sacrifice: made.iter().map(|t| (*t, game.obj(*t).zcc)).collect(),
        warp_exile: Vec::new(),
        effects: Vec::new(),
        source_zcc: 0,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
