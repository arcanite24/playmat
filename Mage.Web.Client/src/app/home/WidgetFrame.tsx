import { Component, createElement, lazy, Suspense, type ComponentType, type ErrorInfo, type LazyExoticComponent, type ReactNode } from 'react';
import { useT } from '../i18n';
import type { LayoutItem } from './layoutModel';
import { widget, WIDGETS, type WidgetProps } from './widgets';
import styles from './HomeBoard.module.css';

// one lazy chunk per widget, made once: a widget loads the first time it is on the board
const COMPONENTS = new Map<string, LazyExoticComponent<ComponentType<WidgetProps>>>(WIDGETS.map((def) => [def.id, lazy(def.load)]));

class Boundary extends Component<{ fallback: ReactNode; children: ReactNode }, { failed: boolean }> {
  state = { failed: false };

  static getDerivedStateFromError(): { failed: boolean } {
    return { failed: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.warn('Home widget failed', error, info.componentStack);
  }

  render(): ReactNode {
    return this.state.failed ? this.props.fallback : this.props.children;
  }
}

/** One widget on the board: its own lazy chunk, and a quiet tile instead of a broken Home when it fails. */
export function WidgetFrame({ item, editing }: { item: LayoutItem; editing: boolean }) {
  const t = useT();
  // looked up, not made, here: each widget's lazy component is created once, above
  const Widget = COMPONENTS.get(item.id);
  const def = widget(item.id);
  const quiet = (text: string) => (
    <section className={styles.quiet} aria-label={def ? t(def.title) : item.id}>
      <p>{text}</p>
    </section>
  );
  if (!Widget) return quiet(t('home.board.unknown'));
  return (
    <Boundary fallback={quiet(t('home.board.failed'))}>
      <Suspense fallback={<div className={styles.loading} aria-hidden="true" />}>
        {createElement(Widget, { item, editing })}
      </Suspense>
    </Boundary>
  );
}
