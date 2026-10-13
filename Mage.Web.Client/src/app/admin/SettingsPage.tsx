import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Send, Wrench, X } from 'lucide-react';
import { useState, type FormEvent } from 'react';
import type { createApi } from '../../protocol/generated/api';
import type { ServerSetting, ServerSettings } from '../../protocol/generated/views';
import { Button } from '../ui/Button';
import { Field } from '../ui/Field';
import { atLeast, describeAge, formatDate, type StaffRole } from './adminFormat';
import styles from './AdminScreen.module.css';

type AdminApi = ReturnType<typeof createApi>;

const SETTINGS_KEY = ['admin', 'settings'];

/** What each setting is, and the values it takes (none: a number). */
const ABOUT: Record<string, { label: string; help: string; choices?: { value: string; label: string }[]; unit?: string }> = {
  registration: {
    label: 'New accounts',
    help: 'Invite only: people need an invite link, or a place on the allowlist, to make an account.',
    choices: [{ value: 'open', label: 'Anyone can register' }, { value: 'invite', label: 'Invite only' }],
  },
  career: {
    label: 'Career',
    help: 'The single-player Career. Off hides it; progress is kept for when it comes back.',
    choices: [{ value: 'on', label: 'On' }, { value: 'off', label: 'Off' }],
  },
  maxAiOpponents: { label: 'AI opponents per table', help: 'Empty: as many as the table has seats.', unit: 'opponents' },
  batchConcurrency: { label: 'Batch games at once', help: 'AI-against-AI games the batch runner plays at the same time. 0 stops it.', unit: 'games' },
  batchGamesPerDay: { label: 'Batch games per player', help: 'How many AI-against-AI games one player may ask for in a day.', unit: 'games a day' },
  archmageDailyCents: { label: 'Archmage daily cap', help: "What Career's tier-five Archmage AI may spend a day on model calls, in cents. 0 keeps it off; it plays as the usual AI until the hybrid AI lands.", unit: 'cents a day' },
  maxSecondsIdle: { label: 'Time to answer', help: 'How long a player may take on one decision before they lose the game.', unit: 'seconds' },
  discordTables: { label: 'Tables opening', help: 'Public tables with two or more people.', choices: [{ value: 'on', label: 'Post' }, { value: 'off', label: 'Off' }] },
  discordEvents: { label: 'Events starting', help: 'Tournaments, when they start.', choices: [{ value: 'on', label: 'Post' }, { value: 'off', label: 'Off' }] },
  discordMatches: { label: 'Matches starting', help: 'Standalone matches between people.', choices: [{ value: 'on', label: 'Post' }, { value: 'off', label: 'Off' }] },
  discordStaff: { label: 'Staff news', help: 'Access requests, reports, staff changes, maintenance and backup trouble.', choices: [{ value: 'on', label: 'Post' }, { value: 'off', label: 'Off' }] },
};

const DISCORD = new Set(['discordTables', 'discordEvents', 'discordMatches', 'discordStaff']);

/** Settings that change without a restart, the Discord channels, and planned maintenance. */
export function SettingsPage({ api, role }: { api: AdminApi; role: StaffRole }) {
  const settings = useQuery({ queryKey: SETTINGS_KEY, queryFn: () => api.adminSettings(), refetchInterval: 30_000 });
  const data: ServerSettings | undefined = settings.data;
  if (settings.error) return <p className={styles.error}>{String(settings.error)}</p>;
  if (!data) return <p className={styles.quiet}>Loading…</p>;
  const list = data.settings ?? [];
  const now = settings.dataUpdatedAt;
  return (
    <div className={styles.overview}>
      {atLeast(role, 'owner') && <MaintenanceBlock api={api} data={data} now={now} />}
      <section aria-labelledby="server-settings" className={styles.block}>
        <h2 id="server-settings">Server</h2>
        <p className={styles.quiet}>These apply at once and stay after a restart. Back to config returns a setting to what the server&rsquo;s config says.</p>
        <ul className={styles.settings}>
          {list.filter((s) => !DISCORD.has(s.name ?? '')).map((setting) => <SettingRow key={setting.name} api={api} setting={setting} now={now} />)}
        </ul>
      </section>
      <section aria-labelledby="discord" className={styles.block}>
        <h2 id="discord">Discord</h2>
        <DiscordChannels api={api} data={data} />
        <ul className={styles.settings}>
          {list.filter((s) => DISCORD.has(s.name ?? '')).map((setting) => <SettingRow key={setting.name} api={api} setting={setting} now={now} />)}
        </ul>
      </section>
    </div>
  );
}

function SettingRow({ api, setting, now }: { api: AdminApi; setting: ServerSetting; now: number }) {
  const client = useQueryClient();
  const name = setting.name ?? '';
  const about = ABOUT[name] ?? { label: name, help: '' };
  const [draft, setDraft] = useState(setting.value ?? '');
  const save = useMutation({
    mutationFn: (value: string) => api.adminSetSetting(name, value),
    onSuccess: (next) => client.setQueryData(SETTINGS_KEY, next),
  });
  const source = setting.changed
    ? `Changed by ${setting.changedBy} ${describeAge(setting.changedAt ?? 0, now) || formatDate(setting.changedAt ?? 0)}`
    : DISCORD.has(name) ? 'On unless turned off here' : 'From the config';
  const id = `setting-${name}`;
  return (
    <li className={styles.setting}>
      <div className={styles.settingText}>
        <label htmlFor={id} className={styles.settingLabel}>{about.label}</label>
        <p className={styles.quiet}>{about.help}</p>
        <p className={styles.quiet}>{setting.locked ?? source}{setting.changed && !setting.locked && <> · config: {setting.configValue || 'none'}</>}</p>
      </div>
      <div className={styles.inline}>
        {about.choices ? (
          <select id={id} className={styles.settingInput} value={setting.value ?? ''} disabled={!!setting.locked || save.isPending}
            onChange={(event) => save.mutate(event.target.value)}>
            {about.choices.map((choice) => <option key={choice.value} value={choice.value}>{choice.label}</option>)}
          </select>
        ) : (
          <form className={styles.inline} onSubmit={(event: FormEvent) => { event.preventDefault(); save.mutate(draft.trim()); }}>
            <input id={id} className={styles.settingInput} inputMode="numeric" value={draft} disabled={!!setting.locked}
              placeholder="no limit" aria-describedby={`${id}-unit`} onChange={(event) => setDraft(event.target.value)} />
            <span id={`${id}-unit`} className={styles.quiet}>{about.unit}</span>
            <Button size="sm" type="submit" disabled={save.isPending || draft.trim() === (setting.value ?? '')}>Save</Button>
          </form>
        )}
        {setting.changed && !setting.locked && (
          <Button size="sm" variant="quiet" disabled={save.isPending}
            onClick={() => save.mutate('', { onSuccess: (next) => setDraft(next.settings?.find((s) => s.name === name)?.value ?? '') })}>Back to config</Button>
        )}
      </div>
      {save.error && <p className={styles.error} role="alert">{save.error.message}</p>}
    </li>
  );
}

function DiscordChannels({ api, data }: { api: AdminApi; data: ServerSettings }) {
  const [result, setResult] = useState<{ channel: string; text: string; failed: boolean } | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  async function test(channel: string) {
    setBusy(channel);
    setResult(null);
    try {
      setResult({ channel, text: await api.adminDiscordTest(channel), failed: false });
    } catch (caught) {
      setResult({ channel, text: caught instanceof Error ? caught.message : String(caught), failed: true });
    } finally {
      setBusy(null);
    }
  }
  const channels = [
    { id: 'lobby', label: 'Lobby channel', set: !!data.discordLobby, unset: 'No webhook: set XMAGE_DISCORD_WEBHOOK on the server.' },
    {
      id: 'staff', label: 'Staff channel', set: !!data.discordStaff || !!data.discordLobby,
      unset: 'No webhook: set XMAGE_DISCORD_STAFF_WEBHOOK on the server.',
      note: data.discordStaff ? undefined : 'Shares the lobby channel (no staff webhook of its own).',
    },
  ];
  return (
    <ul className={styles.settings}>
      {channels.map((channel) => (
        <li key={channel.id} className={styles.setting}>
          <div className={styles.settingText}>
            <span className={styles.settingLabel}>{channel.label}</span>
            <p className={styles.quiet}>{channel.set ? (channel.note ?? 'Webhook set.') : channel.unset} The address stays on the server.</p>
          </div>
          <div className={styles.inline}>
            <Button size="sm" variant="quiet" icon={<Send size={14} />} disabled={!channel.set || busy !== null} onClick={() => test(channel.id)}>
              {busy === channel.id ? 'Posting…' : 'Post a test'}
            </Button>
          </div>
          {result?.channel === channel.id && <p className={result.failed ? styles.error : styles.quiet} role="status">{result.text}</p>}
        </li>
      ))}
    </ul>
  );
}

const WHEN = [
  { minutes: 0, label: 'now' },
  { minutes: 5, label: 'in 5 minutes' },
  { minutes: 15, label: 'in 15 minutes' },
  { minutes: 30, label: 'in 30 minutes' },
  { minutes: 60, label: 'in an hour' },
];

function MaintenanceBlock({ api, data, now }: { api: AdminApi; data: ServerSettings; now: number }) {
  const client = useQueryClient();
  const [minutes, setMinutes] = useState(15);
  const [message, setMessage] = useState('');
  const [restart, setRestart] = useState(true);
  const act = useMutation({
    mutationFn: (work: () => Promise<ServerSettings>) => work(),
    onSuccess: (next) => client.setQueryData(SETTINGS_KEY, next),
  });
  const plan = data.maintenance;
  return (
    <section aria-labelledby="maintenance" className={styles.block}>
      <h2 id="maintenance">Maintenance</h2>
      {plan ? (
        <div className={styles.maintenance}>
          <p>
            <strong>{(plan.startsAt ?? 0) > now ? `Planned for ${formatDate(plan.startsAt ?? 0)}` : 'Under way'}</strong>
            {' '}· announced by {plan.by}{plan.restartWhenIdle ? '; the server restarts once no game is running' : ''}.
          </p>
          {plan.message && <blockquote>{plan.message}</blockquote>}
          <p className={styles.quiet}>No new tables can start. Players were warned and see a banner.</p>
          <div className={styles.inline}>
            <Button size="sm" variant="quiet" icon={<X size={14} />} disabled={act.isPending} onClick={() => act.mutate(() => api.adminMaintenanceCancel())}>Call it off</Button>
          </div>
        </div>
      ) : (
        <form className={styles.block} onSubmit={(event: FormEvent) => { event.preventDefault(); act.mutate(() => api.adminMaintenance(minutes, message.trim() || null, restart)); }}>
          <p className={styles.quiet}>Stops new tables at once, warns everyone online, and shows a banner until it is over. Games in progress can finish.</p>
          <div className={styles.inline}>
            <label className={styles.select}>
              <span>Starts</span>
              <select value={minutes} onChange={(event) => setMinutes(Number(event.target.value))}>
                {WHEN.map((w) => <option key={w.minutes} value={w.minutes}>{w.label}</option>)}
              </select>
            </label>
            <Field label="Message (optional)" value={message} maxLength={300} placeholder="e.g. Back in ten minutes with the new cards."
              onChange={(event) => setMessage(event.target.value)} />
          </div>
          <label className={styles.check}>
            <input type="checkbox" checked={restart} onChange={(event) => setRestart(event.target.checked)} />
            Restart the server once the time has come and no game is running
          </label>
          <div className={styles.inline}>
            <Button size="sm" type="submit" icon={<Wrench size={14} />} disabled={act.isPending}>Announce maintenance</Button>
          </div>
        </form>
      )}
      {act.error && <p className={styles.error} role="alert">{act.error.message}</p>}
    </section>
  );
}
