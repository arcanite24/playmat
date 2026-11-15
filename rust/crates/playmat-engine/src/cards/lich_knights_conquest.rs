//! Lich-Knights' Conquest: "Sacrifice any number of artifacts, enchantments, and/or tokens. Return that many creature
//! cards from your graveyard to the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LichKnightsConquest.LichKnightsConquestEffect", apply)];

/// `chooseTarget(TargetSacrifice(0, MAX, …))` (nothing asked with none): artifacts, enchantments and tokens you
/// control, picked until you're done or none is left; "<player> chose N permanents to sacrifice. (source: …)", they're
/// sacrificed, then that many creature cards (or all there are) chosen from your graveyard (`TargetCardInYourGraveyard(n, n)`) go onto the
/// battlefield together
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let fits = |game: &Game, o: ObjId| {
        game.obj(o).controller == you
            && (game.has_type(o, CardType::Artifact) || game.has_type(o, CardType::Enchantment) || game.obj(o).token)
    };
    // (`chooseTarget` asks nothing when nothing can be chosen)
    if !game.battlefield.iter().any(|o| fits(game, *o)) {
        return Some(());
    }
    let mut chosen: Vec<ObjId> = Vec::new();
    loop {
        let can: Vec<ObjId> =
            game.battlefield.iter().copied().filter(|o| fits(game, *o) && !chosen.contains(o)).collect();
        match ask(game, d, you, &Query::Cards { objs: can.clone(), required: false }) {
            Answer::Obj(o) if chosen.contains(&o) => chosen.retain(|c| *c != o),
            Answer::Obj(o) if can.contains(&o) => {
                chosen.push(o);
                if can.len() == 1 {
                    break;
                }
            }
            Answer::Stop => return None,
            Answer::Obj(_) | Answer::Player(_) | Answer::Unknown => {}
            _ => break,
        }
        if d.stopped() {
            return None;
        }
    }
    let chosen: Vec<ObjId> = chosen.into_iter().filter(|o| game.obj(*o).zone == Zone::Battlefield).collect();
    let n = chosen.len();
    if n > 0 {
        if log::kept(game) {
            let line = format!(
                "{} chose {n} permanents to sacrifice.  (source: {})",
                log::player(you),
                log::object(game, cx.source_id())
            );
            log::inform(game, line);
        }
        for o in chosen {
            sacrifice(game, o);
        }
    }
    if n == 0 {
        return Some(());
    }
    let graveyard = game.player(you).graveyard.clone();
    let creatures: Vec<ObjId> = graveyard.iter().copied().filter(|c| game.obj(*c).is(CardType::Creature)).collect();
    let k = n.min(creatures.len());
    if k == 0 {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let back = choose_cards(game, d, you, &graveyard, &creatures, k, k, &rule)?;
    if !put_cards(game, d, you, &back, crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}
