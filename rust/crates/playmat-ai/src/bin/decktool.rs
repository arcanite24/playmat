//! Inspect engine-playable career decks and dump CardDb metadata for training deck pool generation.
use playmat_ai::duel::Deck;
use playmat_ai::policy::spell_kind;
use playmat_engine::card::{AbilityDef, CardType, Color};
use serde_json::json;
use std::io::Write;
use std::path::Path;

fn color_char(c: Color) -> String {
    let s = format!("{c:?}");
    match s.as_str() {
        "W" | "White" => "W".into(),
        "U" | "Blue" => "U".into(),
        "B" | "Black" => "B".into(),
        "R" | "Red" => "R".into(),
        "G" | "Green" => "G".into(),
        _ => s,
    }
}

fn fnv64(text: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let db = playmat_engine::cards();
    if args.iter().any(|a| a == "--check-career") {
        let root = args
            .windows(2)
            .find(|w| w[0] == "--root")
            .map(|w| w[1].as_str())
            .unwrap_or("/home/playmat/mage");
        let career = Path::new(root).join("Mage.Server/career-src");
        let mut ok = 0usize;
        let mut total = 0usize;
        let mut fmts: Vec<_> = std::fs::read_dir(&career).unwrap().filter_map(|e| e.ok()).collect();
        fmts.sort_by_key(|e| e.file_name());
        for fdir in fmts {
            if !fdir.path().is_dir() {
                continue;
            }
            let mut files: Vec<_> = std::fs::read_dir(fdir.path()).unwrap().filter_map(|e| e.ok()).collect();
            files.sort_by_key(|e| e.file_name());
            for f in files {
                let p = f.path();
                if p.extension().and_then(|s| s.to_str()) != Some("txt") {
                    continue;
                }
                total += 1;
                let text = String::from_utf8_lossy(&std::fs::read(&p).unwrap()).into_owned();
                let stem = p.file_stem().unwrap().to_str().unwrap();
                let deck = Deck::parse(stem, &text);
                let miss = deck.missing(&db);
                let rel = p.strip_prefix(root).unwrap().to_str().unwrap().replace('\\', "/");
                if miss.is_empty() {
                    ok += 1;
                    println!("OK\t{}\t{:016x}\t{}", rel, fnv64(&text), deck.cards.len());
                } else {
                    println!(
                        "MISS\t{}\t{}\t{}",
                        rel,
                        miss.len(),
                        miss.iter().take(3).cloned().collect::<Vec<_>>().join(",")
                    );
                }
            }
        }
        eprintln!("CAREER_CHECK {ok}/{total} playable");
    }
    if let Some(pos) = args.iter().position(|a| a == "--dump-catalog") {
        let out_path = &args[pos + 1];
        let mut f = std::io::BufWriter::new(std::fs::File::create(out_path).unwrap());
        let mut defs: Vec<_> = db.iter().collect();
        defs.sort_by(|a, b| a.name.cmp(&b.name));
        for def in defs {
            let k = spell_kind(def);
            let is_land = def.types.contains(&CardType::Land);
            let is_basic = def.supertypes.iter().any(|s| s == "Basic");
            let is_creature = def.types.contains(&CardType::Creature);
            let is_instant = def.types.contains(&CardType::Instant);
            let is_sorcery = def.types.contains(&CardType::Sorcery);
            let is_enchant = def.types.contains(&CardType::Enchantment);
            let is_artifact = def.types.contains(&CardType::Artifact);
            let is_pw = def.types.contains(&CardType::Planeswalker);
            let has_flash = def.abilities.contains(&AbilityDef::FlashAbility);
            let has_haste = def.abilities.contains(&AbilityDef::HasteAbility);
            let has_flying = def.abilities.contains(&AbilityDef::FlyingAbility);
            let has_trample = def.abilities.contains(&AbilityDef::TrampleAbility);
            let rule_lc = def.spell.rule.to_ascii_lowercase();
            let is_counter = rule_lc.contains("counter target");
            let is_draw = rule_lc.contains("draw a card")
                || rule_lc.contains("draw two")
                || rule_lc.contains("draw three")
                || rule_lc.contains("draw cards");
            let mut cols: Vec<String> = def.colors.iter().map(|c| color_char(*c)).collect();
            cols.sort();
            cols.dedup();
            let mut land_cols: Vec<&str> = Vec::new();
            if is_land {
                let ab_dbg = format!("{:?}", def.abilities);
                for (tok, ch) in [("White", "W"), ("Blue", "U"), ("Black", "B"), ("Red", "R"), ("Green", "G")] {
                    if ab_dbg.contains(tok) {
                        land_cols.push(ch);
                    }
                }
                for (sub, ch) in [("Plains", "W"), ("Island", "U"), ("Swamp", "B"), ("Mountain", "R"), ("Forest", "G")] {
                    if def.subtypes.iter().any(|s| s == sub) && !land_cols.contains(&ch) {
                        land_cols.push(ch);
                    }
                }
            }
            let row = json!({
                "name": def.name,
                "mv": def.mana_value(),
                "colors": cols,
                "land_colors": land_cols,
                "is_land": is_land,
                "is_basic": is_basic,
                "is_creature": is_creature,
                "is_instant": is_instant,
                "is_sorcery": is_sorcery,
                "is_enchant": is_enchant,
                "is_artifact": is_artifact,
                "is_pw": is_pw,
                "power": def.power,
                "toughness": def.toughness,
                "flash": has_flash,
                "haste": has_haste,
                "flying": has_flying,
                "trample": has_trample,
                "removal": k.removal,
                "sweeper": k.sweeper,
                "face": k.face,
                "pump": k.pump,
                "counter": is_counter,
                "draw": is_draw,
                "subtypes": def.subtypes,
                "supertypes": def.supertypes,
            });
            writeln!(f, "{}", row).unwrap();
        }
        eprintln!("CATALOG_DUMPED {} cards to {}", db.iter().count(), out_path);
    }
}
