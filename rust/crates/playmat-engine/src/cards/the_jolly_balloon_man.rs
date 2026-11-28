//! The Jolly Balloon Man: "{1}, {T}: Create a token that's a copy of another target creature you control, except it's
//! a 1/1 red Balloon creature in addition to its other colors and types and it has flying and haste. Sacrifice it at
//! the beginning of the next end step. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheJollyBalloonMan.TheJollyBalloonManEffect", apply)];

/// `CreateTokenCopyTargetEffect(null, CREATURE, haste, 1, …, 1, 1, flying)` with red added and the Balloon type, then
/// `sacrificeTokensCreatedAtNextEndStep`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(of) = cx.obj(0) else { return Some(()) };
    if game.obj(of).zone != Zone::Battlefield {
        return Some(());
    }
    let mods = |def: &mut CardDef| {
        if !def.types.contains(&CardType::Creature) {
            def.types.push(CardType::Creature);
        }
        if !def.abilities.contains(&AbilityDef::HasteAbility) {
            def.abilities.push(AbilityDef::HasteAbility);
        }
        if !def.abilities.contains(&AbilityDef::FlyingAbility) {
            def.abilities.push(AbilityDef::FlyingAbility);
        }
        def.remove_pt_cda();
        def.power = 1;
        def.toughness = 1;
        if !def.subtypes.iter().any(|s| s == "Balloon") {
            def.subtypes.push("Balloon".into());
        }
        if !def.colors.contains(&Color::R) {
            def.colors.push(Color::R);
        }
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
