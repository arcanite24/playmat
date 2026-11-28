import { describe, expect, it } from 'vitest';
import { resolveDefaultServerUrl } from './connection';

describe('resolveDefaultServerUrl', () => {
  it('prefers an explicit VITE_MAGE_SERVER_URL over everything else', () => {
    expect(resolveDefaultServerUrl({
      configured: '  wss://custom.example/ws  ',
      sameOriginWs: 'true',
      location: { protocol: 'http:', host: 'playmat.local:8088', hostname: 'playmat.local' },
    })).toBe('wss://custom.example/ws');
  });

  it('uses same-origin wss://.../ws on HTTPS', () => {
    expect(resolveDefaultServerUrl({
      location: { protocol: 'https:', host: 'play.example.com', hostname: 'play.example.com' },
    })).toBe('wss://play.example.com/ws');
    expect(resolveDefaultServerUrl({
      location: { protocol: 'https:', host: 'play.example.com:8443', hostname: 'play.example.com' },
    })).toBe('wss://play.example.com:8443/ws');
  });

  it('uses same-origin ws://.../ws on plain HTTP when built for the Docker web image', () => {
    expect(resolveDefaultServerUrl({
      sameOriginWs: 'true',
      location: { protocol: 'http:', host: 'servername.local:8088', hostname: 'servername.local' },
    })).toBe('ws://servername.local:8088/ws');
    expect(resolveDefaultServerUrl({
      sameOriginWs: 'true',
      location: { protocol: 'http:', host: '100.64.0.10', hostname: '100.64.0.10' },
    })).toBe('ws://100.64.0.10/ws');
  });

  it('falls back to ws://<hostname>:17172 on plain HTTP for non-Docker local/LAN builds', () => {
    expect(resolveDefaultServerUrl({
      location: { protocol: 'http:', host: '192.168.1.50:5173', hostname: '192.168.1.50' },
    })).toBe('ws://192.168.1.50:17172');
    expect(resolveDefaultServerUrl({
      sameOriginWs: 'false',
      location: { protocol: 'http:', host: 'localhost:5173', hostname: 'localhost' },
    })).toBe('ws://localhost:17172');
  });

  it('falls back to ws://localhost:17172 when window.location is absent or has no hostname', () => {
    expect(resolveDefaultServerUrl({})).toBe('ws://localhost:17172');
    expect(resolveDefaultServerUrl({
      sameOriginWs: 'true',
      location: { protocol: 'file:', host: '', hostname: '' },
    })).toBe('ws://localhost:17172');
  });
});
