//! Sigarda's Splendor: "As Sigarda's Splendor enters the battlefield, note your life total. At the beginning of your
//! upkeep, draw a card if your life total is greater than or equal to the last noted life total for Sigarda's
//! Splendor. Then note your life total. Whenever you cast a white spell, you gain 1 life."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("SigardasSplendor.SigardasSplendorNoteEffect", note_life),
    Handler::effect("SigardasSplendor.SigardasSplendorDrawEffect", draw_then_note),
];

const KEY: &str = "noted life:";

/// `game.getState().setValue(key, life)`, its key the permanent (the zone-change count it has on the battlefield):
/// kept on the object, which forgets it as it changes zones
fn set_noted(game: &mut Game, id: ObjId, you: Seat) {
    let life = game.player(you).life;
    if log::kept(game) {
        let line = format!("{} notes their life total of {life}", log::player(you));
        log::inform(game, line);
    }
    let o = game.obj_mut(id);
    o.marks.retain(|m| !m.starts_with(KEY));
    o.marks.push(format!("{KEY}{life}"));
}

/// As it enters: your life total noted
fn note_life(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    set_noted(game, cx.source_id(), cx.controller());
    Some(())
}

/// A card if your life is at least the noted total (none noted: any), then your life total noted again (on the
/// permanent while it's still the one that triggered; otherwise the value goes nowhere the permanent can read)
fn draw_then_note(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let here = cx.source_on_battlefield(game);
    let noted = if here {
        game.obj(cx.source_id()).marks.iter().find_map(|m| m.strip_prefix(KEY)?.parse::<i32>().ok())
    } else {
        None
    };
    if game.player(you).life >= noted.unwrap_or(i32::MIN) {
        draw(game, you, 1);
    }
    if here {
        set_noted(game, cx.source_id(), you);
    } else if log::kept(game) {
        let line = format!("{} notes their life total of {}", log::player(you), game.player(you).life);
        log::inform(game, line);
    }
    Some(())
}
