import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { useQuery } from '@tanstack/react-query';
import { IdCard, Info, LogOut, MessageSquare, Settings, ShieldCheck, WifiOff, Wrench } from 'lucide-react';
import { lazy, Suspense, useState } from 'react';
import { NavLink, Outlet, useMatch, useNavigate } from 'react-router-dom';
import { APP_NAME } from './brand';
import { api } from './connection';
import { useT, type MessageKey } from './i18n';
import { useServerInfo } from './queries';
import { Avatar } from './social/Avatar';
import { LobbyButton } from './social/LobbyButton';
import { useImportSheet } from './stores/importSheet';
import { useLobby } from './stores/lobby';
import { useSettings } from './stores/settings';
import { useSession } from './stores/session';
import { useShellUi } from './stores/shellUi';
import { Toaster } from './ui/Toaster';
import { IconButton } from './ui/Button';
import { Mark } from './ui/Mark';
import { Stitch } from './ui/Stitch';
import styles from './AppShell.module.css';

const NAV: { to: string; label: MessageKey; end?: boolean }[] = [
  { to: '/', label: 'shell.nav.play', end: true },
  { to: '/decks', label: 'shell.nav.decks' },
  { to: '/events', label: 'shell.nav.events' },
  { to: '/tables', label: 'shell.nav.tables' },
  { to: '/train', label: 'shell.nav.train' },
  { to: '/history', label: 'shell.nav.history' },
];

// the import sheet loads the first time it opens, then stays mounted so it can animate out
const ImportSheet = lazy(() => import('./decks/import/ImportSheet').then((module) => ({ default: module.ImportSheet })));

// the lobby drawer and profiles load the first time they open
const LobbyDrawer = lazy(() => import('./social/LobbyDrawer').then((module) => ({ default: module.LobbyDrawer })));
const ProfileDialog = lazy(() => import('./social/ProfileDialog').then((module) => ({ default: module.ProfileDialog })));
const ReportDialog = lazy(() => import('./social/ReportDialog').then((module) => ({ default: module.ReportDialog })));

// settings load the first time they open, then stay mounted so the dialog can animate out
const SettingsDialog = lazy(() => import('./screens/SettingsDialog').then((module) => ({ default: module.SettingsDialog })));

const FeedbackDialog = lazy(() => import('./screens/FeedbackDialog').then((module) => ({ default: module.FeedbackDialog })));

/** Planned maintenance, from the server info: when it starts, and what it means for new tables. */
function MaintenanceChip() {
  const t = useT();
  const info = useServerInfo();
  const at = info.data?.maintenanceAt ?? 0;
  if (!at) return null;
  const now = info.dataUpdatedAt;
  const label = at > now
    ? t('shell.maintenance', { time: new Date(at).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' }) })
    : t('shell.maintenanceNow');
  const message = info.data?.maintenanceMessage?.trim();
  return (
    <span className={styles.maintenance} role="status" title={[t('shell.maintenanceHint'), message].filter(Boolean).join(' ')}>
      <Wrench size={16} aria-hidden="true" />
      {label}
    </span>
  );
}

function FeedbackHost() {
  const open = useShellUi((state) => state.feedbackOpen);
  const setOpen = useShellUi((state) => state.setFeedbackOpen);
  const [loaded, setLoaded] = useState(false);
  if (open && !loaded) setLoaded(true);
  return loaded ? <Suspense fallback={null}><FeedbackDialog open={open} onOpenChange={setOpen} /></Suspense> : null;
}

function SettingsHost({ open, onOpenChange }: { open: boolean; onOpenChange(open: boolean): void }) {
  const [loaded, setLoaded] = useState(false);
  if (open && !loaded) setLoaded(true);
  return loaded ? <Suspense fallback={null}><SettingsDialog open={open} onOpenChange={onOpenChange} /></Suspense> : null;
}

function SocialHost() {
  const lobbyOpened = useLobby((state) => state.open || state.profile !== null);
  const [loaded, setLoaded] = useState(false);
  if (lobbyOpened && !loaded) setLoaded(true);
  return loaded ? <Suspense fallback={null}><LobbyDrawer /><ProfileDialog /><ReportDialog /></Suspense> : null;
}

function ImportSheetHost() {
  const opened = useImportSheet((state) => state.session > 0);
  return opened ? <Suspense fallback={null}><ImportSheet /></Suspense> : null;
}

/** The mat every screen outside a match is printed on. */
export function AppShell() {
  const userName = useSession((state) => state.userName);
  const connection = useSession((state) => state.connection);
  const signOut = useSession((state) => state.signOut);
  const avatarId = useSettings((state) => state.settings.avatarId);
  const navigate = useNavigate();
  const settingsOpen = useShellUi((state) => state.settingsOpen);
  const setSettingsOpen = useShellUi((state) => state.setSettingsOpen);
  // Career is a game mode: its screens fill the mat under their own header
  const immersive = useMatch('/career/*') !== null;
  const t = useT();
  // staff get Admin in the account menu; the role is checked again on every admin action
  const staff = useQuery({ queryKey: ['staffMe', userName], queryFn: () => api.staffMe(), enabled: connection === 'open', staleTime: 5 * 60_000 });

  return (
    <div className={[styles.mat, immersive ? styles.immersive : ''].join(' ')}>
      <Stitch />
      {!immersive && (
        <header className={styles.rail}>
          <NavLink to="/" className={styles.brand} aria-label={t('shell.home', { app: APP_NAME })}>
            <Mark />
            <span>{APP_NAME}</span>
          </NavLink>
          <nav className={styles.nav} aria-label={t('shell.nav')}>
            {NAV.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                end={item.end}
                className={({ isActive }) => [styles.navItem, isActive ? styles.active : ''].join(' ')}
              >
                {t(item.label)}
              </NavLink>
            ))}
          </nav>
          <div className={styles.tools}>
            {connection !== 'open' && (
              <span className={styles.offline} role="status">
                <WifiOff size={16} aria-hidden="true" />
                {connection === 'reconnecting' ? t('shell.reconnecting') : t('shell.offline')}
              </span>
            )}
            <MaintenanceChip />
            <LobbyButton />
            <IconButton label={t('shell.settings')} icon={<Settings size={18} />} onClick={() => setSettingsOpen(true)} />
            <DropdownMenu.Root>
              <DropdownMenu.Trigger className={styles.user} aria-label={t('shell.account', { name: userName })}>
                <Avatar name={userName} avatarId={avatarId} size="sm" />
                <span>{userName}</span>
              </DropdownMenu.Trigger>
              <DropdownMenu.Portal>
                <DropdownMenu.Content className={styles.menu} align="end" sideOffset={8}>
                  <DropdownMenu.Item className={styles.menuItem} onSelect={() => useLobby.getState().openProfile(userName)}>
                    <IdCard size={16} aria-hidden="true" /> {t('shell.profile')}
                  </DropdownMenu.Item>
                  <DropdownMenu.Item className={styles.menuItem} onSelect={() => setSettingsOpen(true)}>
                    <Settings size={16} aria-hidden="true" /> {t('shell.settings')}
                  </DropdownMenu.Item>
                  {staff.data?.role && (
                    <DropdownMenu.Item className={styles.menuItem} onSelect={() => navigate('/admin')}>
                      <ShieldCheck size={16} aria-hidden="true" /> {t('shell.admin')}
                    </DropdownMenu.Item>
                  )}
                  <DropdownMenu.Item className={styles.menuItem} onSelect={() => useShellUi.getState().setFeedbackOpen(true)}>
                    <MessageSquare size={16} aria-hidden="true" /> {t('shell.feedback')}
                  </DropdownMenu.Item>
                  <DropdownMenu.Item className={styles.menuItem} onSelect={() => navigate('/about')}>
                    <Info size={16} aria-hidden="true" /> {t('shell.about')}
                  </DropdownMenu.Item>
                  <DropdownMenu.Separator className={styles.menuRule} />
                  <DropdownMenu.Item className={styles.menuItem} onSelect={() => void signOut(false)}>
                    <LogOut size={16} aria-hidden="true" /> {t('shell.signOut')}
                  </DropdownMenu.Item>
                </DropdownMenu.Content>
              </DropdownMenu.Portal>
            </DropdownMenu.Root>
          </div>
        </header>
      )}
      <main className={styles.main}>
        <Outlet />
      </main>
      <SocialHost />
      <SettingsHost open={settingsOpen} onOpenChange={setSettingsOpen} />
      <FeedbackHost />
      <ImportSheetHost />
      <Toaster />
    </div>
  );
}
