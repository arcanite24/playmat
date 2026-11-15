/**
 * The server starts a draft or event only once every human has opened it, so a join lost to a dropped connection
 * left everyone waiting. Retry it a few times; the server also sends the start again to players who haven't joined.
 */
export async function joinWithRetry(join: () => Promise<unknown>, attempts = 4, delayMs = 2000): Promise<void> {
  for (let attempt = 1; ; attempt++) {
    try {
      await join();
      return;
    } catch (error) {
      if (attempt >= attempts) throw error;
      await new Promise((resolve) => setTimeout(resolve, delayMs));
    }
  }
}
