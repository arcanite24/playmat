//! Tasigur, the Golden Fang: "Delve. {2}{G/U}{G/U}: Mill two cards, then return a nonland card of an opponent's
//! choice from your graveyard to your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TasigurTheGoldenFang.TasigurTheGoldenFangEffect", apply)];

/// the opponent chooses a nonland card you own in your graveyard (`TargetCardInGraveyard`, not targeted), and it
/// goes to your hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let opponent = game.opponent(you);
    let graveyard = game.player(you).graveyard.clone();
    let possible: Vec<ObjId> = graveyard.iter().copied().filter(|c| !game.obj(*c).is(CardType::Land)).collect();
    if possible.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, opponent, &possible, &possible, 1, 1, &rule)?;
    if let Some(&c) = chosen.first() {
        game.move_to(c, Zone::Hand);
    }
    Some(())
}
