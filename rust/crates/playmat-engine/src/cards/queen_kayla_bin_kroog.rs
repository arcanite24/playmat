//! Queen Kayla bin-Kroog: "{4}, {T}: Discard all the cards in your hand, then draw that many cards. You may choose an
//! artifact or creature card with mana value 1 you discarded this way, then do the same for artifact or creature
//! cards with mana values 2 and 3. Return those cards to the battlefield. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("QueenKaylaBinKroog.QueenKaylaBinKroogEffect", apply)];

/// The whole hand discarded and as many cards drawn; then `QueenKaylaBinKroogTarget` (0 to 3 artifact or creature
/// cards among the discarded ones, wherever they are now, no two with the same mana value: Java checks no more than
/// that), chosen one at a time, and those cards onto the battlefield under your control
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game.player(you).hand.len();
    if n == 0 {
        return Some(());
    }
    let discarded = discard(game, d, you, n as i32);
    if d.stopped() {
        return None;
    }
    draw(game, you, discarded.len());
    let fits = |game: &Game, c: ObjId| {
        let o = game.obj(c);
        o.zone != Zone::Battlefield && !o.token && (o.is(CardType::Artifact) || o.is(CardType::Creature))
    };
    let mut chosen: Vec<ObjId> = Vec::new();
    while chosen.len() < 3 {
        let used: Vec<u32> = chosen.iter().map(|c| game.obj(*c).def.mana_value()).collect();
        let left: Vec<ObjId> = discarded
            .iter()
            .copied()
            .filter(|c| !chosen.contains(c) && fits(game, *c) && !used.contains(&game.obj(*c).def.mana_value()))
            .collect();
        if left.is_empty() {
            break;
        }
        match ask(game, d, you, &Query::Cards { objs: left.clone(), required: false }) {
            Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
            Answer::Obj(o) if left.contains(&o) => chosen.push(o),
            Answer::Stop => return None,
            Answer::Obj(_) | Answer::Player(_) | Answer::Unknown => {}
            _ => break,
        }
        if d.stopped() {
            return None;
        }
    }
    if chosen.is_empty() {
        return Some(());
    }
    if !put_cards(game, d, you, &chosen, crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}
