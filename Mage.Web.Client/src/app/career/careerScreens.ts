import type { MessageKey } from '../i18n';

/** Each Career screen's name in the header, and where its back button goes. */
const SCREENS: { path: string; title: MessageKey; back: string }[] = [
  // a school's page goes back to the academy's hall
  { path: '/career/academy/', title: 'career.academy.title', back: '/career/academy' },
  { path: '/career/academy', title: 'career.academy.title', back: '/career' },
  { path: '/career/opponents', title: 'career.nav.home', back: '/career' },
  { path: '/career/deck', title: 'career.nav.deck', back: '/career' },
  { path: '/career/collection', title: 'career.nav.collection', back: '/career' },
  { path: '/career/shop', title: 'career.nav.shop', back: '/career' },
  // a machine goes back to the storefront
  { path: '/career/capsules/', title: 'capsules.title', back: '/career/capsules' },
  { path: '/career/capsules', title: 'capsules.title', back: '/career' },
  { path: '/career/progress', title: 'career.nav.progress', back: '/career' },
  { path: '/career/rewards', title: 'career.rewards.title', back: '/career' },
  { path: '/career/campaign', title: 'career.modes.campaign', back: '/career' },
  { path: '/career/puzzles', title: 'career.modes.puzzles', back: '/career' },
  { path: '/career/gauntlet', title: 'career.modes.gauntlet', back: '/career' },
  { path: '/career/limited', title: 'career.modes.limited', back: '/career' },
  { path: '/career/challenge', title: 'career.modes.challenge', back: '/career' },
  { path: '/career/hordes', title: 'career.modes.hordes', back: '/career' },
];

export function screenOf(pathname: string): { title: MessageKey | null; back: string } {
  const path = pathname.replace(/\/+$/, '');
  const screen = SCREENS.find((candidate) => candidate.path.endsWith('/')
    ? path.startsWith(candidate.path)
    : path === candidate.path || path.startsWith(`${candidate.path}/`));
  return screen ? { title: screen.title, back: screen.back } : { title: null, back: '/' };
}
