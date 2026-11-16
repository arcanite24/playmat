//! `HasSubtypesSourceEffect(subtypes…)` (`Zone.ALL`, layer 4): "{this} is also a Cleric, Rogue, Warrior, and
//! Wizard" (Stonework Packbeast, Veteran Adventurer, Tajuru Paragon): the source has the subtypes in every zone.

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::continuous("HasSubtypesSourceEffect", affects, Layer::Type(subtypes))
        .args(&["text", "text", "text", "text", "text", "text"])
        .in_any_zone()];

fn affects(_: &Game, cx: &StaticCx, id: ObjId) -> bool {
    id == cx.source
}

/// `SubType.CLERIC` → the subtype's name ("Cleric")
fn subtypes(_: &Game, cx: &StaticCx, _: ObjId) -> Vec<String> {
    cx.args.iter().filter_map(|a| subtype_name(a.trim().strip_prefix("SubType.")?)).collect()
}

/// The SubType constant's description: its words, capitalized (`SubType.CLERIC` "Cleric")
fn subtype_name(constant: &str) -> Option<String> {
    let words: Vec<String> = constant
        .split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_ascii_uppercase().to_string() + &c.as_str().to_ascii_lowercase()).unwrap_or_default()
        })
        .collect();
    (!words.is_empty()).then(|| words.join(" "))
}
