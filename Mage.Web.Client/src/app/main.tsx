import { StrictMode, useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import { QueryClientProvider } from '@tanstack/react-query';
import * as Tooltip from '@radix-ui/react-tooltip';
import { createBrowserRouter, Navigate, Outlet, RouterProvider, useLocation, useNavigate } from 'react-router-dom';
import './styles/base.css';
import { installGlobalErrorHandlers, setReporter } from '../core/telemetry/reporter';
import { serverReporter } from '../core/telemetry/serverReporter';
import { api, rpc } from './connection';
import { AppShell } from './AppShell';
import { i18nReady } from './i18n';
import { queryClient } from './queries';
import { ImportRoute } from './decks/import/ImportRoute';
import { DecksScreen } from './screens/DecksScreen';
import { HomeScreen } from './screens/HomeScreen';
import { LoginScreen } from './screens/LoginScreen';
import { RequestDialog } from './RequestDialog';
import { RouteError } from './screens/RouteError';
import { useEvents } from './stores/events';
import { useGames } from './stores/games';
import { usePlay } from './stores/play';
import { useSession } from './stores/session';
import './stores/lobby';
import './stores/settings';
import './stores/social';
import './stores/toasts';
import { NATIVE_APP } from './brand';

/** Screens that need a signed-in player; also opens games when the server starts them. */
function SignedIn() {
  const phase = useSession((state) => state.phase);
  const navigate = useNavigate();
  const latestGameId = useGames((state) => state.latestGameId);
  const location = useLocation();

  useEffect(() => {
    if (!latestGameId) return;
    usePlay.setState({ phase: 'idle', tableId: null });
    navigate(`/game/${latestGameId}`);
  }, [latestGameId, navigate]);

  // drafts, deck building and tournaments move the player along as the server advances them
  const redirect = useEvents((state) => state.redirect);
  useEffect(() => {
    if (!redirect) return;
    useEvents.getState().clearRedirect();
    navigate(redirect);
  }, [redirect, navigate]);

  // come back here after signing in
  if (phase !== 'signedIn') return <Navigate to="/login" replace state={{ from: location.pathname + location.search + location.hash }} />;
  return (
    <>
      <Outlet />
      <RequestDialog />
    </>
  );
}

// deck sync loads with the first sign-in: servers with accounts keep each account's decks
useSession.subscribe((state) => {
  if (state.phase === 'signedIn') void import('./stores/deckSync');
});

// uncaught errors and unhandled rejections are reported (to the console by default) and kept for debugging
installGlobalErrorHandlers();
// and sent to the game server too, while connected (the admin console lists them)
setReporter(serverReporter(
  (payload) => (rpc.getStatus() === 'open' ? api.clientReportError(payload) : Promise.resolve(false)),
  { path: () => window.location.pathname, userAgent: navigator.userAgent, appVersion: import.meta.env.VITE_APP_VERSION },
));

// installable app with an offline shell; the dev server has no service worker, and the Android app ships its shell
if (import.meta.env.PROD && !NATIVE_APP && 'serviceWorker' in navigator) {
  window.addEventListener('load', () => void import('./pwa').then((m) => m.registerServiceWorker()));
}

const router = createBrowserRouter([
  { path: '/login', element: <LoginScreen /> },
  // an invite link to a contributors-only server: registration with the invite filled in
  { path: '/join', element: <LoginScreen /> },
  // back from Sign in with GitHub
  { path: '/auth/github', element: <LoginScreen /> },
  // credits and legal notices, readable before signing in
  { path: '/about', lazy: () => import('./screens/AboutScreen').then((m) => ({ Component: m.AboutScreen })) },
  { path: '/admin', lazy: () => import('./admin/AdminScreen').then((m) => ({ Component: m.AdminScreen })) },
  {
    element: <SignedIn />,
    errorElement: <RouteError />,
    children: [
      // the heavier screens load when first opened
      { path: '/game/:gameId', lazy: () => import('./screens/GameScreen').then((m) => ({ Component: m.GameScreen })) },
      { path: '/draft/:draftId', lazy: () => import('./events/DraftScreen').then((m) => ({ Component: m.DraftScreen })) },
      { path: '/build/:tableId', lazy: () => import('./events/BuildScreen').then((m) => ({ Component: m.BuildScreen })) },
      { path: '/review/:gameId', lazy: () => import('./review/ReviewScreen').then((m) => ({ Component: m.ReviewScreen })) },
      { path: '/replay/:gameId', lazy: () => import('./replay/ReplayScreen').then((m) => ({ Component: m.ReplayScreen })) },
      {
        element: <AppShell />,
        errorElement: <RouteError />,
        children: [
          { path: '/', element: <HomeScreen /> },
          { path: '/decks', element: <DecksScreen /> },
          { path: '/import', element: <ImportRoute /> },
          { path: '/decks/:deckId', lazy: () => import('./decks/DeckBuilderScreen').then((m) => ({ Component: m.DeckBuilderScreen })) },
          { path: '/events', lazy: () => import('./screens/EventsScreen').then((m) => ({ Component: m.EventsScreen })) },
          { path: '/event/:tournamentId', lazy: () => import('./events/EventScreen').then((m) => ({ Component: m.EventScreen })) },
          { path: '/tables', lazy: () => import('./screens/TablesScreen').then((m) => ({ Component: m.TablesScreen })) },
          // an invite link; signing in first comes back here
          { path: '/join/:tableId', lazy: () => import('./screens/JoinScreen').then((m) => ({ Component: m.JoinScreen })) },
          // Career: opt-in single player with its own collection, offered by servers with accounts
          // Career: opt-in single player with its own collection, offered by servers with accounts. Its screens share one
          // full-bleed shell (the scene, the HUD, menu navigation, the music), so the app's own header steps aside
          {
            path: '/career',
            lazy: () => import('./career/CareerShell').then((m) => ({ Component: m.CareerShell })),
            children: [
              { index: true, lazy: () => import('./career/CareerScreen').then((m) => ({ Component: m.CareerScreen })) },
              { path: 'opponents', lazy: () => import('./career/CareerOpponentsScreen').then((m) => ({ Component: m.CareerOpponentsScreen })) },
              { path: 'deck', lazy: () => import('./career/CareerDeckScreen').then((m) => ({ Component: m.CareerDeckScreen })) },
              { path: 'collection', lazy: () => import('./career/CareerCollectionScreen').then((m) => ({ Component: m.CareerCollectionScreen })) },
              { path: 'shop', lazy: () => import('./career/CareerShopScreen').then((m) => ({ Component: m.CareerShopScreen })) },
              { path: 'progress', lazy: () => import('./career/CareerProgressScreen').then((m) => ({ Component: m.CareerProgressScreen })) },
              { path: 'rewards', lazy: () => import('./career/CareerResultsScreen').then((m) => ({ Component: m.CareerRewardsScreen })) },
              // Career modes (M9): campaign, puzzles, gauntlet, sealed and draft, the weekly challenge; each screen lazy on its own
              // the format schools (M11): the academy's hall, and each school's campaign
              { path: 'academy', lazy: () => import('./career/CareerAcademyScreen').then((m) => ({ Component: m.CareerAcademyScreen })) },
              { path: 'academy/:school', lazy: () => import('./career/CareerAcademyScreen').then((m) => ({ Component: m.CareerAcademyScreen })) },
              { path: ':mode', lazy: () => import('./career/CareerModesRoute').then((m) => ({ Component: m.CareerModesRoute })) },
            ],
          },
          { path: '/train', lazy: () => import('./train/TrainScreen').then((m) => ({ Component: m.TrainScreen })) },
          { path: '/train/:tool', lazy: () => import('./train/TrainScreen').then((m) => ({ Component: m.TrainScreen })) },
          { path: '/history', lazy: () => import('./history/HistoryScreen').then((m) => ({ Component: m.HistoryScreen })) },
        ],
      },
    ],
  },
  { path: '*', element: <Navigate to="/" replace /> },
]);

// a Spanish (or other non-English) player's catalog loads first, so the first paint is already in their language
void i18nReady.then(() => createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <Tooltip.Provider delayDuration={300}>
        <RouterProvider router={router} />
      </Tooltip.Provider>
    </QueryClientProvider>
  </StrictMode>,
));
