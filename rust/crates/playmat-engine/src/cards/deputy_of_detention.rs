//! Deputy of Detention: "When Deputy of Detention enters the battlefield, exile target nonland permanent an opponent
//! controls and all other nonland permanents that player controls with the same name as that permanent until Deputy
//! of Detention leaves the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DeputyOfDetention.DeputyOfDetentionExileEffect", apply)];

/// With this still on the battlefield and the target there: every nonland permanent its controller controls with its
/// name, in battlefield order, exiled together into this one's zone, back when it leaves
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(target) = cx.obj(0) else { return Some(()) };
    if !cx.source_on_battlefield(game) || game.obj(target).zone != Zone::Battlefield {
        return Some(());
    }
    let owner = game.obj(target).controller;
    let name = game.obj(target).name().to_string();
    let exiled: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            let p = game.obj(*o);
            p.controller == owner && !game.has_type(*o, CardType::Land) && p.name() == name
        })
        .collect();
    let (src, zcc) = cx.source;
    for o in exiled {
        game.move_to(o, Zone::Exile);
        if game.obj(o).zone == Zone::Exile {
            game.exiled_until_leaves.push((src, zcc, o));
        }
    }
    Some(())
}
