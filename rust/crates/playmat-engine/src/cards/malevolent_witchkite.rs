//! Malevolent Witchkite: "Flying. When Malevolent Witchkite enters, sacrifice any number of artifacts, enchantments,
//! and/or tokens, then draw that many cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MalevolentWitchkite.MalevolentWitchkiteEffect", apply)];

/// `choose(TargetSacrifice(0, MAX, …))`: artifacts, enchantments and tokens you control, picked until you're done or
/// none is left; "<player> chose N permanents to sacrifice. (source: …)", they're sacrificed, then that many cards
/// drawn
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let fits = |game: &Game, o: ObjId| {
        game.obj(o).controller == you
            && (game.has_type(o, CardType::Artifact) || game.has_type(o, CardType::Enchantment) || game.obj(o).token)
    };
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
    if n > 0 {
        draw(game, you, n);
    }
    Some(())
}
