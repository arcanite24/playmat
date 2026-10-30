// @vitest-environment happy-dom
import { beforeEach, expect, test, vi } from 'vitest';
import { createFakeConnection } from '../../test/fakeConnection';

beforeEach(() => {
  vi.resetModules();
  vi.doMock('../connection', () => createFakeConnection());
  localStorage.clear();
});

test.each([true, false])('floating mana can pay independently of automatic land tapping (%s)', async (tapLands) => {
  const { DEFAULT_SETTINGS, toUserData } = await import('./settings');
  expect(toUserData({ ...DEFAULT_SETTINGS, autoPayMana: tapLands, autoPayRestricted: false })).toMatchObject({
    manaPoolAutomatic: true, manaPoolAutomaticRestricted: false,
  });
  expect(toUserData({ ...DEFAULT_SETTINGS, autoPayMana: tapLands, autoPayRestricted: true })).toMatchObject({
    manaPoolAutomatic: tapLands, manaPoolAutomaticRestricted: true,
  });
});

test('preserves defaults and saved floating mana preferences without migration', async () => {
  localStorage.setItem('playmat.settings', JSON.stringify({ autoPayMana: false, autoPayRestricted: false }));
  const { DEFAULT_SETTINGS, useSettings, toUserData } = await import('./settings');
  expect(toUserData(DEFAULT_SETTINGS)).toMatchObject({ manaPoolAutomatic: true, manaPoolAutomaticRestricted: true });
  expect(toUserData(useSettings.getState().settings)).toMatchObject({ manaPoolAutomatic: true, manaPoolAutomaticRestricted: false });
  useSettings.getState().update({ autoPayRestricted: true });
  expect(JSON.parse(localStorage.getItem('playmat.settings')!)).toMatchObject({ autoPayRestricted: true });
});

test('full control pauses floating mana payments and the preset disables them', async () => {
  const { DEFAULT_SETTINGS, FULL_CONTROL, toUserData } = await import('./settings');
  const settings = { ...DEFAULT_SETTINGS, autoPayRestricted: false };
  expect(toUserData(settings, true).manaPoolAutomatic).toBe(false);
  expect(toUserData(settings).manaPoolAutomatic).toBe(true);
  expect(toUserData({ ...settings, ...FULL_CONTROL })).toMatchObject({ manaPoolAutomatic: false, manaPoolAutomaticRestricted: true });
});
