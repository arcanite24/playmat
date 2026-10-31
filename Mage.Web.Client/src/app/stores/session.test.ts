// @vitest-environment happy-dom
import { beforeEach, describe, expect, test, vi } from 'vitest';
import { createFakeConnection, type FakeConnection } from '../../test/fakeConnection';


const SERVER = 'ws://server:17172';

async function load() {
  const { useSession } = await import('./session');
  return { useSession, fake: (await import('../connection')) as unknown as FakeConnection };
}

function restoreSaved() {
  return JSON.parse(localStorage.getItem('playmat.restore') ?? 'null');
}

async function flush() {
  for (let i = 0; i < 5; i++) await Promise.resolve();
}

// the store wires itself to the connection at import: every test gets a fresh store on a fresh fake connection
function freshModules() {
  vi.resetModules();
  vi.doMock('../connection', () => createFakeConnection());
}

beforeEach(() => {
  freshModules();
  localStorage.clear();
});

describe('session store', () => {
  test('signs in, remembers the login and the restore token', async () => {
    const { useSession, fake } = await load();
    expect(await useSession.getState().signIn(SERVER, 'alice', '')).toBe(true);

    expect(fake.rpc.connect).toHaveBeenCalledWith(SERVER);
    expect(fake.api.connectUser).toHaveBeenCalledWith('alice', '', '');
    expect(useSession.getState()).toMatchObject({ phase: 'signedIn', roomId: 'room-1', userName: 'alice', error: null });
    expect(restoreSaved()).toEqual({ serverUrl: SERVER, userName: 'alice', token: 'token-1' });
    expect(JSON.parse(localStorage.getItem('playmat.login')!)).toEqual({ serverUrl: SERVER, userName: 'alice', passwordless: true, account: false });
  });

  test('sends the saved restore token only to the same server and user', async () => {
    localStorage.setItem('playmat.restore', JSON.stringify({ serverUrl: SERVER, userName: 'alice', token: 'old' }));
    const { useSession, fake } = await load();
    await useSession.getState().signIn(SERVER, 'alice', 'pw');
    expect(fake.api.connectUser).toHaveBeenLastCalledWith('alice', 'pw', 'old');

    await useSession.getState().signOut(true);
    await useSession.getState().signIn(SERVER, 'bob', '');
    expect(fake.api.connectUser).toHaveBeenLastCalledWith('bob', '', '');
  });

  test('a refused login stays signed out with an explanation', async () => {
    const { useSession, fake } = await load();
    fake.api.connectUser.mockResolvedValueOnce(false);
    expect(await useSession.getState().signIn(SERVER, 'alice', 'bad')).toBe(false);
    expect(useSession.getState().phase).toBe('signedOut');
    expect(useSession.getState().error).toMatch(/refused/);
    expect(restoreSaved()).toBeNull();
  });

  test('an older server without restore tokens still signs in', async () => {
    const { useSession, fake } = await load();
    fake.api.sessionGetRestoreToken.mockRejectedValueOnce(new Error('unknown method'));
    expect(await useSession.getState().signIn(SERVER, 'alice', '')).toBe(true);
    expect(restoreSaved()).toBeNull();
  });

  test('a reconnect logs back in with the password and the newest restore token', async () => {
    const { useSession, fake } = await load();
    await useSession.getState().signIn(SERVER, 'alice', 'secret');
    fake.api.sessionGetRestoreToken.mockResolvedValueOnce('token-2');

    fake.rpc.setStatus('reconnecting');
    expect(useSession.getState()).toMatchObject({ phase: 'signedIn', connection: 'reconnecting' });
    fake.rpc.setStatus('open');
    await flush();

    expect(fake.api.connectUser).toHaveBeenLastCalledWith('alice', 'secret', 'token-1');
    await vi.waitFor(() => expect(useSession.getState().loginCount).toBe(2));
    expect(useSession.getState()).toMatchObject({ phase: 'signedIn', roomId: 'room-1', expired: false });
    expect(restoreSaved()).toMatchObject({ token: 'token-2' });
  });

  test('preferences go to the server after every login, the silent one after a dropped connection too', async () => {
    const { useSession, fake } = await load();
    await import('./settings');
    await useSession.getState().signIn(SERVER, 'alice', '');
    await vi.waitFor(() => expect(fake.api.connectSetUserData).toHaveBeenCalledTimes(1));

    // the server forgets them with the old session: without them it skips the beginning of combat (Greasefang)
    fake.rpc.setStatus('reconnecting');
    fake.rpc.setStatus('open');
    await vi.waitFor(() => expect(fake.api.connectSetUserData).toHaveBeenCalledTimes(2));
    const userData = fake.api.connectSetUserData.mock.lastCall?.[1] as { userSkipPrioritySteps: { yourTurn: { beforeCombat: boolean } } };
    expect(userData.userSkipPrioritySteps.yourTurn.beforeCombat).toBe(true);
  });

  test('a reconnect to a restarted server sends the player to sign in again', async () => {
    const { useSession, fake } = await load();
    await useSession.getState().signIn(SERVER, 'alice', '');
    // a new server: a new main room
    fake.api.serverGetMainRoomId.mockResolvedValueOnce('room-2');
    fake.rpc.setStatus('reconnecting');
    fake.rpc.setStatus('open');
    await vi.waitFor(() => expect(useSession.getState().phase).toBe('signedOut'));
    expect(useSession.getState()).toMatchObject({ roomId: null, error: null, expired: true });
    expect(restoreSaved()).toBeNull();
    // the sign-in screen stays put instead of signing back in silently
    expect(JSON.parse(localStorage.getItem('playmat.login')!)).toMatchObject({ userName: 'alice', passwordless: false });
  });

  test('a reconnect the restarted server refuses (an account without its password) signs out the same way', async () => {
    const { useSession, fake } = await load();
    await useSession.getState().signIn(SERVER, 'alice', '');
    fake.api.connectUser.mockResolvedValueOnce(false);
    fake.rpc.setStatus('reconnecting');
    fake.rpc.setStatus('open');
    await vi.waitFor(() => expect(useSession.getState().expired).toBe(true));
    expect(useSession.getState().phase).toBe('signedOut');
    expect(fake.api.serverGetMainRoomId).toHaveBeenCalledTimes(1);
  });

  test('a call the server turns away for want of a session signs out', async () => {
    const { useSession, fake } = await load();
    await useSession.getState().signIn(SERVER, 'alice', '');
    fake.rpc.loseSession('roomCreateTable');
    expect(useSession.getState()).toMatchObject({ phase: 'signedOut', expired: true });
  });

  test('signing in again clears the restart notice', async () => {
    const { useSession, fake } = await load();
    await useSession.getState().signIn(SERVER, 'alice', '');
    fake.api.serverGetMainRoomId.mockResolvedValueOnce('room-2');
    fake.rpc.setStatus('reconnecting');
    fake.rpc.setStatus('open');
    await vi.waitFor(() => expect(useSession.getState().expired).toBe(true));
    await useSession.getState().signIn(SERVER, 'alice', '');
    expect(useSession.getState()).toMatchObject({ phase: 'signedIn', expired: false });
  });

  test('a failed reconnect login signs the player out', async () => {
    const { useSession, fake } = await load();
    await useSession.getState().signIn(SERVER, 'alice', '');
    fake.api.connectUser.mockRejectedValueOnce(new Error('session expired'));
    fake.rpc.setStatus('reconnecting');
    fake.rpc.setStatus('open');
    await vi.waitFor(() => expect(useSession.getState().phase).toBe('signedOut'));
    expect(useSession.getState().error).toBe('session expired');
  });

  test('a closed connection signs out with a message', async () => {
    const { useSession, fake } = await load();
    await useSession.getState().signIn(SERVER, 'alice', '');
    fake.rpc.setStatus('closed');
    expect(useSession.getState()).toMatchObject({ phase: 'signedOut', roomId: null, error: 'The connection to the server was lost.' });
  });

  test('signing out for good forgets the restore token; keeping games keeps it', async () => {
    const { useSession, fake } = await load();
    await useSession.getState().signIn(SERVER, 'alice', '');
    await useSession.getState().signOut(true);
    expect(fake.api.disconnectSession).toHaveBeenCalledWith(true);
    expect(restoreSaved()).toMatchObject({ token: 'token-1' });

    await useSession.getState().signIn(SERVER, 'alice', '');
    await useSession.getState().signOut();
    expect(fake.api.disconnectSession).toHaveBeenLastCalledWith(false);
    expect(restoreSaved()).toBeNull();
    expect(JSON.parse(localStorage.getItem('playmat.login')!)).toMatchObject({ passwordless: false });
    expect(useSession.getState()).toMatchObject({ phase: 'signedOut', roomId: null });
  });

  test('resume signs a passwordless player back in, and nobody else', async () => {
    localStorage.setItem('playmat.login', JSON.stringify({ serverUrl: SERVER, userName: 'alice', passwordless: true }));
    let loaded = await load();
    expect(loaded.useSession.getState()).toMatchObject({ serverUrl: SERVER, userName: 'alice' });
    expect(await loaded.useSession.getState().resume()).toBe(true);
    expect(loaded.fake.api.connectUser).toHaveBeenCalledWith('alice', '', '');

    freshModules();
    localStorage.setItem('playmat.login', JSON.stringify({ serverUrl: SERVER, userName: 'alice', passwordless: false }));
    loaded = await load();
    expect(await loaded.useSession.getState().resume()).toBe(false);
    expect(loaded.fake.api.connectUser).not.toHaveBeenCalled();
  });

  test('a failed resume does not show an error on the sign-in screen', async () => {
    localStorage.setItem('playmat.login', JSON.stringify({ serverUrl: SERVER, userName: 'alice', passwordless: true }));
    const { useSession, fake } = await load();
    fake.api.connectUser.mockResolvedValueOnce(false);
    expect(await useSession.getState().resume()).toBe(false);
    expect(useSession.getState().error).toBeNull();
  });

  describe('single sign-on', () => {
    function loginSaved() {
      return JSON.parse(localStorage.getItem('playmat.login') ?? 'null');
    }

    test('signs in as the signed-on account without a password, and a reload does it again', async () => {
      let loaded = await load();
      expect(await loaded.useSession.getState().signInWithSignOn(SERVER, 'alice', true)).toBe(true);
      expect(loaded.fake.api.connectUser).toHaveBeenCalledWith('alice', '', '');
      expect(loginSaved()).toEqual({ serverUrl: SERVER, userName: 'alice', passwordless: false, account: true, signOn: true });

      freshModules();
      loaded = await load();
      expect(await loaded.useSession.getState().resume()).toBe(true);
      expect(loaded.fake.api.connectUser).toHaveBeenCalledWith('alice', '', 'token-1');
      expect(loginSaved().signOn).toBe(true);
    });

    test('after signing out, only the player signs back in', async () => {
      const { useSession, fake } = await load();
      await useSession.getState().signInWithSignOn(SERVER, 'alice');
      await useSession.getState().signOut();
      fake.api.connectUser.mockClear();

      expect(await useSession.getState().resume()).toBe(false);
      expect(await useSession.getState().signInWithSignOn(SERVER, 'alice', true)).toBe(false);
      expect(fake.api.connectUser).not.toHaveBeenCalled();
      expect(await useSession.getState().signInWithSignOn(SERVER, 'alice')).toBe(true);
    });

    test('a password sign-in is not remembered as a sign-on', async () => {
      const { useSession } = await load();
      await useSession.getState().signInWithSignOn(SERVER, 'alice');
      await useSession.getState().signOut();
      await useSession.getState().signIn(SERVER, 'alice', 'secret');
      expect(loginSaved().signOn).toBeUndefined();
    });
  });

  describe('keep me signed in', () => {
    function deviceSaved() {
      return JSON.parse(localStorage.getItem('playmat.remember') ?? 'null');
    }

    test('a password sign-in with remember keeps the device token, and a reload signs in with it', async () => {
      const first = await load();
      expect(await first.useSession.getState().signIn(SERVER, 'alice', 'secret', true)).toBe(true);
      expect(first.fake.api.authRememberLogin).toHaveBeenCalled();
      expect(deviceSaved()).toEqual({ serverUrl: SERVER, userName: 'alice', token: 'device-1' });

      // a reload after the server forgot us: no restore token left, only the device
      localStorage.removeItem('playmat.restore');
      freshModules();
      const second = await load();
      expect(await second.useSession.getState().resume()).toBe(true);
      expect(second.fake.api.connectUser).toHaveBeenCalledWith('alice', '', '', null, null, 'device-1');
      expect(second.useSession.getState().phase).toBe('signedIn');
    });

    test('without remember an account still needs its password after the server forgets it', async () => {
      const first = await load();
      await first.useSession.getState().signIn(SERVER, 'alice', 'secret', false);
      expect(first.fake.api.authRememberLogin).not.toHaveBeenCalled();
      expect(deviceSaved()).toBeNull();

      localStorage.removeItem('playmat.restore');
      freshModules();
      const second = await load();
      expect(await second.useSession.getState().resume()).toBe(false);
      expect(second.fake.api.connectUser).not.toHaveBeenCalled();
    });

    test('signing out forgets the device on the server too', async () => {
      const { useSession, fake } = await load();
      await useSession.getState().signIn(SERVER, 'alice', 'secret', true);
      await useSession.getState().signOut();
      expect(fake.api.authForgetLogin).toHaveBeenCalledWith('alice', 'device-1');
      expect(deviceSaved()).toBeNull();
    });

    test('a device token the server turns away is dropped', async () => {
      localStorage.setItem('playmat.login', JSON.stringify({ serverUrl: SERVER, userName: 'alice', account: true }));
      localStorage.setItem('playmat.remember', JSON.stringify({ serverUrl: SERVER, userName: 'alice', token: 'revoked' }));
      const { useSession, fake } = await load();
      fake.api.connectUser.mockResolvedValueOnce(false);
      expect(await useSession.getState().resume()).toBe(false);
      expect(deviceSaved()).toBeNull();
      expect(useSession.getState().error).toBeNull();
      // the sign-in screen says why it asks for the password again (a password reset signs every device out)
      expect(useSession.getState().staleLogin).toBe(true);
    });
  });

  describe('signed in somewhere else', () => {
    function deviceSaved() {
      return JSON.parse(localStorage.getItem('playmat.remember') ?? 'null');
    }

    test('steps aside without signing back in, so two tabs cannot keep taking the session from each other', async () => {
      const { useSession, fake } = await load();
      await useSession.getState().signIn(SERVER, 'alice', 'secret', true);
      fake.rpc.signInElsewhere();

      expect(useSession.getState()).toMatchObject({ phase: 'signedOut', roomId: null, signedInElsewhere: true, expired: false, error: null });
      // the sign-in screen's resume, and a reconnect, both stay put
      expect(await useSession.getState().resume()).toBe(false);
      fake.rpc.setStatus('reconnecting');
      fake.rpc.setStatus('open');
      await flush();
      expect(fake.api.connectUser).toHaveBeenCalledTimes(1);
      expect(useSession.getState().phase).toBe('signedOut');
      // nothing is forgotten: the session comes back here with them
      expect(restoreSaved()).toMatchObject({ token: 'token-1' });
      expect(deviceSaved()).toMatchObject({ token: 'device-1' });
    });

    test('a lost connection still signs back in by itself', async () => {
      const { useSession, fake } = await load();
      await useSession.getState().signIn(SERVER, 'alice', 'secret', true);
      fake.rpc.setStatus('reconnecting');
      fake.rpc.setStatus('open');
      await vi.waitFor(() => expect(useSession.getState().loginCount).toBe(2));
      expect(useSession.getState()).toMatchObject({ phase: 'signedIn', signedInElsewhere: false });
    });

    test('use here takes the session back with the remembered device', async () => {
      const { useSession, fake } = await load();
      await useSession.getState().signIn(SERVER, 'alice', 'secret', true);
      fake.rpc.signInElsewhere();

      expect(await useSession.getState().takeBack()).toBe(true);
      expect(fake.api.connectUser).toHaveBeenLastCalledWith('alice', '', 'token-1', null, null, 'device-1');
      expect(useSession.getState()).toMatchObject({ phase: 'signedIn', signedInElsewhere: false, loginCount: 2 });
      expect(deviceSaved()).toMatchObject({ token: 'device-1' });
    });

    test('use here without a remembered device signs in with the password typed in this tab', async () => {
      const { useSession, fake } = await load();
      await useSession.getState().signIn(SERVER, 'alice', 'secret');
      fake.rpc.signInElsewhere();

      expect(await useSession.getState().takeBack()).toBe(true);
      expect(fake.api.connectUser).toHaveBeenLastCalledWith('alice', 'secret', 'token-1');
    });

    test('use here after a password change elsewhere ends on the sign-in form, saying why', async () => {
      const { useSession, fake } = await load();
      await useSession.getState().signIn(SERVER, 'alice', 'secret', true);
      fake.rpc.signInElsewhere();
      // the reset signed every remembered device out
      fake.api.connectUser.mockResolvedValueOnce(false);

      expect(await useSession.getState().takeBack()).toBe(false);
      expect(useSession.getState()).toMatchObject({ phase: 'signedOut', signedInElsewhere: false, staleLogin: true, error: null });
      expect(deviceSaved()).toBeNull();
    });

    test('a refused password from this tab says the same', async () => {
      const { useSession, fake } = await load();
      await useSession.getState().signIn(SERVER, 'alice', 'old-password');
      fake.rpc.signInElsewhere();
      fake.api.connectUser.mockResolvedValueOnce(false);

      expect(await useSession.getState().takeBack()).toBe(false);
      expect(useSession.getState()).toMatchObject({ phase: 'signedOut', staleLogin: true, error: null });
    });

    test('signing in, or out, clears it', async () => {
      const { useSession, fake } = await load();
      await useSession.getState().signIn(SERVER, 'alice', '');
      fake.rpc.signInElsewhere();
      await useSession.getState().signIn(SERVER, 'alice', '');
      expect(useSession.getState().signedInElsewhere).toBe(false);

      fake.rpc.signInElsewhere();
      await useSession.getState().signOut();
      expect(useSession.getState()).toMatchObject({ signedInElsewhere: false, staleLogin: false });
    });
  });
});
