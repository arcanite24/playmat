//! The Key to the Vault: "Whenever equipped creature deals combat damage to a player, look at that many cards from
//! the top of your library. You may exile a nonland card from among them. Put the rest on the bottom of your library
//! in a random order. You may cast the exiled card without paying its mana cost. Equip {2}{U}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheKeyToTheVault.TheKeyToTheVaultEffect", apply)];

/// The top X cards (X the damage): up to one nonland card among them chosen and exiled, the rest to the bottom in a
/// random order, then `castSpellWithAttributesForFree` of the exiled card
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = cx.item.x.max(0) as usize;
    let top: Vec<ObjId> = game.player(you).library.iter().take(n).copied().collect();
    if top.is_empty() {
        return Some(());
    }
    let nonland: Vec<ObjId> = top.iter().copied().filter(|c| !game.has_type(*c, CardType::Land)).collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &top, &nonland, 0, 1, &rule)?.first().copied();
    if let Some(c) = chosen {
        game.move_to(c, Zone::Exile);
    }
    let rest: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !put_on_library(game, d, you, &rest, false, false) {
        return None;
    }
    if let Some(c) = chosen.filter(|c| game.obj(*c).zone == Zone::Exile) {
        super::breaching_dragonstorm::cast_for_free(game, d, you, c, |_, _| true)?;
    }
    Some(())
}
