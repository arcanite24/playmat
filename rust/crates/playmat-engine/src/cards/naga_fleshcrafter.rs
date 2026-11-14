//! Naga Fleshcrafter: "Renew — {2}{U}, Exile this card from your graveyard: Put a +1/+1 counter on target
//! nonlegendary creature you control. Each other creature you control becomes a copy of that creature until end of
//! turn. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("NagaFleshcrafter.NagaFleshcrafterEffect", apply)];

/// The target still on the battlefield: each other creature you control, in battlefield order, becomes a copy of it
/// until end of turn
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(from) = cx.obj(0) else { return Some(()) };
    if game.obj(from).zone != Zone::Battlefield {
        return Some(());
    }
    let others: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| *o != from && game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    for to in others {
        let copy = game.obj(from).copiable();
        become_copy_until_end_of_turn(game, to, copy);
    }
    Some(())
}
