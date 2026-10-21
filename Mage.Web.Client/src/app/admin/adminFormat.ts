/** Formatting for the admin console. */

export const MUTE_CHOICES = [
  { minutes: 60, label: 'an hour' },
  { minutes: 24 * 60, label: 'a day' },
  { minutes: 7 * 24 * 60, label: 'a week' },
  { minutes: 30 * 24 * 60, label: 'a month' },
] as const;

export function formatBytes(value: number): string {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(value < 10 * 1024 ? 1 : 0)} KB`;
  if (value < 1024 * 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MB`;
  return `${(value / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

export function formatDuration(millis: number): string {
  const minutes = Math.max(0, Math.floor(millis / 60_000));
  if (minutes < 60) return `${minutes} min`;
  const hours = Math.floor(minutes / 60);
  if (hours < 48) return `${hours} h ${minutes % 60} min`;
  return `${Math.floor(hours / 24)} days ${hours % 24} h`;
}

export function describeAge(at: number, now: number): string {
  if (!at) return '';
  const seconds = Math.max(0, Math.round((now - at) / 1000));
  if (seconds < 60) return 'just now';
  return `${formatDuration(seconds * 1000)} ago`;
}

export type InviteState = 'active' | 'used up' | 'expired' | 'revoked';

/** Whether an invite link still works, and if not, why. */
export function inviteState(invite: { revoked?: boolean; uses?: number; maxUses?: number; expiresAt?: number }, now: number): InviteState {
  if (invite.revoked) return 'revoked';
  if ((invite.uses ?? 0) >= (invite.maxUses ?? 0)) return 'used up';
  if (now >= (invite.expiresAt ?? 0)) return 'expired';
  return 'active';
}

/** The link to send: it opens registration with the invite filled in. */
export function inviteLink(code: string, origin: string): string {
  return `${origin.replace(/\/+$/, '')}/join?invite=${encodeURIComponent(code)}`;
}

export function formatDate(at: number): string {
  if (!at) return '';
  return new Date(at).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
}

export type StaffRole = 'moderator' | 'admin' | 'owner';

const RANK: Record<StaffRole, number> = { moderator: 1, admin: 2, owner: 3 };

/** Each role can do what the ones below it can. */
export function atLeast(role: StaffRole | null | undefined, least: StaffRole): boolean {
  return !!role && (RANK[role] ?? 0) >= RANK[least];
}

/** A support payment's amount in its own currency: "$5.00", "€3.00". */
export function money(cents: number, currency = 'USD'): string {
  try {
    return new Intl.NumberFormat('en', { style: 'currency', currency }).format(cents / 100);
  } catch {
    return `${(cents / 100).toFixed(2)} ${currency}`;
  }
}
