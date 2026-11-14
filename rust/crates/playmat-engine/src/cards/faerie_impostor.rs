//! Faerie Impostor: "Flying. When Faerie Impostor enters, sacrifice it unless you return another creature you control
//! to its owner's hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FaerieImpostor.FaerieImpostorEffect", apply)];

/// With another creature you control, you may (`chooseUse`) return one (`chooseTarget`, not targeted); otherwise the
/// Impostor, if it's still there, is sacrificed
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let others: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| *o != src && game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    if !others.is_empty() && choose_use(game, d, you, "Return another creature you control to its owner's hand?")? {
        let rule = item_rule(game, cx.item);
        if let Some(&o) = choose_cards(game, d, you, &others, &others, 1, 1, &rule)?.first() {
            put_cards(game, d, you, &[o], crate::card::PutCards::Hand);
            if d.stopped() {
                return None;
            }
            return Some(());
        }
    }
    if cx.source_on_battlefield(game) {
        sacrifice(game, src);
    }
    Some(())
}
