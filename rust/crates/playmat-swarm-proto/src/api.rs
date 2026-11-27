//! The coordinator's HTTP API (`playmat-swarm`, docs/swarm/Design.md "Architecture"), as JSON bodies under `/api/v1`.
//! Every call but `GET /status` carries `Authorization: Bearer <token>`; a token has one role: worker, trainer or
//! admin.
//!
//! Worker: `POST /hello` once, then `POST /lease` until it gets a unit, `POST /units/<id>/heartbeat` while it plays,
//! `POST /units/<id>/uploads` for presigned upload URLs, a `PUT` of every file, and `POST /units/<id>/complete` with
//! the manifest, or `/fail` (counts against the unit; that host never gets it again) or `/release` (handed back
//! unplayed because the host stopped being idle: no failure). Networks come from `GET /nets/<sha256>`, which redirects to the store.
//!
//! Trainer: `POST /nets/<sha256>` for an upload URL, `POST /rounds` with the round's units, `GET /rounds` to list
//! rounds, `GET /rounds/<id>` for progress, `POST /rounds/<id>/priority` to reprioritize a round,
//! `POST /rounds/<id>/cancel` or `POST /units/<id>/cancel` to cancel remaining units, `GET /rounds/<id>/results` for
//! the completed units and their files, `POST /units/<id>/verdict` once it has replayed them. Admin: `POST /tokens`.

use crate::{ResultManifest, VerifyMode, WorkUnit};
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

/// An account's trust tier (docs/swarm/Design.md "Trust and verification")
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrustLevel {
    /// New account (or new host): 100% of first 20 units re-run audited; no gate/rating/verify work
    Probation,
    /// Probation passed: ~3% re-run audit; self-play only
    Normal,
    /// 30 days and 2,000+ accepted units with no mismatch: ~3% self-play audit, 10% gate audit; may take verify/gate/bench
    Reputable,
    /// Operator-named hosts and project VMs: 0% re-run audit; runs verify/gate/bench work
    Trusted,
    /// Banned from leasing
    Banned,
}

impl TrustLevel {
    pub fn name(self) -> &'static str {
        match self {
            TrustLevel::Probation => "probation",
            TrustLevel::Normal => "normal",
            TrustLevel::Reputable => "reputable",
            TrustLevel::Trusted => "trusted",
            TrustLevel::Banned => "banned",
        }
    }

    pub fn parse(s: &str) -> Option<TrustLevel> {
        [TrustLevel::Probation, TrustLevel::Normal, TrustLevel::Reputable, TrustLevel::Trusted, TrustLevel::Banned]
            .into_iter()
            .find(|t| t.name() == s)
    }

    pub fn can_verify(self) -> bool {
        matches!(self, TrustLevel::Reputable | TrustLevel::Trusted)
    }

    pub fn can_gate(self) -> bool {
        matches!(self, TrustLevel::Reputable | TrustLevel::Trusted)
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
    /// Optional GPU accelerator label (e.g. "cuda:RTX 4080" or "mps"); enables `UnitKind::SelfplayGpu` leases
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu: Option<String>,
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
    #[serde(default)]
    pub priority: i32,
    /// units by state: queued, leased, complete (waiting for a verdict), accepted, rejected, quarantined, failed, cancelled
    pub units: BTreeMap<String, u32>,
    /// games in completed and accepted units
    pub games: u64,
    /// pending (queued or leased) verification units for this round
    #[serde(default)]
    pub verify_pending: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundPriority {
    pub priority: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cancel {
    #[serde(default)]
    pub reason: String,
    /// for a round cancel: also cancel units currently leased (their next heartbeat gets 409 Conflict)
    #[serde(default)]
    pub include_leased: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelReply {
    pub cancelled: u32,
    pub round: RoundStatus,
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
    /// initial trust tier (defaults to Probation for Worker, Trusted for Trainer/Admin)
    #[serde(default)]
    pub trust: Option<TrustLevel>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenReply {
    pub id: i64,
    /// shown once: the coordinator keeps only its hash
    pub token: String,
}

/// Self-service volunteer signup (`POST /api/v1/register`, rate-limited, always starts at Probation)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
}

/// Submitted by a verifier worker to `POST /units/<id>/verify-complete`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyReport {
    pub mode: VerifyMode,
    pub target_unit: String,
    pub passed: bool,
    pub reason: String,
    /// deals that failed replay or mismatched on rerun
    #[serde(default)]
    pub mismatches: Vec<u64>,
    /// files whose sha256 differed from the claimed manifest
    #[serde(default)]
    pub file_mismatches: Vec<String>,
    #[serde(default)]
    pub elapsed_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetTrust {
    pub trust: TrustLevel,
    #[serde(default)]
    pub reason: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuarantineRequest {
    #[serde(default)]
    pub round_id: Option<i64>,
    #[serde(default)]
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuarantineReply {
    pub units_affected: u32,
    pub credits_affected: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CreditAdjustment {
    pub credit: f64,
    pub reason: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderRerun {
    #[serde(default)]
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxItem {
    pub id: i64,
    pub created: i64,
    pub status: String,
    pub kind: String,
    pub token: Option<i64>,
    pub account: Option<String>,
    pub host: Option<i64>,
    pub host_name: Option<String>,
    pub unit: Option<String>,
    pub verify_unit: Option<String>,
    pub detail: String,
    pub resolved_by: Option<String>,
    pub resolved_at: Option<i64>,
    pub resolution: Option<String>,
}

/// Resolution action for `POST /admin/inbox/<id>/resolve`:
/// `"confirm-cheat"` (ban account, revoke frozen credits, keep quarantine),
/// `"false-positive"` (restore quarantined units and unfreeze credits),
/// `"order-rerun"` (spawn an extra rerun verify unit for the flagged unit), or
/// `"dismiss"` (close without changing state).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveInbox {
    pub action: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostInfo {
    pub id: i64,
    pub name: String,
    pub build: String,
    pub class: String,
    pub strict: bool,
    pub threads: u32,
    pub os: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu: Option<String>,
    pub seen: i64,
    pub accepted_units: u64,
    pub mismatches: u64,
    pub probation_passed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkerInfo {
    pub id: i64,
    pub name: String,
    pub role: Role,
    pub trust: TrustLevel,
    pub created: i64,
    pub revoked: bool,
    pub accepted_units: u64,
    pub mismatches: u64,
    pub extra_audit_until: i64,
    pub credit_total: f64,
    pub credit_pending: f64,
    pub credit_matured: f64,
    pub credit_frozen: f64,
    pub hosts: Vec<HostInfo>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub account: String,
    pub trust: TrustLevel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
    pub accepted_units: u64,
    pub games: u64,
    pub verified_units: u64,
    pub credit: f64,
    pub credit_pending: f64,
    pub credit_matured: f64,
    pub credit_frozen: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewTeam {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TeamInfo {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub created: i64,
    pub members: u32,
    pub accepted_units: u64,
    pub games: u64,
    pub credit: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicRound {
    pub id: i64,
    pub project: String,
    pub run: String,
    pub round: u32,
    pub priority: i32,
    pub trained: bool,
    pub created: i64,
    pub units: BTreeMap<String, u32>,
    pub games: u64,
    pub verify_pending: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeedItem {
    pub unit: String,
    pub project: String,
    pub run: String,
    pub round: u32,
    pub kind: String,
    pub account: String,
    pub host: String,
    pub games: u64,
    pub credit: f64,
    pub done_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributorHost {
    pub name: String,
    pub os: String,
    pub class: String,
    pub threads: u32,
    pub seen: i64,
    pub accepted_units: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContributorProfile {
    pub account: String,
    pub trust: TrustLevel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
    pub created: i64,
    pub accepted_units: u64,
    pub games: u64,
    pub verified_units: u64,
    pub credit: f64,
    pub credit_pending: f64,
    pub credit_matured: f64,
    pub credit_frozen: f64,
    pub badges: Vec<String>,
    pub hosts: Vec<ContributorHost>,
    pub recent: Vec<FeedItem>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyLog {
    pub accepted_units: u64,
    pub verified_units: u64,
    pub rejected_units: u64,
    pub quarantined_units: u64,
    pub events: Vec<AuditEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: i64,
    pub at: i64,
    pub actor: String,
    pub action: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SprtConfig {
    /// Null hypothesis Elo gain (typically 0.0)
    #[serde(default)]
    pub elo0: f64,
    /// Alternative hypothesis Elo gain (typically 5.0 or 10.0)
    #[serde(default = "default_elo1")]
    pub elo1: f64,
    /// False positive rate (typically 0.05)
    #[serde(default = "default_alpha_beta")]
    pub alpha: f64,
    /// False negative rate (typically 0.05)
    #[serde(default = "default_alpha_beta")]
    pub beta: f64,
    /// Maximum fraction of un-reverified games one account may contribute before cross-account re-runs are required
    #[serde(default = "default_max_share")]
    pub max_share: f64,
    /// Promote candidate automatically when SPRT passes and concentration guard is satisfied
    #[serde(default = "default_true")]
    pub auto_promote: bool,
}

fn default_elo1() -> f64 {
    10.0
}

fn default_alpha_beta() -> f64 {
    0.05
}

fn default_max_share() -> f64 {
    0.5
}

fn default_true() -> bool {
    true
}

impl Default for SprtConfig {
    fn default() -> Self {
        Self {
            elo0: 0.0,
            elo1: default_elo1(),
            alpha: default_alpha_beta(),
            beta: default_alpha_beta(),
            max_share: default_max_share(),
            auto_promote: true,
        }
    }
}

/// Publishes an SPRT gate round (`UnitKind::Match` units on reputable/trusted workers)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NewGate {
    pub project: String,
    pub run: String,
    pub round: u32,
    #[serde(default)]
    pub priority: i32,
    pub candidate_sha: String,
    pub candidate_label: String,
    pub baseline_sha: String,
    #[serde(default)]
    pub baseline_label: String,
    #[serde(default)]
    pub sprt: SprtConfig,
    pub units: Vec<WorkUnit>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GateStatus {
    pub id: i64,
    pub round_id: i64,
    pub project: String,
    pub run: String,
    pub round: u32,
    pub candidate_sha: String,
    pub candidate_label: String,
    pub baseline_sha: String,
    pub baseline_label: String,
    /// running, concentration-hold, passed, promoted, failed, inconclusive, held, rolled-back
    pub status: String,
    pub held: bool,
    pub auto_promote: bool,
    pub elo0: f64,
    pub elo1: f64,
    pub alpha: f64,
    pub beta: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub wins: u64,
    pub draws: u64,
    pub losses: u64,
    pub elo: f64,
    pub elo_ci95: f64,
    pub llr: f64,
    pub max_share: f64,
    pub top_account_share: f64,
    pub created: i64,
    pub decided_at: Option<i64>,
    pub decision_reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnchorScore {
    pub opponent: String,
    pub wins: u64,
    pub draws: u64,
    pub losses: u64,
    pub elo: f64,
    pub ci95: f64,
}

/// Publishes an Archbench rating round (`UnitKind::Bench` units against fixed anchors)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NewBench {
    pub project: String,
    pub run: String,
    pub round: u32,
    #[serde(default)]
    pub priority: i32,
    pub net_sha: String,
    pub net_label: String,
    /// Reference Elo of the primary anchor (default 1000.0 for host@s6)
    #[serde(default = "default_anchor_elo")]
    pub anchor_elo: f64,
    pub units: Vec<WorkUnit>,
}

fn default_anchor_elo() -> f64 {
    1000.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BenchStatus {
    pub id: i64,
    pub round_id: i64,
    pub project: String,
    pub run: String,
    pub round: u32,
    pub net_sha: String,
    pub net_label: String,
    pub anchor_elo: f64,
    pub games: u64,
    pub elo: f64,
    pub ci95: f64,
    pub anchors: Vec<AnchorScore>,
    pub created: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub sha256: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_sha: Option<String>,
    /// candidate, promoted, superseded, rejected, rolled-back
    pub status: String,
    pub created: i64,
    pub promoted_at: Option<i64>,
    pub download_url: String,
    pub gate_id: Option<i64>,
    pub gate_status: Option<String>,
    pub gate_elo: Option<f64>,
    pub gate_llr: Option<f64>,
    pub bench_elo: Option<f64>,
    pub bench_ci95: Option<f64>,
    pub bench_games: u64,
    #[serde(default)]
    pub anchors: Vec<AnchorScore>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateAction {
    #[serde(default)]
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackRequest {
    pub target_sha: String,
    pub reason: String,
}

/// The public numbers (`GET /status`, no token): what the tracker draws from
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    /// hosts seen in the last 10 minutes, and their threads
    pub hosts: u32,
    pub threads: u32,
    /// GPU-capable hosts seen in the last 10 minutes
    #[serde(default)]
    pub gpu_hosts: u32,
    pub units: BTreeMap<String, u32>,
    pub games_accepted: u64,
    pub games_last_hour: u64,
    /// total recorded training decisions across accepted units
    #[serde(default)]
    pub total_decisions: u64,
    /// open items in the verification inbox (mismatches and statistical flags)
    #[serde(default)]
    pub open_inbox: u32,
    /// active hosts in the last 10 minutes grouped by os/platform
    #[serde(default)]
    pub platforms: BTreeMap<String, u32>,
    /// highest-priority active round, or the most recent round
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_round: Option<PublicRound>,
}
