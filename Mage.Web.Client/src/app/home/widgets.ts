import type { ComponentType } from 'react';
import type { MessageKey } from '../i18n';
import type { LayoutItem, WidgetSpec } from './layoutModel';

/**
 * The Home widgets (#432). This file is data plus a loader per widget, so it stays tiny in the entry chunk: each
 * widget's code is its own lazy chunk, loaded only when it is on the board. Ids and sizes must match the server's
 * list (Mage.Server/src/main/resources/home/widgets.json), which checks saved layouts.
 */

export interface WidgetProps {
  item: LayoutItem;
  /** true in Customize: links and buttons inside the widget are inert */
  editing: boolean;
}

export interface WidgetDef extends WidgetSpec {
  title: MessageKey;
  /** one line for the Add widget gallery */
  blurb: MessageKey;
  load: () => Promise<{ default: ComponentType<WidgetProps> }>;
}

const FORMATS = ['standard', 'pioneer'] as const;
export const LEADERBOARDS = ['gauntlet', 'ascension', 'boss-rush', 'endless'] as const;

export const WIDGETS: readonly WidgetDef[] = [
  { id: 'rank', sizes: ['s', 'm'], needs: 'ranked', settings: { format: FORMATS }, title: 'home.widget.rank', blurb: 'home.widget.rank.blurb', load: () => import('./widgets/RankWidget') },
  { id: 'allRanks', sizes: ['m', 'l'], needs: 'ranked', title: 'home.widget.allRanks', blurb: 'home.widget.allRanks.blurb', load: () => import('./widgets/AllRanksWidget') },
  { id: 'rival', sizes: ['s', 'm'], needs: 'ranked', settings: { format: FORMATS }, title: 'home.widget.rival', blurb: 'home.widget.rival.blurb', load: () => import('./widgets/RivalWidget') },
  { id: 'recent', sizes: ['m', 'l'], needs: 'career', title: 'home.widget.recent', blurb: 'home.widget.recent.blurb', load: () => import('./widgets/RecentWidget') },
  { id: 'quests', sizes: ['m', 'l'], needs: 'career', title: 'home.widget.quests', blurb: 'home.widget.quests.blurb', load: () => import('./widgets/QuestsWidget') },
  { id: 'daily', sizes: ['s', 'm'], needs: 'career', title: 'home.widget.daily', blurb: 'home.widget.daily.blurb', load: () => import('./widgets/DailyWidget') },
  { id: 'weekly', sizes: ['s'], needs: 'career', title: 'home.widget.weekly', blurb: 'home.widget.weekly.blurb', load: () => import('./widgets/WeeklyWidget') },
  { id: 'level', sizes: ['s', 'm'], needs: 'career', title: 'home.widget.level', blurb: 'home.widget.level.blurb', load: () => import('./widgets/LevelWidget') },
  { id: 'packs', sizes: ['s', 'm'], needs: 'career', title: 'home.widget.packs', blurb: 'home.widget.packs.blurb', load: () => import('./widgets/PacksWidget') },
  { id: 'collection', sizes: ['m', 'l'], needs: 'career', title: 'home.widget.collection', blurb: 'home.widget.collection.blurb', load: () => import('./widgets/CollectionWidget') },
  { id: 'masteries', sizes: ['s', 'm'], needs: 'career', title: 'home.widget.masteries', blurb: 'home.widget.masteries.blurb', load: () => import('./widgets/MasteriesWidget') },
  { id: 'streak', sizes: ['m', 'w'], needs: 'career', title: 'home.widget.streak', blurb: 'home.widget.streak.blurb', load: () => import('./widgets/StreakWidget') },
  { id: 'savedGames', sizes: ['m', 'w'], title: 'home.widget.savedGames', blurb: 'home.widget.savedGames.blurb', load: () => import('./widgets/SavedGamesWidget') },
  { id: 'leaderboard', sizes: ['m', 'l'], needs: 'career', settings: { board: LEADERBOARDS }, title: 'home.widget.leaderboard', blurb: 'home.widget.leaderboard.blurb', load: () => import('./widgets/LeaderboardWidget') },
  { id: 'capsules', sizes: ['s'], needs: 'career', title: 'home.widget.capsules', blurb: 'home.widget.capsules.blurb', load: () => import('./widgets/CapsulesWidget') },
  { id: 'notice', sizes: ['m', 'w'], title: 'home.widget.notice', blurb: 'home.widget.notice.blurb', load: () => import('./widgets/NoticeWidget') },
];

const BY_ID = new Map(WIDGETS.map((widget) => [widget.id, widget]));

export function widget(id: string): WidgetDef | undefined {
  return BY_ID.get(id);
}
