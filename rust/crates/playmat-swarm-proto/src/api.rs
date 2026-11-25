//! The coordinator's HTTP API (`playmat-swarm`, docs/swarm/Design.md "Architecture"), as JSON bodies under `/api/v1`.
//! Every call but `GET /status` carries `Authorization: Bearer <token>`; a token has one role: worker, trainer or
//! admin.
//!
//! Worker: `POST /hello` once, then `POST /lease` until it gets a unit, `POST /units/<id>/heartbeat` while it plays,
//! `POST /units/<id>/uploads` for presigned upload URLs, a `PUT` of every file, and `POST /units/<id>/complete` with
//! the manifest, or `/fail` (counts against the unit; that host never gets it again) or `/release` (handed back
//! unplayed because the host stopped being idle: no failure). Networks come from `GET /nets/<sha256>`, which redirects to the store.
//!
//! Trainer: `POST /nets/<sha256>` for an upload URL, `POST /rounds` with the round's units, `GET /rounds/<id>` for
//! progress, `GET /rounds/<id>/results` for the completed units and their files, `POST /units/<id>/verdict` once it has
//! replayed them. Admin: `POST /tokens`.

use crate::{ResultManifest, WorkUnit};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The token's role
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Worker,
    Trainer,
    Admin,
}

impl Role {
    pub fn name(self) -> &'static str {
        match self {
            Role::Worker => "worker",
            Role::Trainer => "trainer",
            Role::Admin => "admin",
        }
    }

    pub fn parse(s: &str) -> Option<Role> {
        [Role::Worker, Role::Trainer, Role::Admin].into_iter().find(|r| r.name() == s)
    }
}

/// A worker host introducing itself: its name (the contributor's choice), build and determinism class
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub name: String,
    pub build: String,
    pub class: String,
    pub strict: bool,
    pub threads: u32,
    pub os: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelloReply {
    pub host: i64,
    /// seconds to wait before asking again when there's no work
    pub poll_secs: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaseRequest {
    pub host: i64,
}

/// A leased unit: its `deadline` is the lease's end, which each heartbeat moves on by `lease_secs`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lease {
    pub unit: WorkUnit,
    pub lease_secs: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadRequest {
    pub files: Vec<FileInfo>,
}

/// One presigned `PUT` URL per file, by path
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Urls {
    pub urls: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Complete {
    pub manifest: ResultManifest,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fail {
    pub reason: String,
}

/// A round the trainer publishes: its units, played in `priority` order (higher first) and then by unit id
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NewRound {
    pub project: String,
    pub run: String,
    pub round: u32,
    #[serde(default)]
    pub priority: i32,
    pub units: Vec<WorkUnit>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundStatus {
    pub id: i64,
    pub project: String,
    pub run: String,
    pub round: u32,
    /// units by state: queued, leased, complete (waiting for a verdict), accepted, rejected, failed
    pub units: BTreeMap<String, u32>,
    /// games in completed and accepted units
    pub games: u64,
}

/// A completed unit, for the trainer to replay: the unit as the coordinator leased it (replay against this, never
/// the worker's own `unit.json`), its manifest and a presigned `GET` URL per file
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnitResult {
    pub unit: String,
    pub work: WorkUnit,
    pub state: String,
    pub host: String,
    pub account: String,
    pub manifest: ResultManifest,
    pub files: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    pub accept: bool,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewToken {
    pub role: Role,
    /// who it's for: an account name, a host or "trainer"
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenReply {
    pub id: i64,
    /// shown once: the coordinator keeps only its hash
    pub token: String,
}

/// The public numbers (`GET /status`, no token): what the tracker will draw from
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    /// hosts seen in the last 10 minutes, and their threads
    pub hosts: u32,
    pub threads: u32,
    pub units: BTreeMap<String, u32>,
    pub games_accepted: u64,
    pub games_last_hour: u64,
}
