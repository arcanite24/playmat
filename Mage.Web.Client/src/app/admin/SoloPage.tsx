import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { RefreshCw, Trash2, Upload } from 'lucide-react';
import { useRef, useState } from 'react';
import type { createApi } from '../../protocol/generated/api';
import type { BatchJob, ContentPackView } from '../../protocol/generated/views';
import { Button } from '../ui/Button';
import { atLeast, describeAge, formatDate, type StaffRole } from './adminFormat';
import styles from './AdminScreen.module.css';

type AdminApi = ReturnType<typeof createApi>;

const BATCH_KEY = ['admin', 'batch'];
const PACKS_KEY = ['admin', 'contentPacks'];

/** Single player for staff: the AI-against-AI batch runner and the content packs. */
export function SoloPage({ api, role }: { api: AdminApi; role: StaffRole }) {
  return (
    <div className={styles.overview}>
      <BatchBlock api={api} />
      <PacksBlock api={api} role={role} />
    </div>
  );
}

function BatchBlock({ api }: { api: AdminApi }) {
  const client = useQueryClient();
  const status = useQuery({ queryKey: BATCH_KEY, queryFn: () => api.adminBatchJobs(), refetchInterval: 5_000 });
  const cancel = useMutation({
    mutationFn: (id: string) => api.adminBatchCancel(id),
    onSuccess: () => void client.invalidateQueries({ queryKey: BATCH_KEY }),
  });
  const data = status.data;
  const now = status.dataUpdatedAt;
  return (
    <section aria-labelledby="batch-runner" className={styles.block}>
      <h2 id="batch-runner">Batch games</h2>
      <p className={styles.quiet}>
        AI-against-AI games players queue (deck tests, matchups). Games at once and the daily quota per player are under Settings.
      </p>
      {status.error && <p className={styles.error} role="alert">{String(status.error)}</p>}
      {data && (
        <dl className={[styles.numbers, styles.numbersTight].join(' ')}>
          <Stat label="Playing now" value={`${data.gamesInFlight ?? 0} / ${data.concurrency ?? 0}`} />
          <Stat label="Per player a day" value={String(data.gamesPerDay ?? 0)} />
          <Stat label="Waiting" value={String((data.jobs ?? []).filter((job) => job.status === 'queued').length)} />
        </dl>
      )}
      {data && data.concurrency === 0 && <p className={styles.alarm}>The runner is stopped: games at once is 0.</p>}
      {data && (data.jobs ?? []).length === 0 && <p className={styles.quiet}>No batches yet.</p>}
      {data && (data.jobs ?? []).length > 0 && (
        <table className={styles.table}>
          <thead>
            <tr><th>Asked</th><th>By</th><th>Decks</th><th>Games</th><th>Score</th><th>Status</th><th /></tr>
          </thead>
          <tbody>
            {(data.jobs ?? []).map((job) => (
              <tr key={job.id}>
                <td title={formatDate(job.created ?? 0)}>{describeAge(job.created ?? 0, now) || formatDate(job.created ?? 0)}</td>
                <td>{job.user}</td>
                <td className={styles.clip}>{job.nameA} vs {job.nameB}</td>
                <td>{job.done ?? 0} / {job.games ?? 0}</td>
                <td>{score(job)}</td>
                <td title={job.error ?? undefined}>{job.status}</td>
                <td className={styles.actions}>
                  {(job.status === 'queued' || job.status === 'running') && (
                    <Button size="sm" variant="quiet" disabled={cancel.isPending} onClick={() => cancel.mutate(job.id!)}>Stop</Button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {cancel.error && <p className={styles.error} role="alert">{cancel.error.message}</p>}
    </section>
  );
}

function score(job: BatchJob): string {
  const result = job.result;
  if (!result || (job.done ?? 0) === 0) return '';
  const parts = [`${result.winsA ?? 0}–${result.winsB ?? 0}`];
  if (result.draws) parts.push(`${result.draws} drawn`);
  if (result.errors) parts.push(`${result.errors} failed`);
  if (result.mined) parts.push(`${result.mined} ${result.mined === 1 ? 'puzzle' : 'puzzles'} mined`);
  return parts.join(', ');
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className={styles.stat}>
      <dt>{label}</dt>
      <dd>{value}</dd>
    </div>
  );
}

function PacksBlock({ api, role }: { api: AdminApi; role: StaffRole }) {
  const client = useQueryClient();
  const packs = useQuery({ queryKey: PACKS_KEY, queryFn: () => api.adminContentPacks() });
  const file = useRef<HTMLInputElement>(null);
  const [message, setMessage] = useState<string | null>(null);
  const owner = atLeast(role, 'owner');
  const done = (views: ContentPackView[]) => client.setQueryData(PACKS_KEY, views);
  const reload = useMutation({ mutationFn: () => api.adminContentPackReload(), onSuccess: done });
  const remove = useMutation({ mutationFn: (id: string) => api.adminContentPackRemove(id), onSuccess: done });
  const upload = useMutation({
    mutationFn: async (zip: File) => api.adminContentPackUpload(await toBase64(zip)),
    onSuccess: (view) => {
      setMessage(view.loaded ? `${view.name ?? view.id} is loaded.` : `${view.id} was saved but isn't loaded: see its problems below.`);
      void client.invalidateQueries({ queryKey: PACKS_KEY });
    },
  });
  const error = packs.error ?? reload.error ?? remove.error ?? upload.error;
  const list = packs.data ?? [];
  return (
    <section aria-labelledby="content-packs" className={styles.block}>
      <div className={styles.sectionHead}>
        <h2 id="content-packs">Content packs</h2>
        <span className={styles.actions}>
          <Button size="sm" variant="quiet" busy={reload.isPending} onClick={() => reload.mutate()}>
            <RefreshCw size={14} aria-hidden="true" /> Reload
          </Button>
          {owner && (
            <>
              <input ref={file} type="file" accept=".zip,application/zip" hidden
                onChange={(event) => {
                  const picked = event.target.files?.[0];
                  if (picked) upload.mutate(picked);
                  event.target.value = '';
                }} />
              <Button size="sm" busy={upload.isPending} onClick={() => file.current?.click()}>
                <Upload size={14} aria-hidden="true" /> Upload a pack
              </Button>
            </>
          )}
        </span>
      </div>
      <p className={styles.quiet}>
        Campaigns, puzzles, weekly challenges and opponents from outside the server&rsquo;s code: a zip with pack.json and decks/*.dck,
        or a folder in db/content-packs. A pack with problems stays out and says why.
      </p>
      {message && <p className={styles.quiet} role="status">{message}</p>}
      {error && <p className={styles.error} role="alert">{error instanceof Error ? error.message : String(error)}</p>}
      {packs.data && list.length === 0 && <p className={styles.quiet}>No packs on this server.</p>}
      {list.length > 0 && (
        <table className={styles.table}>
          <thead>
            <tr><th>Pack</th><th>Version</th><th>Contents</th><th>State</th><th /></tr>
          </thead>
          <tbody>
            {list.map((pack) => (
              <tr key={pack.id}>
                <td>
                  <b>{pack.name ?? pack.id}</b>
                  <div className={styles.quiet}>{pack.id}{pack.author ? ` · ${pack.author}` : ''}</div>
                </td>
                <td>{pack.version ?? ''}</td>
                <td>{contents(pack)}</td>
                <td>
                  {pack.loaded ? 'Loaded' : <span className={styles.alarm}>Not loaded</span>}
                  {(pack.problems ?? []).length > 0 && (
                    <ul className={styles.quiet}>
                      {(pack.problems ?? []).map((problem) => <li key={problem}>{problem}</li>)}
                    </ul>
                  )}
                </td>
                <td className={styles.actions}>
                  {owner && (
                    <Button size="sm" variant="quiet" aria-label={`Remove ${pack.id}`} disabled={remove.isPending} onClick={() => {
                      if (window.confirm(`Remove the pack ${pack.id}? Its content leaves the server.`)) remove.mutate(pack.id!);
                    }}>
                      <Trash2 size={14} aria-hidden="true" />
                    </Button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}

function contents(pack: ContentPackView): string {
  const parts: string[] = [];
  const add = (count: number | undefined, one: string, many: string) => {
    if (count) parts.push(`${count} ${count === 1 ? one : many}`);
  };
  add(pack.campaigns, 'campaign', 'campaigns');
  add(pack.puzzles, 'puzzle', 'puzzles');
  add(pack.challenges, 'challenge', 'challenges');
  add(pack.opponents, 'opponent', 'opponents');
  add(pack.decks, 'deck', 'decks');
  return parts.join(', ');
}

async function toBase64(file: File): Promise<string> {
  const bytes = new Uint8Array(await file.arrayBuffer());
  let binary = '';
  for (let i = 0; i < bytes.length; i += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  }
  return btoa(binary);
}
