//! Where results and networks live (docs/swarm/Design.md, "Data budget"). Workers and the trainer never get store
//! credentials: they get presigned URLs for one object each, a `PUT` signed for its exact size or a `GET`, valid for
//! a while. Game data never passes through the coordinator on R2; the local store serves it itself, for the LAN and
//! the tests.
//!
//! Keys: `nets/<sha256>.pmnet` and `intake/<round>/<unit>/<path>`.

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

pub enum Store {
    Local(Local),
    R2(R2),
}

impl Store {
    /// A URL that uploads exactly `size` bytes to `key`
    pub fn put_url(&self, key: &str, size: u64, secs: u64) -> String {
        match self {
            Store::Local(l) => l.url("PUT", key, size, secs),
            Store::R2(r) => r.presign("PUT", key, Some(size), secs, now()),
        }
    }

    pub fn get_url(&self, key: &str, secs: u64) -> String {
        match self {
            Store::Local(l) => l.url("GET", key, 0, secs),
            Store::R2(r) => r.presign("GET", key, None, secs, now()),
        }
    }

    /// The stored object's size, `None` if it isn't there
    pub async fn size(&self, key: &str) -> anyhow::Result<Option<u64>> {
        match self {
            Store::Local(l) => Ok(std::fs::metadata(l.path(key)?).ok().map(|m| m.len())),
            Store::R2(r) => {
                let url = r.presign("HEAD", key, None, 60, now());
                let res = r.http.head(url).send().await?;
                if res.status() == reqwest::StatusCode::NOT_FOUND {
                    return Ok(None);
                }
                let res = res.error_for_status()?;
                // the header itself: reqwest's content_length() is the (empty) body's for a HEAD response
                Ok(res.headers().get("content-length").and_then(|v| v.to_str().ok()?.parse().ok()))
            }
        }
    }

    /// Reads a UTF-8 text object up to `max_bytes` (used by `/api/v1/watch/{unit}/{deal}` to inspect `answers.jsonl`).
    pub async fn read_text(&self, key: &str, max_bytes: usize) -> Option<String> {
        match self {
            Store::Local(l) => {
                let p = l.path(key).ok()?;
                let meta = std::fs::metadata(&p).ok()?;
                if meta.len() as usize > max_bytes {
                    return None;
                }
                std::fs::read_to_string(&p).ok()
            }
            Store::R2(r) => {
                let url = r.presign("GET", key, None, 60, now());
                let res = r.http.get(url).send().await.ok()?;
                if !res.status().is_success() {
                    return None;
                }
                let bytes = res.bytes().await.ok()?;
                if bytes.len() > max_bytes {
                    return None;
                }
                String::from_utf8(bytes.to_vec()).ok()
            }
        }
    }

    /// Deletes an object from the store (`Ok(true)` if removed, `Ok(false)` if already absent).
    pub async fn delete(&self, key: &str) -> anyhow::Result<bool> {
        match self {
            Store::Local(l) => {
                let p = l.path(key)?;
                match std::fs::remove_file(&p) {
                    Ok(()) => Ok(true),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
                    Err(e) => Err(e.into()),
                }
            }
            Store::R2(r) => {
                let url = r.presign("DELETE", key, None, 60, now());
                let res = r.http.delete(url).send().await?;
                if res.status() == reqwest::StatusCode::NOT_FOUND {
                    return Ok(false);
                }
                res.error_for_status()?;
                Ok(true)
            }
        }
    }
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut m = HmacSha256::new_from_slice(key).expect("any key length");
    m.update(data);
    m.finalize().into_bytes().to_vec()
}

/// A key the stores accept: path segments of `[A-Za-z0-9._-]`, no `..`, nothing empty
pub fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.split('/').all(|s| {
            !s.is_empty()
                && s != "."
                && s != ".."
                && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        })
}

/// Files on the coordinator's disk, behind its own `/store/<key>` routes and signed with a per-process secret
pub struct Local {
    pub root: PathBuf,
    /// the coordinator's public URL, e.g. `http://minivac:17190`
    pub base: String,
    secret: [u8; 32],
}

impl Local {
    pub fn new(root: PathBuf, base: String) -> Local {
        let mut secret = [0u8; 32];
        getrandom::getrandom(&mut secret).expect("randomness");
        Local { root, base: base.trim_end_matches('/').to_string(), secret }
    }

    pub fn path(&self, key: &str) -> anyhow::Result<PathBuf> {
        anyhow::ensure!(valid_key(key), "bad key {key}");
        Ok(self.root.join(Path::new(key)))
    }

    fn sig(&self, method: &str, key: &str, size: u64, exp: u64) -> String {
        hex(&hmac(&self.secret, format!("{method}\n{key}\n{size}\n{exp}").as_bytes()))
    }

    fn url(&self, method: &str, key: &str, size: u64, secs: u64) -> String {
        let exp = now() + secs;
        let sig = self.sig(method, key, size, exp);
        format!("{}/store/{key}?exp={exp}&size={size}&sig={sig}", self.base)
    }

    /// Whether a request to `/store/<key>` carries a live signature for that method and size
    pub fn check(&self, method: &str, key: &str, size: u64, exp: u64, sig: &str) -> bool {
        exp >= now() && valid_key(key) && self.sig(method, key, size, exp) == sig
    }
}

/// Cloudflare R2 (S3's API), presigned with AWS Signature Version 4 in the query string
pub struct R2 {
    /// `<account>.r2.cloudflarestorage.com`
    pub host: String,
    pub bucket: String,
    pub key_id: String,
    secret: String,
    http: reqwest::Client,
}

impl R2 {
    pub fn new(account: &str, bucket: &str, key_id: &str, secret: &str) -> R2 {
        R2 {
            host: format!("{account}.r2.cloudflarestorage.com"),
            bucket: bucket.into(),
            key_id: key_id.into(),
            secret: secret.into(),
            http: reqwest::Client::new(),
        }
    }

    /// The query-string signature: `host` is always signed, and a `PUT`'s `content-length` too, so the upload must
    /// be exactly the size the worker declared
    pub fn presign(&self, method: &str, key: &str, size: Option<u64>, secs: u64, at: u64) -> String {
        let path = format!("/{}/{}", self.bucket, encode(key, false));
        sigv4(&self.host, &path, "auto", &self.key_id, &self.secret, method, size, secs, at)
    }
}

/// A presigned URL for `https://<host><path>` (S3's query-string authentication)
#[allow(clippy::too_many_arguments)]
fn sigv4(
    host: &str,
    path: &str,
    region: &str,
    key_id: &str,
    secret: &str,
    method: &str,
    size: Option<u64>,
    secs: u64,
    at: u64,
) -> String {
    {
        let (date, stamp) = amz_time(at);
        let scope = format!("{date}/{region}/s3/aws4_request");
        let signed = if size.is_some() { "content-length;host" } else { "host" };
        let mut q = [
            ("X-Amz-Algorithm", "AWS4-HMAC-SHA256".to_string()),
            ("X-Amz-Credential", format!("{key_id}/{scope}")),
            ("X-Amz-Date", stamp.clone()),
            ("X-Amz-Expires", secs.min(604_800).to_string()),
            ("X-Amz-SignedHeaders", signed.to_string()),
        ]
        .map(|(k, v)| (k.to_string(), encode(&v, true)));
        q.sort();
        let query = q.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&");
        let headers = match size {
            Some(n) => format!("content-length:{n}\nhost:{host}\n"),
            None => format!("host:{host}\n"),
        };
        let canonical = format!("{method}\n{path}\n{query}\n{headers}\n{signed}\nUNSIGNED-PAYLOAD");
        let to_sign = format!("AWS4-HMAC-SHA256\n{stamp}\n{scope}\n{}", hex(&Sha256::digest(canonical.as_bytes())));
        let mut k = hmac(format!("AWS4{secret}").as_bytes(), date.as_bytes());
        for part in [region, "s3", "aws4_request"] {
            k = hmac(&k, part.as_bytes());
        }
        let sig = hex(&hmac(&k, to_sign.as_bytes()));
        format!("https://{host}{path}?{query}&X-Amz-Signature={sig}")
    }
}

/// S3's URI encoding: unreserved characters stay, everything else is `%XX`; `/` stays in paths
fn encode(s: &str, slash: bool) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) || (b == b'/' && !slash) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// (`YYYYMMDD`, `YYYYMMDDTHHMMSSZ`) in UTC
fn amz_time(at: u64) -> (String, String) {
    let days = (at / 86_400) as i64;
    let secs = at % 86_400;
    // Howard Hinnant's days-to-civil
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + (m <= 2) as i64;
    let date = format!("{y:04}{m:02}{d:02}");
    (date.clone(), format!("{date}T{:02}{:02}{:02}Z", secs / 3600, secs / 60 % 60, secs % 60))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_dates() {
        assert_eq!(amz_time(0), ("19700101".into(), "19700101T000000Z".into()));
        assert_eq!(amz_time(1_795_651_200 + 3661).1, "20261126T010101Z");
        assert_eq!(amz_time(951_782_400).0, "20000229");
    }

    #[test]
    fn keys_stay_inside_the_store() {
        assert!(valid_key("intake/3/u-1/w00.meta.i32"));
        for k in ["", "../x", "a//b", "a/./b", "/abs", "a b", "a/b/"] {
            assert!(!valid_key(k), "{k}");
        }
    }

    /// AWS's worked example of a presigned GET (S3 API reference, "Authenticating Requests: Using Query Parameters")
    #[test]
    fn sigv4_matches_aws_example() {
        let u = sigv4(
            "examplebucket.s3.amazonaws.com",
            "/test.txt",
            "us-east-1",
            "AKIAIOSFODNN7EXAMPLE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            "GET",
            None,
            86_400,
            1_369_353_600, // 2013-05-24T00:00:00Z
        );
        assert!(u.ends_with("X-Amz-Signature=aeeed9bbccd4d02ee5c0109b86d86835f995330da4c265957d157751f604d404"), "{u}");
    }

    #[test]
    fn presigned_urls_have_the_parts_r2_needs() {
        let r = R2::new("acct", "mana-intake", "AKID", "SECRET");
        let u = r.presign("PUT", "intake/1/u/w00.meta.i32", Some(42), 900, 1_795_651_200);
        assert!(u.starts_with("https://acct.r2.cloudflarestorage.com/mana-intake/intake/1/u/w00.meta.i32?"));
        assert!(u.contains("X-Amz-Credential=AKID%2F20261126%2Fauto%2Fs3%2Faws4_request"));
        assert!(u.contains("X-Amz-SignedHeaders=content-length%3Bhost"));
        assert!(u.contains("X-Amz-Expires=900"));
        let sig = u.rsplit("X-Amz-Signature=").next().unwrap();
        assert_eq!(sig.len(), 64);
        assert_eq!(u, r.presign("PUT", "intake/1/u/w00.meta.i32", Some(42), 900, 1_795_651_200));
        assert_ne!(u, r.presign("PUT", "intake/1/u/w00.meta.i32", Some(43), 900, 1_795_651_200));
    }

    #[test]
    fn local_signatures_bind_method_key_and_size() {
        let l = Local::new(PathBuf::from("/tmp/x"), "http://h:1/".into());
        let u = l.url("PUT", "nets/a.pmnet", 10, 60);
        let q: std::collections::HashMap<_, _> =
            u.split('?').nth(1).unwrap().split('&').map(|kv| kv.split_once('=').unwrap()).collect();
        let exp: u64 = q["exp"].parse().unwrap();
        assert!(l.check("PUT", "nets/a.pmnet", 10, exp, q["sig"]));
        assert!(!l.check("GET", "nets/a.pmnet", 10, exp, q["sig"]));
        assert!(!l.check("PUT", "nets/a.pmnet", 11, exp, q["sig"]));
        assert!(!l.check("PUT", "nets/b.pmnet", 10, exp, q["sig"]));
    }
}
