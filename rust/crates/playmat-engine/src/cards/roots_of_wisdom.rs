//! Roots of Wisdom: "Mill three cards, then return a land card or Elf card from your graveyard to your hand. If you
//! can't, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RootsOfWisdom.RootsOfWisdomEffect", apply)
    .effects(&["new MillCardsControllerEffect(3)"])];

/// The mill; with a land or Elf card in your graveyard, one of them (`TargetCardInYourGraveyard`, not targeted) to
/// your hand; else a draw
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    cx.run_on(game, d, &cx.effects[..1], Vec::new());
    if d.stopped() {
        return None;
    }
    let possible: Vec<ObjId> = game
        .player(you)
        .graveyard
        .iter()
        .copied()
        .filter(|c| game.has_type(*c, CardType::Land) || game.has_subtype(*c, "Elf"))
        .collect();
    if !possible.is_empty() {
        let rule = item_rule(game, cx.item);
        if let Some(c) = choose_cards(game, d, you, &possible, &possible, 1, 1, &rule)?.first().copied() {
            game.move_to(c, Zone::Hand);
            if game.obj(c).zone == Zone::Hand {
                return Some(());
            }
        }
    }
    draw(game, you, 1);
    Some(())
}
