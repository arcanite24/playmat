import { describe, expect, test, vi } from 'vitest';
import { joinWithRetry } from './joinRetry';

describe('joinWithRetry', () => {
  test('joins once when the first call works', async () => {
    const join = vi.fn().mockResolvedValue(true);
    await joinWithRetry(join, 4, 0);
    expect(join).toHaveBeenCalledTimes(1);
  });

  test('tries again after a failed join', async () => {
    const join = vi.fn().mockRejectedValueOnce(new Error('socket closed')).mockResolvedValue(true);
    await joinWithRetry(join, 4, 0);
    expect(join).toHaveBeenCalledTimes(2);
  });

  test('gives up with the last error after the last attempt', async () => {
    const join = vi.fn().mockRejectedValue(new Error('socket closed'));
    await expect(joinWithRetry(join, 3, 0)).rejects.toThrow('socket closed');
    expect(join).toHaveBeenCalledTimes(3);
  });
});
