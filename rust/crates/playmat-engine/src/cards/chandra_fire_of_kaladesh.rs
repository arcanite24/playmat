//! Chandra, Fire of Kaladesh: "{T}: Chandra deals 1 damage to target player or planeswalker. If Chandra has dealt
//! 3 or more damage this turn, exile her, then return her to the battlefield transformed." Her
//! `SourceDealtDamageCondition(3)` is read from `DamageDoneWatcher`: the damage the source, as the object it is now, dealt this turn

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("ChandraFireOfKaladesh.SourceDealtDamageCondition", dealt_3),
    Handler::effect("ChandraFireOfKaladesh.ChandraRoaringFlameEmblemEffect", emblem)
        .effects(&["new GetEmblemEffect(new ChandraRoaringFlameEmblem())"]),
];

fn dealt_3(game: &Game, _you: Seat, source: ObjId) -> bool {
    let zcc = game.obj(source).zcc;
    let dealt: i32 = game.watch.damage.iter().filter(|d| d.source == source && d.source_zcc == zcc).map(|d| d.amount).sum();
    dealt >= 3
}

/// Chandra, Roaring Flame's −7: 6 damage to each opponent; each player dealt damage this way gets the emblem (in turn
/// order, as `game.getOpponents` lists them in a duel)
fn emblem(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut dealt = Vec::new();
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        if deal_damage(game, cx.source_id(), Target::Player(p), 6, false) > 0 {
            dealt.push(p);
        }
    }
    for p in dealt {
        cx.run_as(game, d, &cx.effects[..1], p);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
