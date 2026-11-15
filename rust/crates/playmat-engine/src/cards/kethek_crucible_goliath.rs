//! Kethek, Crucible Goliath: "At the beginning of your end step, you may sacrifice another creature. If you do,
//! reveal cards from the top of your library until you reveal a nonlegendary creature card with lesser mana value,
//! put it onto the battlefield, then put the rest on the bottom of your library in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KethekCrucibleGoliath.KethekCrucibleGoliathEffect", apply)];

/// `TargetSacrifice(0, 1, another creature you control)` chosen with `Player.choose` (asked even with none); the
/// one sacrificed sets the mana value, then `RevealCardsFromLibraryUntilEffect(nonlegendary creature card with
/// lesser mana value, BATTLEFIELD, BOTTOM_RANDOM)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let possible: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| *o != src && game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &possible, &possible, 0, 1, &rule)?;
    let Some(&o) = chosen.first() else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let x = game.obj(o).def.mana_value();
    sacrifice(game, o);
    if game.obj(o).zone == Zone::Battlefield {
        return Some(());
    }
    if game.player(you).library.is_empty() {
        return Some(());
    }
    let fits = |game: &Game, c: ObjId| {
        let def = &game.obj(c).def;
        game.obj(c).is(CardType::Creature)
            && !def.supertypes.iter().any(|s| s == "LEGENDARY")
            && def.mana_value() < x
    };
    let mut revealed = Vec::new();
    let mut hit = None;
    for c in game.player(you).library.clone() {
        revealed.push(c);
        if fits(game, c) {
            hit = Some(c);
            break;
        }
    }
    let zcc = game.obj(src).zcc;
    let shown = revealed.iter().map(|c| (*c, Zone::Library)).collect();
    game.revealed.push(crate::state::Revealed { source: src, zcc, paren: false, cards: shown });
    if let Some(c) = hit {
        if !put_cards(game, d, you, &[c], crate::card::PutCards::Battlefield) {
            return None;
        }
    }
    let left: Vec<ObjId> = revealed.into_iter().filter(|c| Some(*c) != hit && game.obj(*c).zone == Zone::Library).collect();
    if !put_cards(game, d, you, &left, crate::card::PutCards::BottomRandom) {
        return None;
    }
    Some(())
}
