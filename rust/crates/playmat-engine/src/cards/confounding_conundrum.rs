//! Confounding Conundrum: "When Confounding Conundrum enters the battlefield, draw a card. Whenever a land enters
//! the battlefield under an opponent's control, if that player had another land enter the battlefield under their
//! control this turn, they return a land they control to its owner's hand."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("ConfoundingConundrum.ConfoundingConundrumCondition", another_land),
    Handler::effect("ConfoundingConundrum.ConfoundingConundrumEffect", return_land),
];

/// `ConfoundingConundrumWatcher.checkPlayer` for the entering land's controller (the trigger's player pointer): more
/// than one land entered under their control this turn
fn another_land(game: &Game, _: Seat, _: ObjId) -> bool {
    let Some(Target::Player(p)) = game.checking_item.as_ref().and_then(|t| t.targets.first()?.first().copied()) else {
        return false;
    };
    game.watch.entered_permanents.iter().filter(|(o, _, c)| *c == p && game.obj(*o).def.is(CardType::Land)).count() > 1
}

/// That player chooses a land they control (`TargetPermanent`, not targeted; nothing when they have none) and puts
/// it into its owner's hand
fn return_land(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let lands: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Land))
        .collect();
    if lands.is_empty() {
        return Some(());
    }
    let chosen = choose_cards(game, d, p, &lands, &lands, 1, 1, "return to hand")?;
    if let Some(&c) = chosen.first() {
        game.move_to(c, Zone::Hand);
    }
    Some(())
}
