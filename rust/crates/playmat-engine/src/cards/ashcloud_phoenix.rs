//! Ashcloud Phoenix: "Flying. When Ashcloud Phoenix dies, return it to the battlefield face down under your control.
//! Morph {4}{R}{R}. When Ashcloud Phoenix is turned face up, it deals 2 damage to each player."

use super::*;
use crate::card::FaceDown;
use std::sync::Arc;

pub const HANDLERS: &[Handler] = &[Handler::effect("AshcloudPhoenix.AshcloudPhoenixEffect", apply)];

/// the card, still in the graveyard it died into, onto the battlefield face down under your control (`moveCards`
/// with `faceDown`: a 2/2 that its morph cost turns face up)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (c, zcc) = cx.source;
    if game.obj(c).zone != Zone::Graveyard || game.obj(c).zcc != zcc {
        return Some(());
    }
    let full = game.obj(c).def.clone();
    let def = full.face_down(FaceDown::Morph);
    let o = game.obj_mut(c);
    o.face_up = Some(full);
    o.face_down = true;
    o.face_down_as = Some(FaceDown::Morph);
    o.def = Arc::new(def);
    put_onto_battlefield(game, d, c, cx.controller());
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
