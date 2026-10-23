import { act, fireEvent, render } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';
import { loadTestCatalog } from '../../test/catalog';
import { useSettings } from '../stores/settings';
import { setCatalog } from '../ui/cosmeticsCatalog';
import { CardFace } from '../ui/CardFace';
import { PlayerPlate } from './PlayerPlate';
import { MatPrint } from './MatPrint';
import { lookCloth } from './touches';
import { playerSleeve } from '../support/supportModel';

vi.mock('../ui/MatPrint', () => ({ MatPrint: () => <div data-testid="deck-print" /> }));

beforeEach(() => {
  loadTestCatalog();
  useSettings.getState().update({ paintedArt: true });
});

function plate(isMe: boolean, look = { avatar: 'owl', frame: 'frame-bronze' }) {
  return <PlayerPlate player={{ playerId: isMe ? 'me' : 'opponent', name: isMe ? 'Alice' : 'Bob' }} isMe={isMe} look={look} targetable={false} selected={false} deciding={false} onClick={() => undefined} />;
}

describe('battlefield cosmetics', () => {
  test.each([true, false])('painted avatar and frame render for isMe=%s and fall back on image errors', (isMe) => {
    const { container } = render(plate(isMe));
    const avatar = container.querySelector('img[src="/career-art/avatar/owl.webp"]')!;
    const frame = container.querySelector('img[src="/career-art/plate-frame/frame-bronze.webp"]')!;
    expect(avatar).toBeTruthy();
    expect(frame).toBeTruthy();
    fireEvent.error(avatar);
    fireEvent.error(frame);
    expect(container.querySelectorAll('img').length).toBe(0);
    expect(container.querySelector('[data-crest] svg path')).toBeTruthy();
    expect(container.querySelector('[data-frame] > svg')).toBeTruthy();
  });

  test('Painted art off retains the crest and frame for both seats', () => {
    useSettings.getState().update({ paintedArt: false });
    const { container } = render(<>{plate(true)}{plate(false)}</>);
    expect(container.querySelectorAll('img').length).toBe(0);
    expect(container.querySelectorAll('[data-crest] svg path').length).toBe(2);
    expect(container.querySelectorAll('[data-frame] > svg').length).toBe(2);
  });

  test('empty and unknown cosmetics keep the initial and do not draw a broken frame', () => {
    const { container, rerender } = render(plate(true, { avatar: '', frame: '' }));
    expect(container.textContent).toContain('A');
    rerender(plate(true, { avatar: 'unknown', frame: 'unknown' }));
    expect(container.querySelector('img')).toBeNull();
    expect(container.querySelector('[data-frame]')).toBeNull();
  });

  test('a catalog arriving after the look draws the avatar and frame', () => {
    setCatalog([]);
    const { container } = render(plate(false));
    expect(container.querySelector('img')).toBeNull();
    act(() => { loadTestCatalog(); });
    expect(container.querySelectorAll('img').length).toBe(2);
  });

  test('equipped playmats use their art, with the deck print as the missing-image and art-off fallback', () => {
    const cloth = lookCloth({ cloth: 'sand' });
    expect(cloth?.career).toBe(true);
    const { container, getByTestId, rerender } = render(<MatPrint cloth={cloth} card={null} enabled />);
    const image = container.querySelector('img')!;
    expect(image.getAttribute('src')).toBe('/career-art/playmat/sand.webp');
    fireEvent.error(image);
    expect(getByTestId('deck-print')).toBeTruthy();
    act(() => useSettings.getState().update({ paintedArt: false }));
    rerender(<MatPrint cloth={lookCloth({ cloth: 'navy' })} card={null} enabled />);
    expect(container.querySelector('img')).toBeNull();
    expect(getByTestId('deck-print')).toBeTruthy();
    rerender(<MatPrint key="art-off" cloth={cloth} card={null} enabled />);
    expect(container.querySelector('img')).toBeNull();
  });

  test('mat art none hides Career paintings and deck prints, and can be switched back on', () => {
    const { container, rerender } = render(<MatPrint cloth={lookCloth({ cloth: 'sand' })} card={null} enabled={false} />);
    expect(container.childElementCount).toBe(0);
    rerender(<MatPrint cloth={lookCloth({ cloth: 'sand' })} card={null} enabled />);
    expect(container.querySelector('img')).toBeTruthy();
    rerender(<MatPrint cloth={lookCloth({ cloth: 'navy' })} card={{ name: 'Forest', setCode: 'M21', cardNumber: '274' }} enabled={false} />);
    expect(container.childElementCount).toBe(0);
  });

  test('equipped card backs map by sleeve id for both seats and fall back to the weave', () => {
    const sleeve = playerSleeve({ sleeve: 'copper' }, '#20302b');
    const { container, rerender } = render(<><CardFace card={{}} hidden sleeve={sleeve} /><CardFace card={{}} hidden sleeve={sleeve} /></>);
    const images = container.querySelectorAll('img[src="/career-art/sleeve/copper.webp"]');
    expect(images.length).toBe(2);
    images.forEach((image) => fireEvent.error(image));
    expect(container.querySelectorAll('[data-art]').length).toBe(0);
    act(() => useSettings.getState().update({ paintedArt: false }));
    rerender(<CardFace card={{}} hidden sleeve={sleeve} />);
    expect(container.querySelector('img')).toBeNull();
  });

  test('card backs refresh when their sleeve arrives in the catalog', () => {
    setCatalog([]);
    const { container } = render(<CardFace card={{}} hidden sleeve="#6e3f24" />);
    expect(container.querySelector('img')?.getAttribute('src')).not.toContain('/career-art/sleeve/');
    act(() => { loadTestCatalog(); });
    expect(container.querySelector('img')?.getAttribute('src')).toBe('/career-art/sleeve/copper.webp');
  });
});
