//! Ulamog's Nullifier: "When Ulamog's Nullifier enters, you may put two cards your opponents own from exile into
//! their owners' graveyards. If you do, counter target spell."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("UlamogsNullifier.UlamogsNullifierEffect", apply)];

/// With the target spell still there and two or more cards opponents own in exile: two of them
/// (`TargetCardInExile(2, 2)`, `chooseTarget`) to the graveyard, then the spell is countered
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(spell) = cx.obj(0) else { return Some(()) };
    if game.obj(spell).zone != Zone::Stack {
        return Some(());
    }
    let theirs: Vec<ObjId> = game.exile.iter().copied().filter(|c| game.obj(*c).owner != you).collect();
    if theirs.len() < 2 {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &theirs, &theirs, 2, 2, &rule)?;
    if chosen.len() < 2 {
        return Some(());
    }
    for c in chosen {
        game.move_to(c, Zone::Graveyard);
    }
    cx.run(game, d, &[EffectDef::CounterTargetEffect]);
    Some(())
}
