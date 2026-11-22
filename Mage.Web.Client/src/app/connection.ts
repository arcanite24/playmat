import { createApi } from '../protocol/generated/api';
import { RpcClient } from '../core/rpc/RpcClient';
import { EventBus } from '../core/rpc/EventBus';

/** One connection to the server for the whole app; game states come as patches. */
export const rpc = new RpcClient({ stateDiffs: true });
export const events = new EventBus(rpc);
export const api = createApi(rpc);

export interface ServerUrlEnv {
  configured?: string;
  sameOriginWs?: string;
  location?: Pick<Location, 'protocol' | 'host' | 'hostname'>;
}

export function resolveDefaultServerUrl({ configured, sameOriginWs, location }: ServerUrlEnv): string {
  const trimmed = configured?.trim();
  if (trimmed) return trimmed;
  if (!location?.hostname) return 'ws://localhost:17172';
  // served over HTTPS (behind a proxy), or built for the Docker web image whose Caddy always routes /ws on the same
  // origin even over plain HTTP (TLS_MODE=external on a LAN or Tailscale)
  if (location.protocol === 'https:') return `wss://${location.host}/ws`;
  if (sameOriginWs === 'true' && location.protocol === 'http:') return `ws://${location.host}/ws`;
  return `ws://${location.hostname}:17172`;
}

export const DEFAULT_SERVER_URL = resolveDefaultServerUrl({
  configured: import.meta.env.VITE_MAGE_SERVER_URL as string | undefined,
  sameOriginWs: import.meta.env.VITE_SAME_ORIGIN_WS as string | undefined,
  location: typeof window === 'undefined' ? undefined : window.location,
});
