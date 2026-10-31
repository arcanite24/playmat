import { ArrowLeft, ChevronDown, KeyRound } from 'lucide-react';
import { lazy, Suspense, useEffect, useRef, useState, type CSSProperties, type FormEvent } from 'react';
import { Link, useLocation, useNavigate } from 'react-router-dom';
import { APP_NAME } from '../brand';
import { api, rpc } from '../connection';
import { useT, type MessageKey } from '../i18n';
import { RichText } from '../i18n/RichText';
import type { InviteCheck, ServerInfo } from '../../protocol/generated/views';
import type { StarterDeck } from '../stores/decks';
import { useSession } from '../stores/session';
import { Button } from '../ui/Button';
import { CardFace } from '../ui/CardFace';
import { Field } from '../ui/Field';
import { Mark } from '../ui/Mark';
import { MatPrint } from '../ui/MatPrint';
import { checkPassword } from './accountRules';
import styles from './LoginScreen.module.css';

// Cloudflare's check loads only on servers that turn it on
const TurnstileWidget = lazy(() => import('./TurnstileWidget').then((module) => ({ default: module.TurnstileWidget })));
// the month's server costs, on servers that take support
const SupportBar = lazy(() => import('../support/SupportBar').then((module) => ({ default: module.SupportBar })));

type ServerStatus = 'checking' | 'online' | 'offline';
type Mode = 'sit' | 'register' | 'forgot' | 'reset' | 'ask' | 'githubName';

const MODE_TITLES: Record<Mode | 'signIn', MessageKey> = {
  sit: 'login.title.sit',
  signIn: 'login.title.signIn',
  register: 'login.title.register',
  forgot: 'login.title.reset',
  reset: 'login.title.reset',
  ask: 'login.title.ask',
  githubName: 'login.title.github',
};

const PASSWORD_PROBLEMS: Record<NonNullable<ReturnType<typeof checkPassword>>, MessageKey> = {
  short: 'login.password.short',
  mix: 'login.password.mix',
  name: 'login.password.name',
};

const DEAL_MS = 700;
const ART_EVERY_MS = 9000;

/**
 * Sitting down at the table: the mat sews its edge in, the name is printed in register, and the player takes a seat
 * with nothing more than a name. Signing in deals the deck across the table on the way to Play.
 */
export function LoginScreen() {
  const session = useSession();
  const t = useT();
  const navigate = useNavigate();
  const location = useLocation();
  const from = (location.state as { from?: string } | null)?.from;
  // an invite link (/join?invite=...): registration that lets the account in to a contributors-only server
  const query = new URLSearchParams(location.search);
  const invite = query.get('invite') ?? '';
  // back from GitHub (/auth/github?code=...&state=...)
  const backFromGithub = location.pathname === '/auth/github';
  const github = backFromGithub ? { code: query.get('code') ?? '', state: query.get('state') ?? '' } : null;
  // game links go stale across sign-ins; anything else is worth returning to, unless the server restarted under it
  const [restarted] = useState(session.expired);
  const destination = !restarted && from && !from.startsWith('/game/') && from !== '/login' ? from : '/';
  const [userName, setUserName] = useState(session.userName);
  const [password, setPassword] = useState('');
  const [remember, setRemember] = useState(true);
  const [needsPassword, setNeedsPassword] = useState(false);
  const [serverUrl, setServerUrl] = useState(session.serverUrl);
  const [editServer, setEditServer] = useState(false);
  const [dealing, setDealing] = useState(false);
  const [refused, setRefused] = useState(0);
  const [mode, setMode] = useState<Mode>(invite ? 'register' : 'sit');
  const [note, setNote] = useState('');
  const [email, setEmail] = useState('');
  const [code, setCode] = useState('');
  // a notice another screen left for this one (an account just deleted)
  const [notice, setNotice] = useState<string | null>((location.state as { notice?: string } | null)?.notice ?? null);
  const [agreed, setAgreed] = useState(false);
  const [working, setWorking] = useState(false);
  // Cloudflare Turnstile's token, when the server asks for one on registering and asking for access
  const [turnstileToken, setTurnstileToken] = useState('');
  const [turnstileBroken, setTurnstileBroken] = useState(false);
  const { status, info } = useServerStatus(serverUrl);
  const inviteCheck = useInviteCheck(invite, status === 'online');
  const art = useRotatingArt();
  const resumed = useRef(false);
  // another sign-in took the session over: this tab waits for the player instead of taking it back by itself
  const [takingBack, setTakingBack] = useState(false);
  const elsewhere = (session.signedInElsewhere || takingBack) && !invite && !backFromGithub;

  useEffect(() => {
    if (resumed.current || invite || backFromGithub) return;
    resumed.current = true;
    void session.resume().then((ok) => ok && navigate(destination, { replace: true }));
  }, [session, navigate, destination, invite, backFromGithub]);

  const accounts = !!info?.accounts;
  const contributors = info?.access === 'allowlist';
  const inviteOnly = info?.registration === 'invite';
  const maintenanceAt = info?.maintenanceAt ?? 0;
  const busy = session.phase === 'signingIn' || dealing;
  const name = userName.trim();
  const nameError = name && !/^[A-Za-z0-9_]{3,14}$/.test(name) ? t('login.nameRule') : null;
  const turnstileKey = (mode === 'register' || mode === 'ask') && info?.turnstileSiteKey ? info.turnstileSiteKey : null;
  const checked = !turnstileKey || !!turnstileToken;
  const minPassword = info?.minPasswordLength || 8;
  const passwordRule = t('login.passwordRule', { min: minPassword });
  const passwordProblem = password && (mode === 'register' || mode === 'reset') ? checkPassword(password, minPassword, name) : null;
  const passwordError = passwordProblem ? t(PASSWORD_PROBLEMS[passwordProblem], { min: minPassword }) : null;
  const canSubmit = mode === 'sit' ? !!name && !nameError && (!accounts || !!password)
    : mode === 'register' ? !!name && !nameError && !!email.trim() && !!password && !passwordError && checked && agreed
      : mode === 'forgot' ? !!email.trim()
        : mode === 'ask' ? !!name && !nameError && !!email.trim() && checked
          : mode === 'githubName' ? !!name && !nameError
          : code.length === 6 && !!password && !passwordError;
  const submitLabel = t(mode === 'register' ? 'login.submit.register' : mode === 'forgot' ? 'login.submit.forgot' : mode === 'reset' ? 'login.submit.reset'
    : mode === 'ask' ? 'login.submit.ask' : mode === 'githubName' ? 'login.submit.github'
    : busy ? 'login.submit.busy' : accounts ? 'login.submit.signIn' : 'login.submit.sit');

  function switchMode(next: Mode) {
    setMode(next);
    setNotice(null);
    setPassword('');
    setTurnstileToken('');
    useSession.setState({ error: null });
  }

  /** The form shakes and starts over; a Turnstile token is used up by the try, so the check runs again. */
  function refuse() {
    setRefused((count) => count + 1);
    setTurnstileToken('');
  }

  // back from GitHub: sign in to the account it names, or pick a name for a new one first
  const githubDone = useRef(false);
  useEffect(() => {
    if (!github || status !== 'online' || githubDone.current) return;
    githubDone.current = true;
    void finishGithub(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- once, when the server answers
  }, [status]);

  async function finishGithub(chosen: string | null) {
    if (!github) return;
    setWorking(true);
    try {
      const result = await api.githubFinish(github.state, github.code, chosen);
      if (result.userName && result.rememberToken) {
        if (await session.signInWithToken(serverUrl.trim(), result.userName, result.rememberToken)) seated();
        return;
      }
      setMode('githubName');
      if (result.suggestedName && !chosen) setUserName(result.suggestedName);
      useSession.setState({ error: result.nameProblem ?? (chosen ? null : t('login.github.nameTaken')) });
    } catch (error) {
      useSession.setState({ error: error instanceof Error ? error.message : String(error) });
      setMode('sit');
    } finally {
      setWorking(false);
    }
  }

  async function startGithub() {
    setWorking(true);
    try {
      window.location.assign(await api.githubStart());
    } catch (error) {
      useSession.setState({ error: error instanceof Error ? error.message : String(error) });
      setWorking(false);
    }
  }

  async function takeBack() {
    setTakingBack(true);
    if (await session.takeBack()) {
      seated();
      return;
    }
    // what this tab had no longer works: the form, saying why
    setTakingBack(false);
    refuse();
  }

  function seated() {
    // the deck is dealt across the table, then Play takes over
    setDealing(true);
    setTimeout(() => navigate(destination, { replace: true }), DEAL_MS);
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!canSubmit) return;
    const url = serverUrl.trim();
    if (mode === 'sit') {
      if (await session.signIn(url, name, password, remember)) seated();
      else refuse();
      return;
    }
    setWorking(true);
    try {
      if (mode === 'register') {
        if (await session.register(url, name, email.trim(), password, remember, invite, turnstileToken)) seated();
        else refuse();
      } else if (mode === 'githubName' && github) {
        await finishGithub(name);
      } else if (mode === 'ask') {
        try {
          if (rpc.getStatus() !== 'open' || rpc.getUrl() !== url) await rpc.connect(url);
          await api.accessRequest(name, email.trim(), note.trim() || null, turnstileToken || null);
          switchMode('sit');
          setNotice(t('login.asked', { email: email.trim() }));
        } catch (error) {
          useSession.setState({ error: error instanceof Error ? error.message : String(error) });
          refuse();
        }
      } else if (mode === 'forgot') {
        if (await session.requestPasswordReset(url, email.trim())) {
          setMode('reset');
          setCode('');
          setNotice(t('login.codeSent', { email: email.trim() }));
        }
      } else if (await session.resetPassword(url, email.trim(), code, password)) {
        switchMode('sit');
        setNotice(t('login.passwordChanged'));
      } else {
        refuse();
      }
    } finally {
      setWorking(false);
    }
  }

  return (
    <div className={[styles.mat, dealing ? styles.leaving : ''].join(' ')}>
      <div className={styles.art} aria-hidden="true">
        <MatPrint key={art ? `${art.setCode}:${art.cardNumber}` : 'none'} card={art} />
      </div>
      <SewnEdge />

      <header className={styles.brand}>
        <span className={styles.mark}><Mark size={112} /></span>
        <h1 className={styles.wordmark} aria-label={APP_NAME}>
          {/* three separations of a screen print sliding into register */}
          <span className={styles.passShadow} aria-hidden="true">{APP_NAME}</span>
          <span className={styles.passLight} aria-hidden="true">{APP_NAME}</span>
          <span className={styles.passKey} aria-hidden="true">{APP_NAME}</span>
        </h1>
      </header>

      {elsewhere && (
        <section className={styles.seat} aria-labelledby="elsewhere-title">
          <h2 id="elsewhere-title" className={styles.seatLabel}>{t('login.elsewhere.title')}</h2>
          <p className={styles.notice} role="status">{t('login.elsewhere.text', { name: session.userName })}</p>
          <Button type="button" variant="decision" size="xl" busy={busy || takingBack} className={styles.submit} onClick={() => void takeBack()}>
            {t('login.elsewhere.useHere', { app: APP_NAME })}
          </Button>
          <p className={styles.quiet}>{t('login.elsewhere.hint')}</p>
          <button type="button" className={styles.linkButton} onClick={() => useSession.setState({ signedInElsewhere: false })}>
            {t('login.elsewhere.other')}
          </button>
        </section>
      )}

      <form key={refused} hidden={elsewhere} className={[styles.seat, refused > 0 ? styles.refused : ''].join(' ')} onSubmit={submit} aria-labelledby="seat-title">
        <h2 id="seat-title" className={styles.seatLabel}>{t(MODE_TITLES[mode === 'sit' && accounts ? 'signIn' : mode])}</h2>
        {info?.notice && <p className={styles.serverNotice}>{info.notice}</p>}
        {contributors && mode === 'sit' && <p className={styles.notice}>{t('login.contributors')}</p>}
        {inviteOnly && !invite && mode === 'register' && <p className={styles.notice}>{t('login.inviteOnly')}</p>}
        {info?.ownerPending && (mode === 'sit' || mode === 'register') && <p className={styles.notice}>{t('login.ownerPending')}</p>}
        {maintenanceAt > 0 && <p className={styles.notice} role="status">{t('shell.maintenanceHint')}{info?.maintenanceMessage ? ` ${info.maintenanceMessage}` : ''}</p>}
        {invite && mode === 'register' && (
          <p className={inviteCheck?.valid === false ? styles.error : styles.notice} role="status">
            {inviteCheck === null ? t('login.invite.checking') : inviteCheck.valid ? t('login.invite.valid') : inviteCheck.reason}
          </p>
        )}

        {(mode === 'sit' || mode === 'register' || mode === 'ask' || mode === 'githubName') && (
          <>
            <SeatPlate name={name && !nameError ? name : ''} placeholder={t('login.yourName')} />
            <Field
              label={accounts ? t('login.accountName') : t('login.playerName')}
              value={userName}
              autoComplete="username"
              autoFocus
              maxLength={14}
              onChange={(event) => setUserName(event.target.value)}
              error={nameError}
              required
            />
          </>
        )}

        {(mode === 'register' || mode === 'forgot' || mode === 'reset' || mode === 'ask') && (
          <Field
            label={t('login.email')}
            type="email"
            value={email}
            autoComplete="email"
            autoFocus={mode === 'forgot'}
            onChange={(event) => setEmail(event.target.value)}
            hint={mode === 'register' ? t('login.emailHint') : mode === 'ask' ? t('login.askEmailHint') : undefined}
            required
            readOnly={mode === 'reset'}
          />
        )}

        {mode === 'ask' && (
          <Field
            label={t('login.askNote')}
            value={note}
            maxLength={500}
            onChange={(event) => setNote(event.target.value)}
            hint={t('login.askNoteHint')}
          />
        )}

        {mode === 'reset' && (
          <Field
            label={t('login.code')}
            value={code}
            inputMode="numeric"
            autoComplete="one-time-code"
            autoFocus
            maxLength={6}
            onChange={(event) => setCode(event.target.value.replace(/\D/g, ''))}
            hint={t('login.codeHint')}
            required
          />
        )}

        {mode === 'sit' && (accounts || needsPassword) && (
          <Field
            label={t('login.password')}
            type="password"
            value={password}
            autoComplete="current-password"
            autoFocus={needsPassword && !accounts}
            onChange={(event) => setPassword(event.target.value)}
            required={accounts}
          />
        )}
        {(mode === 'register' || mode === 'reset') && (
          <Field
            label={mode === 'reset' ? t('login.newPassword') : t('login.password')}
            type="password"
            value={password}
            autoComplete="new-password"
            onChange={(event) => setPassword(event.target.value)}
            hint={passwordRule}
            error={passwordError}
            required
          />
        )}

        {((mode === 'sit' && accounts) || mode === 'register') && (
          <label className={styles.remember}>
            <input type="checkbox" checked={remember} onChange={(event) => setRemember(event.target.checked)} />
            {t('login.remember')}
          </label>
        )}

        {mode === 'register' && (
          <label className={styles.remember}>
            <input type="checkbox" checked={agreed} required onChange={(event) => setAgreed(event.target.checked)} />
            <span>
              <RichText
                text={t('login.agree')}
                parts={{
                  terms: <Link to="/terms" target="_blank">{t('login.agree.terms')}</Link>,
                  privacy: <Link to="/privacy" target="_blank">{t('login.agree.privacy')}</Link>,
                }}
              />
            </span>
          </label>
        )}

        {mode === 'sit' && !accounts && !needsPassword && (
          <button type="button" className={styles.linkButton} onClick={() => setNeedsPassword(true)}>
            <KeyRound size={15} aria-hidden="true" /> {t('login.needsPassword')}
          </button>
        )}

        {mode === 'sit' && (
          <div className={styles.server}>
            <button type="button" className={styles.serverChip} aria-expanded={editServer} onClick={() => setEditServer((value) => !value)}>
              <span className={[styles.dot, styles[status]].join(' ')} aria-hidden="true" />
              <span>{describeServer(serverUrl) ?? t('login.server.choose')}</span>
              <span className={styles.statusText}>{t(status === 'online' ? 'login.server.online' : status === 'offline' ? (navigator.onLine ? 'login.server.offline' : 'login.server.noNetwork') : 'login.server.checking')}</span>
              <ChevronDown size={15} aria-hidden="true" className={editServer ? styles.flipped : ''} />
            </button>
            {editServer && (
              <Field
                label={t('login.server.address')}
                value={serverUrl}
                onChange={(event) => setServerUrl(event.target.value)}
                hint={t('login.server.hint')}
                spellCheck={false}
              />
            )}
          </div>
        )}

        {turnstileKey && (
          <div className={styles.turnstile}>
            <Suspense fallback={null}>
              <TurnstileWidget siteKey={turnstileKey} onToken={setTurnstileToken} onUnavailable={() => setTurnstileBroken(true)} />
            </Suspense>
            {turnstileBroken && <p className={styles.error} role="alert">{t('login.turnstile.unavailable')}</p>}
          </div>
        )}

        {notice && <p className={styles.notice} role="status">{notice}</p>}
        {session.expired && !session.error && <p className={styles.error} role="alert">{t('login.serverRestarted')}</p>}
        {session.staleLogin && !session.error && <p className={styles.notice} role="status">{t('login.staleLogin')}</p>}
        {session.error && <p className={styles.error} role="alert">{session.error}</p>}

        <Button type="submit" variant="decision" size="xl" busy={busy || working} disabled={!canSubmit} className={styles.submit}>
          {submitLabel}
        </Button>

        {mode === 'sit' && accounts && info?.github && (
          <Button type="button" variant="quiet" icon={<GithubMark />} onClick={startGithub} busy={working}>
            {t('login.github')}
          </Button>
        )}

        {mode === 'sit' && accounts && (
          <div className={styles.accountLinks}>
            <button type="button" className={styles.linkButton} onClick={() => switchMode('register')}>{t('login.createAccount')}</button>
            {(contributors || inviteOnly) && <button type="button" className={styles.linkButton} onClick={() => switchMode('ask')}>{t('login.ask')}</button>}
            {info?.mail
              ? <button type="button" className={styles.linkButton} onClick={() => switchMode('forgot')}>{t('login.forgot')}</button>
              : <span className={styles.quiet}>{t('login.forgotAskAdmin')}</span>}
          </div>
        )}
        {mode !== 'sit' && (
          <button type="button" className={styles.linkButton} onClick={() => switchMode('sit')}>
            <ArrowLeft size={15} aria-hidden="true" /> {t('login.back')}
          </button>
        )}
        <nav className={styles.legal} aria-label={t('shell.about')}>
          <Link to="/about" className={styles.about}>{t('shell.about')}</Link>
          <Link to="/privacy" className={styles.about}>{t('legal.privacyLink')}</Link>
          <Link to="/terms" className={styles.about}>{t('legal.termsLink')}</Link>
        </nav>
        {mode === 'sit' && status === 'online' && info?.support && <Suspense fallback={null}><SupportBar place="login" /></Suspense>}
      </form>

      <Deck dealing={dealing} />
    </div>
  );
}

/** GitHub's mark (Octicons, MIT), for Sign in with GitHub. */
function GithubMark() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z" />
    </svg>
  );
}

/** How you'll appear at the table, printed as you type. */
function SeatPlate({ name, placeholder }: { name: string; placeholder: string }) {
  return (
    <div className={[styles.plate, name ? styles.plateOn : ''].join(' ')} aria-hidden="true">
      <span key={name.slice(0, 1)} className={styles.avatar}>{name ? name.slice(0, 1).toUpperCase() : '?'}</span>
      <span className={styles.plateText}>
        <span className={styles.plateName}>{name || placeholder}</span>
        <span className={styles.plateLife}>20</span>
      </span>
    </div>
  );
}

/** The stitched edge, sewn around the mat when the page opens. */
function SewnEdge() {
  return (
    <svg className={styles.edge} aria-hidden="true" preserveAspectRatio="none">
      <rect className={styles.groove} x="1.5" y="1.5" width="calc(100% - 3px)" height="calc(100% - 3px)" rx="18" ry="18" />
      <rect className={styles.thread} x="1.5" y="1.5" width="calc(100% - 3px)" height="calc(100% - 3px)" rx="18" ry="18" />
    </svg>
  );
}

const DECK_SIZE = 7;

/** A sleeved deck set down by the seat; it deals itself across the table on sign-in. */
function Deck({ dealing }: { dealing: boolean }) {
  return (
    <div className={[styles.deck, dealing ? styles.dealing : ''].join(' ')} aria-hidden="true">
      {Array.from({ length: DECK_SIZE }, (_, index) => (
        <div
          key={index}
          className={styles.deckCard}
          style={{
            ['--i' as string]: index,
            ['--angle' as string]: `${(index - (DECK_SIZE - 1) / 2) * 14}deg`,
          } as CSSProperties}
        >
          <CardFace card={{}} hidden sleeve="#3b1f2b" size="small" />
        </div>
      ))}
    </div>
  );
}

/** The server's host, or null when there is no address yet. */
function describeServer(url: string): string | null {
  try {
    return new URL(url).host;
  } catch {
    return url || null;
  }
}

/**
 * Whether the server answers, and whether it has accounts; checked on load and when the address changes (the connection
 * is kept for sign-in).
 */
function useServerStatus(url: string): { status: ServerStatus; info: ServerInfo | null } {
  const [result, setResult] = useState<{ url: string; status: ServerStatus; info: ServerInfo | null } | null>(null);
  // the installed app opens offline too (its shell is cached): check again when the network comes back
  const [backOnline, setBackOnline] = useState(0);
  useEffect(() => {
    const recheck = () => setBackOnline((count) => count + 1);
    window.addEventListener('online', recheck);
    return () => window.removeEventListener('online', recheck);
  }, []);
  useEffect(() => {
    let cancelled = false;
    const timer = setTimeout(async () => {
      try {
        // a sign-in (or a resumed session) may already be opening this connection: wait for it instead
        for (let waited = 0; rpc.getStatus() === 'connecting' && rpc.getUrl() === url && waited < 5000; waited += 150) {
          await new Promise((resolve) => setTimeout(resolve, 150));
        }
        if (rpc.getStatus() !== 'open' || rpc.getUrl() !== url) await rpc.connect(url);
        await api.getServerState();
        // older servers don't know serverInfo: they have no accounts as far as the web client is concerned
        const info = await api.serverInfo().catch(() => null);
        if (!cancelled) setResult({ url, status: 'online', info });
      } catch {
        if (!cancelled) setResult({ url, status: 'offline', info: null });
      }
    }, 400);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [url, backOnline]);
  return result?.url === url ? { status: result.status, info: result.info } : { status: 'checking', info: null };
}

/** Whether the invite link still works: null while it is being checked (or the server isn't reached yet). */
function useInviteCheck(code: string, online: boolean): InviteCheck | null {
  const [result, setResult] = useState<InviteCheck | null>(null);
  useEffect(() => {
    if (!code || !online) return;
    let cancelled = false;
    api.inviteCheck(code)
      .then((check) => !cancelled && setResult(check))
      // an older server without invites: let registration say what is wrong
      .catch(() => !cancelled && setResult({ valid: true }));
    return () => {
      cancelled = true;
    };
  }, [code, online]);
  return result;
}

/** Art printed into the mat, changing every few seconds: the starter decks' covers. */
function useRotatingArt() {
  const [covers, setCovers] = useState<StarterDeck['cover'][]>([]);
  const [index, setIndex] = useState(0);
  useEffect(() => {
    let cancelled = false;
    fetch('/starter-decks/index.json')
      .then((response) => (response.ok ? response.json() : []))
      .then((starters: StarterDeck[]) => {
        if (!cancelled) setCovers(starters.map((starter) => starter.cover).sort(() => Math.random() - 0.5));
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);
  useEffect(() => {
    if (covers.length < 2 || window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) return;
    const timer = setInterval(() => setIndex((value) => value + 1), ART_EVERY_MS);
    return () => clearInterval(timer);
  }, [covers.length]);
  return covers.length ? covers[index % covers.length] : null;
}
