//! Yenna, Redtooth Regent: "{2}, {T}: Choose target enchantment you control that doesn't have the same name as
//! another permanent you control. Create a token that's a copy of it, except it isn't legendary. If the token is an
//! Aura, untap Yenna, Redtooth Regent, then scry 2. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::predicate("YennaRedtoothRegent.YennaRedtoothRegentPredicate", unique_name),
    Handler::effect("YennaRedtoothRegent.YennaRedtoothRegentEffect", apply),
];

/// no other permanent the player controls has the same name (`CardUtil.haveSameNames`: an empty name matches none)
fn unique_name(game: &Game, you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let name = game.obj(id).name().to_string();
    if name.is_empty() {
        return true;
    }
    !game.battlefield.iter().any(|o| *o != id && game.obj(*o).controller == you && game.obj(*o).name() == name)
}

/// A token copy of the target that isn't legendary (`CreateTokenCopyTargetEffect` with a permanent modifier); if
/// one made is an Aura, Yenna (still on the battlefield) untaps, then you scry 2
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(target) = cx.obj(0) else { return Some(()) };
    if game.obj(target).zone != Zone::Battlefield {
        return Some(());
    }
    let made = token_copy_modified(game, d, target, you, 1, &|def: &mut CardDef| {
        def.supertypes.retain(|s| s != "LEGENDARY");
    });
    if d.stopped() {
        return None;
    }
    let aura = made.iter().any(|t| game.obj(*t).zone == Zone::Battlefield && game.has_subtype(*t, "Aura"));
    if !aura {
        return Some(());
    }
    if cx.source_on_battlefield(game) {
        untap_permanent(game, cx.source_id());
    }
    let rule = item_rule(game, cx.item);
    if !scry(game, d, you, 2, &rule) {
        return None;
    }
    Some(())
}
