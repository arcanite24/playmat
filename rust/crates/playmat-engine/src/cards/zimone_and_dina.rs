//! Zimone and Dina: "{T}, Sacrifice another creature: Draw a card. You may put a land card from your hand onto the
//! battlefield tapped. If you control eight or more lands, repeat this process once."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ZimoneAndDina.ZimoneAndDinaEffect", apply)];

/// The draw and the land (`TargetCardInHand(0, 1, land)` with `Player.choose`, onto the battlefield tapped), again
/// with eight or more lands
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    draw_and_land(game, d, cx, you)?;
    let lands = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Land))
        .count();
    if lands >= 8 {
        draw_and_land(game, d, cx, you)?;
    }
    Some(())
}

fn draw_and_land(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx, you: Seat) -> Option<()> {
    draw(game, you, 1);
    let hand = game.player(you).hand.clone();
    let lands: Vec<ObjId> = hand.iter().copied().filter(|c| game.obj(*c).is(CardType::Land)).collect();
    let rule = item_rule(game, cx.item);
    if let Some(&c) = choose_cards(game, d, you, &hand, &lands, 0, 1, &rule)?.first() {
        if !put_cards(game, d, you, &[c], crate::card::PutCards::BattlefieldTapped) {
            return None;
        }
    }
    Some(())
}
