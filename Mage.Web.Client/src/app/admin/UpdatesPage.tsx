import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Check, Copy, RefreshCw, RotateCcw, Download } from 'lucide-react';
import { useState, type ReactNode } from 'react';
import type { createApi } from '../../protocol/generated/api';
import type { UpdateJob, UpdateStatus, UpdaterInfo } from '../../protocol/generated/views';
import { APP_VERSION } from '../brand';
import { Button } from '../ui/Button';
import { atLeast, describeAge, type StaffRole } from './adminFormat';
import styles from './AdminScreen.module.css';

type AdminApi = ReturnType<typeof createApi>;

const COMPOSE = 'docker compose -f ops/web/docker-compose.yml --env-file ops/web/.env';

/** What the running build is called: its image tag, else its commit. */
function describeBuild(version?: string, commit?: string): string {
  if (version) return version;
  return commit ? `commit ${commit.slice(0, 7)}` : 'an unknown build';
}

/**
 * Updates (admins and the owner): whether a newer published image is out than the one this server runs, what it
 * brings, and how to get it: the Update button (the owner, when the updater runs) or the commands. The server asks the
 * registry and GitHub (UpdateCheck), keeps the answer for an hour, and lets a refresh go once a minute; the updater's
 * state is read fresh each time, every few seconds while a job runs.
 */
export function UpdatesPage({ api, role }: { api: AdminApi; role: StaffRole }) {
  const client = useQueryClient();
  const [refreshing, setRefreshing] = useState(false);
  const status = useQuery({
    queryKey: ['admin', 'update'],
    queryFn: () => api.adminUpdateStatus(false),
    staleTime: 5 * 60_000,
    refetchInterval: (query) => (query.state.data?.updater?.busy ? 3_000 : false),
  });

  async function refresh() {
    setRefreshing(true);
    try {
      client.setQueryData(['admin', 'update'], await api.adminUpdateStatus(true));
    } finally {
      setRefreshing(false);
    }
  }

  const data: UpdateStatus | undefined = status.data;
  if (status.error && !data) return <p className={styles.error}>{String(status.error)}</p>;
  if (!data) return <p className={styles.quiet}>Checking for updates…</p>;
  const current = describeBuild(data.currentVersion, data.currentCommit);
  const latest = describeBuild(data.latestVersion, data.latestCommit);
  const channel = data.channel === 'release' ? 'release' : 'master';
  const changes = data.changes ?? [];

  return (
    <div className={styles.overview}>
      <section aria-labelledby="update-state" className={styles.block}>
        <div className={styles.sectionHead}>
          <h2 id="update-state">Updates</h2>
          {data.enabled && (
            <Button size="sm" variant="quiet" icon={<RefreshCw size={14} />} busy={refreshing} onClick={() => void refresh()}>
              Check again
            </Button>
          )}
        </div>
        <p className={styles.lede}>
          {data.published
            ? <>This server runs the published image <span className={styles.mono}>{current}</span>, which follows {channel === 'release' ? 'releases' : 'master'}.</>
            : <>This server runs a local build ({current}). It's compared with master's newest published image.</>}
        </p>
        {!data.enabled ? (
          <p className={styles.quiet}>Update checks are off (XMAGE_UPDATE_CHECK=off in .env).</p>
        ) : data.error ? (
          <p className={styles.error}>{data.error}</p>
        ) : data.updateAvailable ? (
          <p className={styles.maintenance}>
            {latest} is out{data.commitsBehind ? `: ${data.commitsBehind} ${data.commitsBehind === 1 ? 'change' : 'changes'} newer than this server` : ''}.
          </p>
        ) : data.updateAvailable === false ? (
          <p>Up to date: nothing newer has been published on {channel}.</p>
        ) : (
          <p className={styles.quiet}>Can't tell whether {latest} is newer than this build.</p>
        )}
        {data.enabled && (data.checkedAt ?? 0) > 0 && (
          <p className={styles.quiet}>Checked {describeAge(data.checkedAt ?? 0, status.dataUpdatedAt)}.</p>
        )}
      </section>

      {data.updateAvailable && (changes.length > 0 || data.notesUrl) && (
        <section aria-labelledby="update-changes" className={styles.block}>
          <h2 id="update-changes">What's new</h2>
          {changes.length > 0 && (
            <ul className={styles.list}>
              {changes.map((change) => (
                <li key={change.sha}>
                  {change.url ? <a href={change.url} target="_blank" rel="noreferrer">{change.title}</a> : change.title}
                </li>
              ))}
            </ul>
          )}
          {data.notesUrl && (
            <p className={styles.quiet}>
              <a href={data.notesUrl} target="_blank" rel="noreferrer">
                {channel === 'release' ? 'Release notes on GitHub' : 'Every change on GitHub'}
              </a>
              {(data.commitsBehind ?? 0) > changes.length ? ` (the newest ${changes.length} are listed here)` : ''}
            </p>
          )}
        </section>
      )}

      <UpdateButton
        api={api}
        owner={atLeast(role, 'owner')}
        status={data}
        // the server restarts during a job: its calls fail until it's back (and this session with them)
        lost={!!status.error}
        now={status.dataUpdatedAt}
        onChange={(next) => client.setQueryData(['admin', 'update'], next)}
      />

      <section aria-labelledby="update-how" className={styles.block}>
        <h2 id="update-how">{data.updater?.ready ? 'Update by hand' : 'How to update'}</h2>
        {data.published ? (
          <>
            <p>
              On the server, in the folder you cloned, pull the new images and restart on them. Volumes and .env are
              kept. If .env pins the images by digest (<span className={styles.mono}>@sha256:…</span>), set them to the
              tags below first.
            </p>
            <Commands lines={[
              `XMAGE_SERVER_IMAGE=${data.serverImage}`,
              `XMAGE_WEB_IMAGE=${data.webImage}`,
            ]} label="ops/web/.env" />
            <Commands lines={['git pull', `${COMPOSE} pull xmage web`, `${COMPOSE} up -d --no-build`]} label="Shell" />
          </>
        ) : (
          <>
            <p>
              Switch to the published images and you won't have to build again: add these two lines to
              {' '}<span className={styles.mono}>ops/web/.env</span>, then pull and restart. Volumes and settings are kept.
            </p>
            <Commands lines={[
              `XMAGE_SERVER_IMAGE=${data.serverImage}`,
              `XMAGE_WEB_IMAGE=${data.webImage}`,
            ]} label="ops/web/.env" />
            <Commands lines={['git pull', `${COMPOSE} pull xmage web`, `${COMPOSE} up -d --no-build`]} label="Shell" />
            <p className={styles.quiet}>To build from the checkout again, empty those two lines and run {COMPOSE} up -d --build.</p>
          </>
        )}
        <p className={styles.quiet}>
          With games running, announce maintenance in Settings first: the restart ends every game in progress.
        </p>
      </section>
    </div>
  );
}

function Commands({ lines, label }: { lines: string[]; label: string }) {
  const [copied, setCopied] = useState(false);
  const text = lines.join('\n');
  async function copy() {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2_000);
    } catch {
      setCopied(false);
    }
  }
  return (
    <figure className={styles.commands}>
      <figcaption>
        <span>{label}</span>
        <Button size="sm" variant="quiet" icon={copied ? <Check size={14} /> : <Copy size={14} />} onClick={() => void copy()}>
          {copied ? 'Copied' : 'Copy'}
        </Button>
      </figcaption>
      <pre>{text}</pre>
    </figure>
  );
}

const STEPS: { id: string; label: string }[] = [
  { id: 'pull', label: 'Download' },
  { id: 'backup', label: 'Back up' },
  { id: 'drain', label: 'Warn players' },
  { id: 'restart', label: 'Restart' },
];

const RESULTS: Record<string, string> = { ok: 'Done', failed: 'Failed', 'rolled-back': 'Failed, went back' };

/**
 * The Update button (the owner only): asks the updater, the compose service with the Docker socket, to update to the
 * newest version or go back to the one before, and follows its job. Without the updater it says how to turn it on.
 */
function UpdateButton({ api, owner, status, lost, now, onChange }: {
  api: AdminApi;
  owner: boolean;
  status: UpdateStatus;
  lost: boolean;
  /** when the status was read */
  now: number;
  onChange(next: UpdateStatus): void;
}) {
  const [confirming, setConfirming] = useState<'update' | 'rollback' | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [asking, setAsking] = useState(false);
  const updater: UpdaterInfo | undefined = status.updater;
  const job: UpdateJob | undefined = updater?.job;
  const busy = !!updater?.busy;
  const target = status.latestVersion;

  async function ask(work: () => Promise<UpdateStatus>) {
    setAsking(true);
    setError(null);
    try {
      onChange(await work());
      setConfirming(null);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setAsking(false);
    }
  }

  let body: ReactNode;
  if (!updater?.running) {
    if (!owner) return null;
    body = (
      <p className={styles.quiet}>
        The owner can update from here once the updater runs beside the server: add
        {' '}<span className={styles.mono}>COMPOSE_PROFILES=updater</span> to .env and run
        {' '}<span className={styles.mono}>{COMPOSE} up -d</span> on a Linux host. It's the one container with the
        Docker socket; see “Update button” in ops/web/README.md first.
      </p>
    );
  } else if (!updater.ready) {
    body = <p className={styles.error}>{updater.problem ?? "The updater can't update this server."}</p>;
  } else {
    body = (
      <>
        {busy && <JobProgress job={job} lost={lost} />}
        {!busy && job && <JobResult job={job} current={status.currentVersion} now={now} />}
        {owner && !busy && confirming === null && (
          <div className={styles.toolbar}>
            {status.updateAvailable && target && (
              <Button variant="decision" icon={<Download size={16} />} onClick={() => setConfirming('update')}>
                Update to {target}
              </Button>
            )}
            {updater.previous && (
              <Button size="sm" variant="quiet" icon={<RotateCcw size={14} />} onClick={() => setConfirming('rollback')}>
                Go back to {updater.previous}
              </Button>
            )}
          </div>
        )}
        {owner && confirming && (
          <div className={styles.confirm} role="group" aria-label={confirming === 'update' ? 'Confirm the update' : 'Confirm going back'}>
            <h3>{confirming === 'update' ? `Update to ${target}?` : `Go back to ${updater.previous}?`}</h3>
            <p>
              {confirming === 'update'
                ? 'The databases are backed up first. Players are warned, games still running get up to 15 minutes to end, then the server restarts on the new version (about a minute). If it doesn\'t come up healthy, the server goes back to this version by itself.'
                : 'The databases are backed up, then the server restarts on the version that ran before the last update. Games still running end.'}
            </p>
            {error && <p className={styles.error}>{error}</p>}
            <div className={styles.toolbar}>
              <Button variant="decision" busy={asking}
                onClick={() => void ask(() => (confirming === 'update' ? api.adminUpdateApply(target!) : api.adminUpdateRollback()))}>
                {confirming === 'update' ? 'Update now' : 'Go back'}
              </Button>
              <Button size="sm" variant="quiet" onClick={() => { setConfirming(null); setError(null); }}>Cancel</Button>
            </div>
          </div>
        )}
        {(updater.log?.length ?? 0) > 0 && (
          <details className={styles.updateLog}>
            <summary>Updater log</summary>
            <pre>{updater.log!.join('\n')}</pre>
          </details>
        )}
      </>
    );
  }

  return (
    <section aria-labelledby="update-button" className={styles.block}>
      <h2 id="update-button">Update button</h2>
      {body}
    </section>
  );
}

function JobProgress({ job: last, lost }: { job?: UpdateJob; lost: boolean }) {
  // asked, but the updater hasn't taken it yet: the status file still holds the job before
  const job = last && (last.state === 'queued' || last.state === 'running') ? last : undefined;
  const at = STEPS.findIndex((step) => step.id === job?.step);
  return (
    <div className={styles.maintenance} aria-live="polite">
      <p><strong>{!job ? 'Waiting for the updater to start…' : job.action === 'rollback' ? 'Going back' : `Updating to ${job.tag}`}</strong></p>
      <ol className={styles.steps}>
        {STEPS.map((step, index) => (
          <li key={step.id} className={index < at ? styles.stepDone : index === at ? styles.stepNow : undefined}>{step.label}</li>
        ))}
      </ol>
      {job?.message && <p className={styles.quiet}>{job.message}</p>}
      {lost && (
        <p className={styles.quiet}>
          The server is restarting, so this page lost it for now. Sign in again in a minute or two: this section says how the update went.
        </p>
      )}
    </div>
  );
}

function JobResult({ job, current, now }: { job: UpdateJob; current?: string; now: number }) {
  const ok = job.state === 'ok';
  // the browser still runs the web client it loaded before the update
  const stale = ok && job.action === 'update' && !!current && current === job.tag && APP_VERSION !== current && APP_VERSION !== 'dev';
  return (
    <div className={styles.block}>
      <p className={ok ? undefined : styles.error}>
        {RESULTS[job.state ?? ''] ?? job.state}: {job.message}
        {job.finishedAt ? <span className={styles.quiet}> · {describeAge(job.finishedAt * 1000, now)}{job.by ? `, asked by ${job.by}` : ''}</span> : null}
      </p>
      {stale && (
        <p>
          This page is still the old version.{' '}
          <button type="button" className={styles.linkButton} onClick={() => window.location.reload()}>Reload it</button>
        </p>
      )}
    </div>
  );
}

