//! `ChooseOpponentEffect`: "as this enters, choose an opponent" (Nyxathid), and Nyxathid's
//! `CardsInChosenPlayerHandCount`, "for each card in the chosen player's hand".

use super::*;
use crate::card::TargetDef;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("ChooseOpponentEffect", choose).args(&["one_of:Outcome.Detriment|Outcome.Benefit|Outcome.Neutral"]),
    Handler::value("Nyxathid.CardsInChosenPlayerHandCount", chosen_hand),
];

/// the mark of the chosen player on the permanent (`getValue(sourceId + "_player")`)
const MARK: &str = "chosen_player:";

/// A `TargetOpponent` chosen (`Player.choose`), then the permanent's "Chosen player: …" info line
fn choose(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let id = cx.source_id();
    let chosen = if target_possible(game, you, &TargetDef::TargetOpponent, Some(id)) {
        choose_target(game, d, you, &TargetDef::TargetOpponent, Some(id), "", false)
    } else {
        None
    };
    if d.stopped() {
        return None;
    }
    let Some(Target::Player(p)) = chosen.as_ref().and_then(|c| c.first()).copied() else { return Some(()) };
    let name = log::player(p);
    let o = game.obj_mut(id);
    o.marks.retain(|m| !m.starts_with(MARK));
    o.marks.push(format!("{MARK}{p}"));
    o.add_info("chosen opponent", &format!("Chosen player: {name}"));
    Some(())
}

/// the number of cards in the hand of the player chosen as the source entered (0 without one)
fn chosen_hand(game: &Game, _you: Seat, source: ObjId, _: i32) -> i32 {
    game.obj(source)
        .marks
        .iter()
        .find_map(|m| m.strip_prefix(MARK)?.parse::<Seat>().ok())
        .map_or(0, |p| game.player(p).hand.len() as i32)
}
