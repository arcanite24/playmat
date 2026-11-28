//! Gilgamesh, Master-at-Arms: "Whenever Gilgamesh enters or attacks, look at the top six cards of your library. You
//! may put any number of Equipment cards from among them onto the battlefield. Put the rest on the bottom of your
//! library in a random order. When you put one or more Equipment onto the battlefield this way, you may attach one of
//! them to a Samurai you control."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("GilgameshMasterAtArms.GilgameshMasterAtArmsLookEffect", look),
    Handler::effect("GilgameshMasterAtArms.GilgameshMasterAtArmsAttachEffect", attach),
];

/// The top six (none, nothing happens); any number of the Equipment among them chosen (`player.choose` with
/// `TargetCardInLibrary(0, MAX)`) and put onto the battlefield, the rest to the bottom at random; with any of them on
/// the battlefield, the reflexive trigger, fixed on those permanents
fn look(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(6).copied().collect();
    if top.is_empty() {
        return Some(());
    }
    let equipment: Vec<ObjId> = top.iter().copied().filter(|c| game.has_subtype(*c, "Equipment")).collect();
    let chosen = choose_any(game, d, you, &equipment, usize::MAX)?;
    // the reflexive trigger goes ahead of the Equipment's own enters triggers (Java's stack order)
    let pending_at = game.pending.len();
    if !chosen.is_empty() && !put_cards(game, d, you, &chosen, crate::card::PutCards::Battlefield) {
        return None;
    }
    let rest: Vec<ObjId> = top.iter().copied().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !rest.is_empty() && !put_cards(game, d, you, &rest, crate::card::PutCards::BottomRandom) {
        return None;
    }
    let entered: Vec<(ObjId, u32)> = chosen
        .iter()
        .copied()
        .filter(|c| game.obj(*c).zone == Zone::Battlefield)
        .map(|c| (c, game.obj(c).zcc))
        .collect();
    if entered.is_empty() {
        return Some(());
    }
    let handler = serde_json::from_value(serde_json::json!("GilgameshMasterAtArms.GilgameshMasterAtArmsAttachEffect")).ok()?;
    let effect =
        EffectDef::CardEffect { handler, args: Vec::new(), tokens: Vec::new(), effects: Vec::new(), filters: Vec::new() };
    let delayed = crate::state::Delayed {
        rule: "you may attach one of them to a Samurai you control.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![effect],
        source_zcc: cx.source.1,
        fixed: entered,
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    let at = pending_at.min(game.pending.len());
    game.pending.insert(at, StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

/// Up to one Samurai you control (none, nothing), then which of those Equipment still on the battlefield: the only
/// one, or up to one chosen
fn attach(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let equipment: Vec<ObjId> = cx.objs(0).into_iter().filter(|o| game.obj(*o).zone == Zone::Battlefield).collect();
    if equipment.is_empty() {
        return Some(());
    }
    let samurai: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_subtype(*o, "Samurai"))
        .collect();
    let Some(samurai) = choose_any(game, d, you, &samurai, 1)?.first().copied() else { return Some(()) };
    let equip = if equipment.len() == 1 {
        equipment[0]
    } else {
        match choose_any(game, d, you, &equipment, 1)?.first() {
            Some(e) => *e,
            None => return Some(()),
        }
    };
    let zcc = game.obj(samurai).zcc;
    game.attach_to(equip, samurai, zcc);
    Some(())
}

/// `HumanPlayer.choose` for up to `max` (none required), as `choose_cards_rounds`, except that with nothing it can
/// choose the player is still asked, again after each pick it ignores, until they're done
pub(super) fn choose_any(
    game: &mut Game,
    d: &mut dyn Decider,
    seat: Seat,
    possible: &[ObjId],
    max: usize,
) -> Option<Vec<ObjId>> {
    if !possible.is_empty() {
        return choose_cards_rounds(game, d, seat, possible, 0, max);
    }
    loop {
        match ask(game, d, seat, &Query::Cards { objs: Vec::new(), required: false }) {
            Answer::Stop => return None,
            Answer::Obj(_) | Answer::Player(_) | Answer::Unknown => {}
            _ => return Some(Vec::new()),
        }
        if d.stopped() {
            return None;
        }
    }
}
