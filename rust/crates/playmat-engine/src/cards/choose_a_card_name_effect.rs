//! `ChooseACardNameEffect(TypeOfName, lookAtOpponentHand)`: "choose a nonland card name" (Phyrexian Revoker,
//! Slaughter Games…), and "look at an opponent's hand, then choose any card name" (Sorcerous Spyglass). The forms
//! `LAND_NAME` and `ALL` without a look are the engine's `ChooseLandNameEffect`. The name is kept on the source
//! (`chosen_name`): the permanent entering, or the spell resolving, for its next effect.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ChooseACardNameEffect", apply).args(&["text", "text"])];

/// `TypeOfName.getMessage`, first letter upper case
fn message(kind: &str) -> Option<&'static str> {
    Some(match kind.trim().rsplit('.').next()? {
        "ALL" => "Choose a card name",
        "LAND_NAME" => "Choose a land card name",
        "NOT_BASIC_LAND_NAME" => "Choose a card name other than a basic land card name",
        "NONBASIC_LAND_NAME" => "Choose a nonbasic land card name",
        "NON_ARTIFACT_AND_NON_LAND_NAME" => "Choose a nonartifact, nonland card name",
        "NON_LAND_AND_NON_CREATURE_NAME" => "Choose a nonland and non creature name",
        "NON_LAND_NAME" => "Choose a nonland card name",
        "CREATURE_NAME" => "Choose a creature card name",
        "ARTIFACT_NAME" => "Choose an artifact card name",
        _ => return None,
    })
}

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let kind = cx.args.first().map(String::as_str).unwrap_or("");
    let message = message(kind).unwrap_or("Choose a card name");
    let you = cx.controller();
    let (source, zcc) = cx.source;
    if cx.args.get(1).is_some_and(|a| a.trim() == "true") {
        // `TargetOpponent(true)`, chosen even with one opponent; the hand looked at, then "Press Ok to name a card"
        let q = Query::Target { players: vec![game.opponent(you)], objs: Vec::new(), required: true };
        let opponent = loop {
            match ask(game, d, you, &q) {
                Answer::Player(p) => break p,
                Answer::Stop => return None,
                _ => {}
            }
            if d.stopped() {
                return None;
            }
        };
        let cards = game.player(opponent).hand.clone();
        if !cards.is_empty() {
            game.looked_at.push(crate::state::LookedAt { viewer: you, source, zcc, bare: false, cards });
        }
        choose_use(game, d, you, "Press Ok to name a card")?;
    }
    let q = Query::Choice { message: message.into(), choices: Vec::new() };
    let chosen = loop {
        match ask(game, d, you, &q) {
            Answer::Text(t) => break t,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    if matches!(game.obj(source).zone, Zone::Battlefield | Zone::Stack) {
        game.obj_mut(source).chosen_name = Some(chosen);
    }
    Some(())
}
