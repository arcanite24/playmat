//! Ed25519-signed release verification and atomic self-update for `playmat-worker` (docs/swarm/Design.md).
//!
//! Releases are published to `arcanite24/mana-at-home` (separate from Playmat's own releases) with:
//! - `playmat-worker-<os>-<arch>[.exe]`
//! - `SHA256SUMS.txt`
//! - `SHA256SUMS.txt.sig` (hex-encoded 64-byte Ed25519 signature over `SHA256SUMS.txt`)
//!
//! Every download is checked in two layers before touching the executable:
//! 1. Ed25519 signature over `SHA256SUMS.txt` against `DEFAULT_RELEASE_PUBKEY_HEX` (or `MANA_UPDATE_PUBKEY`).
//! 2. SHA-256 of the downloaded binary against the signed entry in `SHA256SUMS.txt`.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use playmat_swarm_proto::sha256_hex;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const DEFAULT_UPDATE_REPO: &str = "arcanite24/mana-at-home";
pub const DEFAULT_RELEASE_PUBKEY_HEX: &str = "6f6dfe3870e00761d4af3bed5a4f658d4f6f3fd99aec687096593912dec1c298";

pub fn current_release_tag() -> &'static str {
    option_env!("MANA_RELEASE_TAG").unwrap_or("dev")
}

pub fn current_build() -> &'static str {
    option_env!("PLAYMAT_BUILD").unwrap_or("dev")
}

pub fn platform_asset_name() -> Result<&'static str, String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("playmat-worker-linux-amd64"),
        ("linux", "aarch64") => Ok("playmat-worker-linux-arm64"),
        ("macos", "aarch64") => Ok("playmat-worker-macos-arm64"),
        ("windows", "x86_64") => Ok("playmat-worker-windows-amd64.exe"),
        (os, arch) => Err(format!("no prebuilt release binary for {os}-{arch}")),
    }
}

fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) {
        return Err("odd-length hex string".into());
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let byte = u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| format!("invalid hex: {e}"))?;
        out.push(byte);
    }
    Ok(out)
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn sign_bytes(payload: &[u8], seed_hex: &str) -> Result<String, String> {
    let raw = decode_hex(seed_hex)?;
    let arr: [u8; 32] = raw.try_into().map_err(|_| "signing key seed must be 32 bytes (64 hex chars)".to_string())?;
    let key = SigningKey::from_bytes(&arr);
    let sig = key.sign(payload);
    Ok(encode_hex(&sig.to_bytes()))
}

pub fn verify_signature(payload: &[u8], sig_hex: &str, pubkey_hex: &str) -> Result<(), String> {
    let pk_raw = decode_hex(pubkey_hex)?;
    let pk_arr: [u8; 32] = pk_raw.try_into().map_err(|_| "public key must be 32 bytes (64 hex chars)".to_string())?;
    let vk = VerifyingKey::from_bytes(&pk_arr).map_err(|e| format!("invalid Ed25519 public key: {e}"))?;

    let sig_raw = decode_hex(sig_hex)?;
    let sig_arr: [u8; 64] = sig_raw.try_into().map_err(|_| "signature must be 64 bytes (128 hex chars)".to_string())?;
    let sig = Signature::from_bytes(&sig_arr);

    vk.verify(payload, &sig).map_err(|e| format!("Ed25519 signature verification failed: {e}"))
}

pub fn parse_sha256_for_asset(sums_text: &str, asset_name: &str) -> Option<String> {
    for line in sums_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?.trim_start_matches('*').trim_start_matches("./");
        if name == asset_name && hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Some(hash.to_ascii_lowercase());
        }
    }
    None
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    assets: Vec<GhAsset>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateOutcome {
    UpToDate { tag: String },
    Available { tag: String, asset: String },
    Updated { tag: String, path: PathBuf },
}

pub fn check_or_apply(repo: &str, pubkey_hex: &str, apply: bool, force: bool) -> Result<UpdateOutcome, String> {
    let asset = platform_asset_name()?;
    let http = Client::builder()
        .timeout(Duration::from_secs(120))
        .user_agent(concat!("playmat-worker/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;

    let rel_url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let rel: GhRelease = http
        .get(&rel_url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|e| format!("release lookup ({rel_url}): {e}"))?
        .error_for_status()
        .map_err(|e| format!("release lookup ({rel_url}): {e}"))?
        .json()
        .map_err(|e| format!("release json: {e}"))?;

    let cur = current_release_tag();
    if !force && cur == rel.tag_name {
        return Ok(UpdateOutcome::UpToDate { tag: rel.tag_name });
    }

    let find = |name: &str| -> Result<&str, String> {
        rel.assets
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.browser_download_url.as_str())
            .ok_or_else(|| format!("release {} is missing required asset {name}", rel.tag_name))
    };

    let sums_url = find("SHA256SUMS.txt")?;
    let sig_url = find("SHA256SUMS.txt.sig")?;
    let bin_url = find(asset)?;

    let sums_bytes = http
        .get(sums_url)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.bytes())
        .map_err(|e| format!("download SHA256SUMS.txt: {e}"))?;
    let sig_text = http
        .get(sig_url)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.text())
        .map_err(|e| format!("download SHA256SUMS.txt.sig: {e}"))?;

    verify_signature(&sums_bytes, &sig_text, pubkey_hex)?;

    let sums_str = std::str::from_utf8(&sums_bytes).map_err(|e| format!("SHA256SUMS.txt utf8: {e}"))?;
    let expected_sha =
        parse_sha256_for_asset(sums_str, asset).ok_or_else(|| format!("SHA256SUMS.txt has no entry for {asset}"))?;

    if !apply {
        return Ok(UpdateOutcome::Available { tag: rel.tag_name, asset: asset.into() });
    }

    let bin_bytes = http
        .get(bin_url)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.bytes())
        .map_err(|e| format!("download {asset}: {e}"))?;
    let actual_sha = sha256_hex(&bin_bytes);
    if actual_sha != expected_sha {
        return Err(format!("sha256 mismatch for {asset}: expected {expected_sha}, got {actual_sha}"));
    }

    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    replace_executable(&exe, &bin_bytes)?;
    Ok(UpdateOutcome::Updated { tag: rel.tag_name, path: exe })
}

fn replace_executable(exe: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = exe.parent().ok_or("executable has no parent directory")?;
    let tmp = parent.join(format!(".playmat-worker-update-{}.tmp", std::process::id()));
    std::fs::write(&tmp, bytes).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("chmod {}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, exe).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("replace {}: {e}", exe.display())
        })?;
    }
    #[cfg(windows)]
    {
        let backup = exe.with_extension("exe.old");
        let _ = std::fs::remove_file(&backup);
        std::fs::rename(exe, &backup).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("move old exe: {e}")
        })?;
        if let Err(e) = std::fs::rename(&tmp, exe) {
            let _ = std::fs::rename(&backup, exe);
            let _ = std::fs::remove_file(&tmp);
            return Err(format!("install new exe: {e}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ed25519_sign_and_verify_roundtrip_and_tamper_rejection() {
        let seed_hex = "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";
        let seed_bytes: [u8; 32] = decode_hex(seed_hex).unwrap().try_into().unwrap();
        let pub_hex = encode_hex(SigningKey::from_bytes(&seed_bytes).verifying_key().as_bytes());

        let sums = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  playmat-worker-linux-amd64\n";
        let sig = sign_bytes(sums, seed_hex).unwrap();
        assert!(verify_signature(sums, &sig, &pub_hex).is_ok());

        let tampered =
            b"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb  playmat-worker-linux-amd64\n";
        assert!(verify_signature(tampered, &sig, &pub_hex).is_err());

        let wrong_pub = "00".repeat(32);
        assert!(verify_signature(sums, &sig, &wrong_pub).is_err());
    }

    #[test]
    fn parses_sha256sums_entries() {
        let sums = "\
1111111111111111111111111111111111111111111111111111111111111111  playmat-worker-linux-amd64
2222222222222222222222222222222222222222222222222222222222222222 *playmat-worker-windows-amd64.exe
";
        assert_eq!(
            parse_sha256_for_asset(sums, "playmat-worker-linux-amd64").as_deref(),
            Some("1111111111111111111111111111111111111111111111111111111111111111")
        );
        assert_eq!(
            parse_sha256_for_asset(sums, "playmat-worker-windows-amd64.exe").as_deref(),
            Some("2222222222222222222222222222222222222222222222222222222222222222")
        );
        assert!(parse_sha256_for_asset(sums, "playmat-worker-macos-arm64").is_none());
    }
}
