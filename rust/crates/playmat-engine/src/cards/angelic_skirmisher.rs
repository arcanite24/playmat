//! Angelic Skirmisher: "At the beginning of each combat, choose first strike, vigilance, or lifelink. Creatures you
//! control gain that ability until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AngelicSkirmisher.AngelicSkirmisherEffect", apply)];

/// With Angelic Skirmisher still on the battlefield, you choose one of the three (`ChoiceImpl`, required); the
/// creatures you control gain it until end of turn (`GainAbilityControlledEffect`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if game.obj(cx.source_id()).zone != Zone::Battlefield {
        return Some(());
    }
    let you = cx.controller();
    let choices: Vec<String> = ["First strike", "Vigilance", "Lifelink"].iter().map(|s| s.to_string()).collect();
    let q = Query::Choice { message: "Choose ability for your creatures".into(), choices: choices.clone() };
    let chosen = loop {
        match ask(game, d, you, &q) {
            Answer::Text(t) if choices.contains(&t) => break t,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let ability = match chosen.as_str() {
        "First strike" => AbilityDef::FirstStrikeAbility,
        "Vigilance" => AbilityDef::VigilanceAbility,
        _ => AbilityDef::LifelinkAbility,
    };
    let e = EffectDef::GainAbilityControlledEffect {
        ability: Box::new(ability),
        duration: Duration::EndOfTurn,
        filter: Filter(vec![Pred::Type { t: CardType::Creature }]),
        exclude_source: false,
    };
    cx.run(game, d, &[e]);
    Some(())
}
