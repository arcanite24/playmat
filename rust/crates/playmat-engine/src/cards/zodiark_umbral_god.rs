//! Zodiark, Umbral God: "When Zodiark enters, each player sacrifices half the non-God creatures they control of
//! their choice, rounded down."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("ZodiarkUmbralGod.ZodiarkUmbralGodEffect", apply).filters(&["filter"])];

/// Each player in turn order from you chooses exactly half their non-God creatures (`TargetPermanent(n, n, …)`,
/// auto-chosen as Java's when only those are left); then all the chosen are sacrificed. (Java sacrifices them in a
/// `HashSet`'s order; here, in the order chosen.)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    let mut chosen = Vec::new();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let possible: Vec<ObjId> = matching(game, &cx.filters[0], p, cx.source_id())
            .into_iter()
            .filter(|o| game.obj(*o).controller == p)
            .collect();
        let n = possible.len() / 2;
        if n < 1 {
            continue;
        }
        chosen.extend(choose_cards(game, d, p, &possible, &possible, n, n, &rule)?);
    }
    for o in chosen {
        if game.obj(o).zone == Zone::Battlefield {
            sacrifice(game, o);
        }
    }
    Some(())
}
