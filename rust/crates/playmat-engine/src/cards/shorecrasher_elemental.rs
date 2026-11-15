//! Shorecrasher Elemental: "{U}: Exile Shorecrasher Elemental, then return it to the battlefield face down under its
//! owner's control. {1}: Shorecrasher Elemental gets +1/-1 or -1/+1 until end of turn. Megamorph {4}{U}"

use super::*;
use crate::card::FaceDown;
use std::sync::Arc;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("ShorecrasherElemental.ShorecrasherElementalEffect", flicker),
    Handler::effect("ShorecrasherElemental.ShorecrasherElementalBoostEffect", boost),
];

/// The source still the permanent that activated it: exiled, then from exile onto the battlefield face down under
/// its owner's control (`moveCards` with `faceDown`: a 2/2 its megamorph cost turns face up)
fn flicker(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let c = cx.source_id();
    game.move_to(c, Zone::Exile);
    if game.obj(c).zone != Zone::Exile || game.obj(c).token {
        return Some(());
    }
    let full = game.obj(c).def.clone();
    let def = full.face_down(FaceDown::Morph);
    let o = game.obj_mut(c);
    o.face_up = Some(full);
    o.face_down = true;
    o.face_down_as = Some(FaceDown::Morph);
    o.def = Arc::new(def);
    let owner = game.obj(c).owner;
    put_onto_battlefield(game, d, c, owner);
    if game.obj(c).zone != Zone::Battlefield {
        if let Some(full) = game.obj_mut(c).face_up.take() {
            let o = game.obj_mut(c);
            o.def = full;
            o.face_down = false;
            o.face_down_as = None;
        }
    }
    if d.stopped() {
        return None;
    }
    Some(())
}

/// "Select how to boost" (`ChoiceImpl`, required): +1/-1 or -1/+1 until end of turn
fn boost(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let choices: Vec<String> = vec!["+1/-1".into(), "-1/+1".into()];
    let q = Query::Choice { message: "Select how to boost".into(), choices: choices.clone() };
    let chosen = loop {
        match ask(game, d, you, &q) {
            Answer::Text(t) if choices.contains(&t) => break t,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let (p, t) = if chosen == "+1/-1" { (1, -1) } else { (-1, 1) };
    let e = EffectDef::BoostSourceEffect {
        power: crate::card::Amount::Fixed(p),
        toughness: crate::card::Amount::Fixed(t),
        duration: crate::card::Duration::EndOfTurn,
    };
    cx.run(game, d, &[e]);
    Some(())
}
