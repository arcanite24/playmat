import { Component, Fragment, type ErrorInfo, type ReactNode } from 'react';
import { reportError } from '../../core/telemetry/reporter';
import { Button } from '../ui/Button';
import { Stitch } from '../ui/Stitch';
import { t } from '../i18n';
import './matchMessages';
import styles from './MatchErrorBoundary.module.css';

interface Props {
  gameId: string;
  /** where "back" goes (Play, or the event the game belongs to) */
  backLabel: string;
  onBack(): void;
  children: ReactNode;
}

interface State {
  error: Error | null;
  /** bumped by "Reload board" so the stage mounts fresh */
  attempt: number;
}

/**
 * Keeps a crash in the board from taking the whole game screen with it. The game goes on at the server and the
 * session keeps receiving updates, so redrawing the board usually recovers.
 */
export class MatchErrorBoundary extends Component<Props, State> {
  state: State = { error: null, attempt: 0 };

  static getDerivedStateFromError(error: unknown): Partial<State> {
    return { error: error instanceof Error ? error : new Error(String(error)) };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    reportError(error, 'react', { boundary: 'match', gameId: this.props.gameId, componentStack: info.componentStack });
  }

  private reload = () => this.setState((state) => ({ error: null, attempt: state.attempt + 1 }));

  render() {
    const { error, attempt } = this.state;
    if (!error) return <Fragment key={attempt}>{this.props.children}</Fragment>;
    return (
      <div className={styles.fallback} role="alert">
        <Stitch inset={10} radius={22} />
        <div className={styles.panel}>
          <h1 className={styles.title}>{t('match.crash.title')}</h1>
          <p className={styles.body}>{t('match.crash.text')}</p>
          <p className={styles.detail}><code>{error.message}</code></p>
          <div className={styles.actions}>
            <Button variant="decision" size="lg" onClick={this.reload}>{t('match.crash.reload')}</Button>
            <Button variant="print" size="lg" onClick={this.props.onBack}>{this.props.backLabel}</Button>
          </div>
        </div>
      </div>
    );
  }
}
