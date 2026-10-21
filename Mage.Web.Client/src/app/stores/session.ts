import { create } from 'zustand';
import { api, DEFAULT_SERVER_URL, rpc } from '../connection';
import { ConnectionError, RpcError, type ConnectionStatus } from '../../core/rpc/RpcClient';
import { forgetOpenGames } from './openGames';
import { readJson, writeJson } from './persist';

export type SessionPhase = 'signedOut' | 'signingIn' | 'signedIn';

interface RememberedLogin {
  serverUrl: string;
  userName: string;
  /** signed in without a password last time, so a reload can sign in again silently */
  passwordless?: boolean;
  /** signed in to an account: a reload signs back in with the restore token while the server still holds the user */
  account?: boolean;
}

const LOGIN_KEY = 'playmat.login';
const RESTORE_KEY = 'playmat.restore';
const REMEMBER_KEY = 'playmat.remember';

/** "Keep me signed in": the server's token for this device, which signs the account in without its password. */
interface RememberedDevice {
  serverUrl: string;
  userName: string;
  token: string;
}

/** Server-issued token that hands our user (and its tables) back to a later connection. */
interface RememberedRestore {
  serverUrl: string;
  userName: string;
  token: string;
}

export interface SessionState {
  phase: SessionPhase;
  connection: ConnectionStatus;
  serverUrl: string;
  userName: string;
  roomId: string | null;
  /** counts logins, including the silent one after a dropped connection: the server has just handed our games back */
  loginCount: number;
  error: string | null;
  /** the server restarted (or forgot this session) while we were signed in: the sign-in screen says so */
  expired: boolean;
  /**
   * another sign-in as this player (another tab or device) took the session over. Nothing signs back in by itself
   * then, or the two would keep taking it from each other: the sign-in screen offers takeBack
   */
  signedInElsewhere: boolean;
  /** this device's saved sign-in stopped working (the password changed, or it signed out elsewhere): sign in again */
  staleLogin: boolean;
  /** with remember, an account stays signed in on this device ("keep me signed in") until it signs out */
  signIn(serverUrl: string, userName: string, password: string, remember?: boolean): Promise<boolean>;
  /** creates an account with the chosen password and signs in with it */
  /** inviteCode: an invite link's code, which lets the account in to a contributors-only server */
  /** turnstileToken: Cloudflare Turnstile's token, for servers that ask for one */
  register(serverUrl: string, userName: string, email: string, password: string, remember?: boolean, inviteCode?: string, turnstileToken?: string): Promise<boolean>;
  /** signs in with a remembered-device token the server gave this device (after Sign in with GitHub) */
  signInWithToken(serverUrl: string, userName: string, token: string): Promise<boolean>;
  /** emails a reset code to the account with this address (the server doesn't say whether there is one) */
  requestPasswordReset(serverUrl: string, email: string): Promise<boolean>;
  resetPassword(serverUrl: string, email: string, code: string, password: string): Promise<boolean>;
  signOut(keepGames?: boolean): Promise<void>;
  /** after a reload: sign the remembered player back in when the server needs no password or the device is remembered */
  resume(): Promise<boolean>;
  /** signed in somewhere else: take the session back to this device, with what it already has (no password asked) */
  takeBack(): Promise<boolean>;
}

/** The restore token the server gave this player on this server, '' when there is none. */
function restoreToken(serverUrl: string, userName: string): string {
  const saved = readJson<RememberedRestore | null>(RESTORE_KEY, null);
  return saved && saved.serverUrl === serverUrl && saved.userName === userName ? saved.token : '';
}

/** The token changes with every login, so ask for the new one each time. */
async function rememberRestoreToken(serverUrl: string, userName: string): Promise<void> {
  try {
    const token = await api.sessionGetRestoreToken();
    writeJson<RememberedRestore | null>(RESTORE_KEY, token ? { serverUrl, userName, token } : null);
  } catch {
    // older server without restore tokens: reconnects fall back to the server's same-address rule
  }
}

/** This device's "keep me signed in" token for the player on this server, '' when there is none. */
function rememberToken(serverUrl: string, userName: string): string {
  const saved = readJson<RememberedDevice | null>(REMEMBER_KEY, null);
  return saved && saved.serverUrl === serverUrl && saved.userName === userName ? saved.token : '';
}

async function rememberDevice(serverUrl: string, userName: string): Promise<void> {
  try {
    const token = await api.authRememberLogin();
    writeJson<RememberedDevice | null>(REMEMBER_KEY, token ? { serverUrl, userName, token } : null);
  } catch {
    // a server without accounts (or an older one) has nothing to remember
  }
}

/** Signs this device out for good: the server stops accepting its token. */
async function forgetDevice(serverUrl: string, userName: string): Promise<void> {
  const token = rememberToken(serverUrl, userName);
  writeJson<RememberedDevice | null>(REMEMBER_KEY, null);
  try {
    if (token) await api.authForgetLogin(userName, token);
  } catch {
    // unreachable server: the token runs out unused
  }
}

/** Logs in; a remembered device's token stands in for an empty password. */
function connectUser(userName: string, password: string, restore: string, device: string): Promise<boolean> {
  return device ? api.connectUser(userName, password, restore, null, null, device) : api.connectUser(userName, password, restore);
}

const REFUSED = 'The server refused the login. Check your name and password.';

function describeError(error: unknown): string {
  if (error instanceof ConnectionError) {
    return globalThis.navigator?.onLine !== false
      ? `Can't reach the server. Check the address and that it is running.`
      : `You're offline. Connect to the internet to sit down at the table.`;
  }
  if (error instanceof RpcError) return error.message;
  return error instanceof Error ? error.message : String(error);
}

const remembered = readJson<RememberedLogin | null>(LOGIN_KEY, null);

async function connectTo(serverUrl: string): Promise<void> {
  if (rpc.getStatus() !== 'open' || rpc.getUrl() !== serverUrl) {
    await rpc.connect(serverUrl);
  }
}

// kept in memory only, to log back in after a dropped connection
let sessionPassword = '';

export const useSession = create<SessionState>((set, get) => ({
  phase: 'signedOut',
  connection: rpc.getStatus(),
  serverUrl: remembered?.serverUrl ?? DEFAULT_SERVER_URL,
  userName: remembered?.userName ?? '',
  roomId: null,
  loginCount: 0,
  error: null,
  expired: false,
  signedInElsewhere: false,
  staleLogin: false,

  async signIn(serverUrl, userName, password, remember = false) {
    set({ phase: 'signingIn', error: null, expired: false, signedInElsewhere: false, staleLogin: false, serverUrl, userName });
    try {
      await connectTo(serverUrl);
      const device = password ? '' : rememberToken(serverUrl, userName);
      const ok = await connectUser(userName, password, restoreToken(serverUrl, userName), device);
      if (!ok) {
        // a remembered device the server turned away (signed out elsewhere, password changed): ask for the password
        if (device) writeJson<RememberedDevice | null>(REMEMBER_KEY, null);
        set(device ? { phase: 'signedOut', error: null, staleLogin: true } : { phase: 'signedOut', error: REFUSED });
        return false;
      }
      sessionPassword = password;
      await rememberRestoreToken(serverUrl, userName);
      // typing the password decides it again: remembered from now on, or this device forgotten
      if (password && remember) await rememberDevice(serverUrl, userName);
      else if (password) await forgetDevice(serverUrl, userName);
      const roomId = await api.serverGetMainRoomId();
      // an account signed back in with the restore token stays an account
      const before = readJson<RememberedLogin | null>(LOGIN_KEY, null);
      const account = !!password || (!!before?.account && before.serverUrl === serverUrl && before.userName === userName);
      writeJson<RememberedLogin>(LOGIN_KEY, { serverUrl, userName, passwordless: !password && !account, account });
      set((state) => ({ phase: 'signedIn', roomId, loginCount: state.loginCount + 1 }));
      return true;
    } catch (error) {
      set({ phase: 'signedOut', error: describeError(error) });
      return false;
    }
  },

  async resume() {
    const login = readJson<RememberedLogin | null>(LOGIN_KEY, null);
    // signed in somewhere else: signing back in here would sign the other one out, which would do the same
    if (!login || get().phase !== 'signedOut' || get().signedInElsewhere) return false;
    // an account needs its password again unless this device is remembered or the server still holds the user for
    // our restore token
    const known = rememberToken(login.serverUrl, login.userName) || restoreToken(login.serverUrl, login.userName);
    if (!login.passwordless && !(login.account && known)) return false;
    const ok = await get().signIn(login.serverUrl, login.userName, '');
    if (!ok) set({ error: null });
    return ok;
  },

  async takeBack() {
    const { serverUrl, userName } = get();
    // the remembered device or the restore token when there is one, else the password typed in this tab
    const password = rememberToken(serverUrl, userName) ? '' : sessionPassword;
    if (await get().signIn(serverUrl, userName, password)) return true;
    // what this tab had no longer works (the password changed since): the sign-in form, saying why
    if (get().error === REFUSED || get().staleLogin) set({ error: null, staleLogin: true });
    return false;
  },

  async signInWithToken(serverUrl, userName, token) {
    writeJson<RememberedDevice | null>(REMEMBER_KEY, { serverUrl, userName, token });
    // an account: a reload signs back in with the token, never as a passwordless guest
    writeJson<RememberedLogin>(LOGIN_KEY, { serverUrl, userName, passwordless: false, account: true });
    return get().signIn(serverUrl, userName, '');
  },

  async register(serverUrl, userName, email, password, remember = false, inviteCode, turnstileToken) {
    set({ phase: 'signingIn', error: null, serverUrl, userName });
    try {
      await connectTo(serverUrl);
      await api.authRegister(userName, password, email, inviteCode || null, turnstileToken || null);
    } catch (error) {
      set({ phase: 'signedOut', error: describeError(error) });
      return false;
    }
    set({ phase: 'signedOut' });
    return get().signIn(serverUrl, userName, password, remember);
  },

  async requestPasswordReset(serverUrl, email) {
    set({ error: null, serverUrl });
    try {
      await connectTo(serverUrl);
      await api.authSendTokenToEmail(email);
      return true;
    } catch (error) {
      set({ error: describeError(error) });
      return false;
    }
  },

  async resetPassword(serverUrl, email, code, password) {
    set({ error: null, serverUrl });
    try {
      await connectTo(serverUrl);
      await api.authResetPassword(email, code, password);
      return true;
    } catch (error) {
      set({ error: describeError(error) });
      return false;
    }
  },

  async signOut(keepGames = false) {
    try {
      if (get().phase === 'signedIn') await api.disconnectSession(keepGames);
    } catch {
      // leaving anyway
    }
    sessionPassword = '';
    // signing out means this device asks for the password next time
    await forgetDevice(get().serverUrl, get().userName);
    // leaving for good gives the tables up, so the token (and the games to rejoin) are of no use any more
    if (!keepGames) {
      writeJson<RememberedRestore | null>(RESTORE_KEY, null);
      forgetOpenGames(get().serverUrl, get().userName);
    }
    writeJson<RememberedLogin>(LOGIN_KEY, { serverUrl: get().serverUrl, userName: get().userName, passwordless: false, account: false });
    rpc.disconnect();
    set({ phase: 'signedOut', roomId: null, signedInElsewhere: false, staleLogin: false });
  },
}));

rpc.onStatus((connection) => {
  useSession.setState({ connection });
  if (connection === 'closed' && useSession.getState().phase === 'signedIn') {
    useSession.setState({ phase: 'signedOut', roomId: null, error: 'The connection to the server was lost.' });
  }
});

/**
 * The server no longer knows us (it restarted, or dropped the session): our tables and games are gone, and an account
 * needs its password again. Back to the sign-in screen, which says why, instead of a shell that only fails.
 */
function expire(): void {
  const { serverUrl, userName, phase } = useSession.getState();
  if (phase !== 'signedIn') return;
  sessionPassword = '';
  writeJson<RememberedRestore | null>(RESTORE_KEY, null);
  forgetOpenGames(serverUrl, userName);
  const before = readJson<RememberedLogin | null>(LOGIN_KEY, null);
  writeJson<RememberedLogin>(LOGIN_KEY, { serverUrl, userName, passwordless: false, account: !!before?.account });
  useSession.setState({ phase: 'signedOut', roomId: null, error: null, expired: true });
}

// a reconnect opens a fresh server session: log back in with the restore token to get our tables back
rpc.onStatus((connection) => {
  const state = useSession.getState();
  if (connection !== 'open' || state.phase !== 'signedIn') return;
  void (async () => {
    try {
      // refused: an account the server no longer holds for our token (it restarted) wants its password again
      const device = sessionPassword ? '' : rememberToken(state.serverUrl, state.userName);
      if (!await connectUser(state.userName, sessionPassword, restoreToken(state.serverUrl, state.userName), device)) return expire();
      await rememberRestoreToken(state.serverUrl, state.userName);
      const roomId = await api.serverGetMainRoomId();
      // the main room is made when the server starts: a new one means a new server, without our tables
      if (state.roomId && roomId !== state.roomId) return expire();
      useSession.setState((current) => ({ roomId, loginCount: current.loginCount + 1 }));
    } catch (error) {
      useSession.setState({ phase: 'signedOut', error: describeError(error) });
    }
  })();
});

// a call the server turned away because this connection has no session any more
rpc.onSessionLost(() => expire());

/**
 * Another sign-in as this player took the session over. Our tables and games now belong to that one, so the restore
 * token and the games to rejoin are kept (takeBack brings them back), but nothing signs in again by itself.
 */
function signedInElsewhere(): void {
  if (useSession.getState().phase !== 'signedIn') return;
  useSession.setState({ phase: 'signedOut', roomId: null, error: null, expired: false, signedInElsewhere: true });
}

rpc.onSignedInElsewhere(() => signedInElsewhere());
