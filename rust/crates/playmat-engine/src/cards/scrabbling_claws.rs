//! Scrabbling Claws: "{T}: Target player exiles a card from their graveyard. {1}, Sacrifice Scrabbling Claws: Exile
//! target card from a graveyard. Draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ScrabblingClaws.ScrabblingClawsEffect", apply)];

/// The player chooses (`chooseTarget`, `TargetCardInGraveyard` of their own) a card of their graveyard to exile;
/// nothing there, nothing happens
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let yard = game.player(p).graveyard.clone();
    if yard.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    if let Some(&c) = choose_cards(game, d, p, &yard, &yard, 1, 1, &rule)?.first() {
        game.move_to(c, Zone::Exile);
    }
    Some(())
}
