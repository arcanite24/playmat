import { lazy, Suspense, type ComponentType, type LazyExoticComponent } from 'react';
import { Navigate, useParams } from 'react-router-dom';

/**
 * The Career modes under one route (/career/campaign, /career/puzzles, ...), each screen still its own lazy chunk: the
 * entry keeps a single route for them rather than a preload list per screen.
 */
const SCREENS: Record<string, LazyExoticComponent<ComponentType>> = {
  campaign: lazy(() => import('./CareerCampaignScreen').then((m) => ({ default: m.CareerCampaignScreen }))),
  puzzles: lazy(() => import('./CareerPuzzlesScreen').then((m) => ({ default: m.CareerPuzzlesScreen }))),
  gauntlet: lazy(() => import('./CareerGauntletScreen').then((m) => ({ default: m.CareerGauntletScreen }))),
  limited: lazy(() => import('./CareerLimitedScreen').then((m) => ({ default: m.CareerLimitedScreen }))),
  challenge: lazy(() => import('./CareerChallengeScreen').then((m) => ({ default: m.CareerChallengeScreen }))),
  hordes: lazy(() => import('./CareerHordesScreen').then((m) => ({ default: m.CareerHordesScreen }))),
};

export function CareerModesRoute() {
  const { mode = '' } = useParams();
  const Screen = Object.hasOwn(SCREENS, mode) ? SCREENS[mode] : null;
  if (!Screen) return <Navigate to="/career" replace />;
  return (
    <Suspense fallback={null}>
      <Screen />
    </Suspense>
  );
}
