/**
 * HTTP and SSE client for the Mana@Home coordinator (`playmat-swarm`, `/api/v1`).
 * Public endpoints require no bearer token; admin endpoints accept an admin token.
 */

export const DEFAULT_SWARM_URL =
  (import.meta.env.VITE_SWARM_URL as string | undefined)?.replace(/\/+$/, '') || 'https://mana.deimos.studio';

export type TrustLevel = 'probation' | 'normal' | 'reputable' | 'trusted' | 'banned';
export type LeaderboardWindow = '24h' | '7d' | '30d' | 'all';

export interface PublicRound {
  id: number;
  project: string;
  run: string;
  round: number;
  priority: number;
  trained: boolean;
  created: number;
  units: Record<string, number>;
  games: number;
  verify_pending: number;
}

export interface SwarmStatus {
  hosts: number;
  threads: number;
  units: Record<string, number>;
  games_accepted: number;
  games_last_hour: number;
  total_decisions: number;
  open_inbox: number;
  platforms: Record<string, number>;
  active_round?: PublicRound | null;
}

export interface FeedItem {
  unit: string;
  project: string;
  run: string;
  round: number;
  kind: string;
  account: string;
  host: string;
  games: number;
  credit: number;
  done_at: number;
}

export interface LeaderboardEntry {
  account: string;
  trust: TrustLevel;
  team?: string | null;
  accepted_units: number;
  games: number;
  verified_units: number;
  credit: number;
  credit_pending: number;
  credit_matured: number;
  credit_frozen: number;
}

export interface TeamInfo {
  id: number;
  name: string;
  description: string;
  created: number;
  members: number;
  accepted_units: number;
  games: number;
  credit: number;
}

export interface ContributorHost {
  name: string;
  os: string;
  class: string;
  threads: number;
  seen: number;
  accepted_units: number;
}

export interface ContributorProfile {
  account: string;
  trust: TrustLevel;
  team?: string | null;
  created: number;
  accepted_units: number;
  games: number;
  verified_units: number;
  credit: number;
  credit_pending: number;
  credit_matured: number;
  credit_frozen: number;
  badges: string[];
  hosts: ContributorHost[];
  recent: FeedItem[];
}

export interface AuditEntry {
  id: number;
  at: number;
  actor: string;
  action: string;
  detail: string;
}

export interface TransparencyLog {
  accepted_units: number;
  verified_units: number;
  rejected_units: number;
  quarantined_units: number;
  events: AuditEntry[];
}

export interface RoundStatus {
  id: number;
  project: string;
  run: string;
  round: number;
  priority: number;
  units: Record<string, number>;
  games: number;
  verify_pending: number;
}

export interface HostInfo {
  id: number;
  name: string;
  build: string;
  class: string;
  strict: boolean;
  threads: number;
  os: string;
  seen: number;
  accepted_units: number;
  mismatches: number;
  probation_passed: boolean;
}

export interface WorkerInfo {
  id: number;
  name: string;
  role: 'worker' | 'trainer' | 'admin';
  trust: TrustLevel;
  created: number;
  revoked: boolean;
  accepted_units: number;
  mismatches: number;
  extra_audit_until: number;
  credit_total: number;
  credit_pending: number;
  credit_matured: number;
  credit_frozen: number;
  hosts: HostInfo[];
}

export interface InboxItem {
  id: number;
  created: number;
  status: string;
  kind: string;
  token: number | null;
  account: string | null;
  host: number | null;
  host_name: string | null;
  unit: string | null;
  verify_unit: string | null;
  detail: string;
  resolved_by: string | null;
  resolved_at: number | null;
  resolution: string | null;
}

export interface TokenReply {
  id: number;
  token: string;
}

export interface QuarantineReply {
  units_affected: number;
  credits_affected: number;
}

export interface AnchorScore {
  opponent: string;
  wins: number;
  draws: number;
  losses: number;
  elo: number;
  ci95: number;
}

export interface GateStatus {
  id: number;
  round_id: number;
  project: string;
  run: string;
  round: number;
  candidate_sha: string;
  candidate_label: string;
  baseline_sha: string;
  baseline_label: string;
  status: string;
  held: boolean;
  auto_promote: boolean;
  elo0: number;
  elo1: number;
  alpha: number;
  beta: number;
  lower_bound: number;
  upper_bound: number;
  wins: number;
  draws: number;
  losses: number;
  elo: number;
  elo_ci95: number;
  llr: number;
  max_share: number;
  top_account_share: number;
  created: number;
  decided_at: number | null;
  decision_reason: string;
}

export interface BenchStatus {
  id: number;
  round_id: number;
  project: string;
  run: string;
  round: number;
  net_sha: string;
  net_label: string;
  anchor_elo: number;
  games: number;
  elo: number;
  ci95: number;
  anchors: AnchorScore[];
  created: number;
  updated_at: number;
}

export interface NetworkInfo {
  sha256: string;
  label: string;
  parent_sha?: string | null;
  status: string;
  created: number;
  promoted_at: number | null;
  download_url: string;
  gate_id: number | null;
  gate_status: string | null;
  gate_elo: number | null;
  gate_llr: number | null;
  bench_elo: number | null;
  bench_ci95: number | null;
  bench_games: number;
  anchors: AnchorScore[];
}

async function request<T>(base: string, path: string, init?: RequestInit & { token?: string }): Promise<T> {
  const headers: Record<string, string> = {};
  if (init?.body) headers['Content-Type'] = 'application/json';
  if (init?.token) headers['Authorization'] = `Bearer ${init.token}`;
  const res = await fetch(`${base.replace(/\/+$/, '')}/api/v1${path}`, { ...init, headers });
  if (!res.ok) {
    let detail = `${res.status} ${res.statusText}`;
    try {
      const body = (await res.json()) as { error?: string };
      if (body.error) detail = body.error;
    } catch {
      // plain text error body
    }
    throw new Error(detail);
  }
  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

export const swarmPublic = {
  status: (base = DEFAULT_SWARM_URL) => request<SwarmStatus>(base, '/status'),
  projects: (base = DEFAULT_SWARM_URL) => request<PublicRound[]>(base, '/projects'),
  feed: (limit = 30, base = DEFAULT_SWARM_URL) => request<FeedItem[]>(base, `/feed?limit=${limit}`),
  leaderboard: (window: LeaderboardWindow = 'all', base = DEFAULT_SWARM_URL) =>
    request<LeaderboardEntry[]>(base, `/leaderboard?window=${window}`),
  teams: (window: LeaderboardWindow = 'all', base = DEFAULT_SWARM_URL) =>
    request<TeamInfo[]>(base, `/teams?window=${window}`),
  contributor: (name: string, base = DEFAULT_SWARM_URL) =>
    request<ContributorProfile>(base, `/contributors/${encodeURIComponent(name)}`),
  transparency: (base = DEFAULT_SWARM_URL) => request<TransparencyLog>(base, '/transparency'),
  networks: (base = DEFAULT_SWARM_URL) => request<NetworkInfo[]>(base, '/networks'),
  gates: (base = DEFAULT_SWARM_URL) => request<GateStatus[]>(base, '/gates'),
  benches: (base = DEFAULT_SWARM_URL) => request<BenchStatus[]>(base, '/benches'),
};

export function createSwarmAdminClient(base: string, token: string) {
  const call = <T>(method: string, path: string, body?: unknown) =>
    request<T>(base, path, {
      method,
      token,
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });
  return {
    status: () => call<SwarmStatus>('GET', '/status'),
    rounds: () => call<RoundStatus[]>('GET', '/rounds'),
    setPriority: (roundId: number, priority: number) =>
      call<RoundStatus>('POST', `/rounds/${roundId}/priority`, { priority }),
    cancelRound: (roundId: number, reason: string, includeLeased: boolean) =>
      call<{ cancelled: number; round: RoundStatus }>('POST', `/rounds/${roundId}/cancel`, {
        reason,
        include_leased: includeLeased,
      }),
    markTrained: (roundId: number) => call<void>('POST', `/admin/rounds/${roundId}/trained`, {}),
    orderRerun: (unitId: string, reason: string) =>
      call<{ verify_unit: string }>('POST', `/admin/units/${encodeURIComponent(unitId)}/rerun`, { reason }),
    reissueUnit: (unitId: string) => call<void>('POST', `/admin/units/${encodeURIComponent(unitId)}/reissue`, {}),
    workers: () => call<WorkerInfo[]>('GET', '/admin/workers'),
    setTrust: (tokenId: number, trust: TrustLevel, reason: string) =>
      call<void>('POST', `/admin/tokens/${tokenId}/trust`, { trust, reason }),
    revokeToken: (tokenId: number) => call<void>('POST', `/admin/tokens/${tokenId}/revoke`, {}),
    quarantineToken: (tokenId: number, reason: string, roundId?: number) =>
      call<QuarantineReply>('POST', `/admin/tokens/${tokenId}/quarantine`, {
        reason,
        round_id: roundId ?? null,
      }),
    restoreToken: (tokenId: number, reason: string, roundId?: number) =>
      call<QuarantineReply>('POST', `/admin/tokens/${tokenId}/restore`, {
        reason,
        round_id: roundId ?? null,
      }),
    adjustCredit: (tokenId: number, credit: number, reason: string) =>
      call<void>('POST', `/admin/tokens/${tokenId}/credit`, { credit, reason }),
    newToken: (role: 'worker' | 'trainer' | 'admin', name: string, trust?: TrustLevel) =>
      call<TokenReply>('POST', '/tokens', { role, name, trust: trust ?? null }),
    inbox: (status?: string) => call<InboxItem[]>('GET', status ? `/admin/inbox?status=${status}` : '/admin/inbox'),
    resolveInbox: (id: number, action: 'confirm-cheat' | 'false-positive' | 'order-rerun' | 'dismiss', reason: string) =>
      call<void>('POST', `/admin/inbox/${id}/resolve`, { action, reason }),
    networks: () => call<NetworkInfo[]>('GET', '/networks'),
    gates: () => call<GateStatus[]>('GET', '/gates'),
    benches: () => call<BenchStatus[]>('GET', '/benches'),
    holdGate: (gateId: number, reason: string) =>
      call<GateStatus>('POST', `/admin/gates/${gateId}/hold`, { reason }),
    resumeGate: (gateId: number) => call<GateStatus>('POST', `/admin/gates/${gateId}/resume`, {}),
    promoteGate: (gateId: number, reason: string) =>
      call<GateStatus>('POST', `/admin/gates/${gateId}/promote`, { reason }),
    rollbackNetwork: (targetSha: string, reason: string) =>
      call<void>('POST', '/admin/networks/rollback', { target_sha: targetSha, reason }),
    audit: (params?: { action?: string; actor?: string; limit?: number }) => {
      const qs = new URLSearchParams();
      if (params?.action) qs.set('action', params.action);
      if (params?.actor) qs.set('actor', params.actor);
      if (params?.limit) qs.set('limit', String(params.limit));
      const suffix = qs.toString() ? `?${qs.toString()}` : '';
      return call<AuditEntry[]>('GET', `/admin/audit${suffix}`);
    },
  };
}

export function formatUnixAge(unixSecs: number): string {
  if (!unixSecs) return 'never';
  const diff = Math.max(0, Math.floor(Date.now() / 1000) - unixSecs);
  if (diff < 60) return `${diff}s ago`;
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return `${Math.floor(diff / 86400)}d ago`;
}

export function formatCredit(n: number): string {
  if (!Number.isFinite(n)) return '0';
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
  if (n >= 10_000) return `${(n / 1_000).toFixed(1)}k`;
  return n.toFixed(1);
}

export function roundProgress(units: Record<string, number>): { done: number; total: number; pct: number } {
  const accepted = units.accepted ?? 0;
  const complete = units.complete ?? 0;
  const cancelled = units.cancelled ?? 0;
  const total = Object.values(units).reduce((a, b) => a + b, 0) - cancelled;
  const done = accepted + complete;
  const pct = total > 0 ? Math.min(100, Math.round((done / total) * 100)) : 100;
  return { done, total: Math.max(total, done), pct };
}
