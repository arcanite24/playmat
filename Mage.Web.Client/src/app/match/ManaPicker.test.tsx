import { render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { parseManaAbility } from '../../core/game/manaChoices';
import { ManaPicker } from './ManaPicker';
import { DEFAULT_STAGE, StageContext } from './stageContext';
import './matchMessages';

afterEach(() => vi.restoreAllMocks());

test.each([
  [400, '524px'],
  [50, '336px'],
])('anchors beside a card at y=%s in a scaled stage and stays within its edges', (top, expectedTop) => {
  const stage = document.createElement('div');
  const card = document.createElement('button');
  card.dataset.objectId = 'land';
  stage.append(card);
  vi.spyOn(stage, 'getBoundingClientRect').mockReturnValue(new DOMRect(50, 30, 960, 540));
  vi.spyOn(card, 'getBoundingClientRect').mockReturnValue(new DOMRect(900, top, 100, 140));
  vi.spyOn(HTMLElement.prototype, 'offsetWidth', 'get').mockReturnValue(400);
  vi.spyOn(HTMLElement.prototype, 'offsetHeight', 'get').mockReturnValue(200);
  const { rerender } = render(
    <StageContext.Provider value={{ ...DEFAULT_STAGE, element: stage, scale: 0.5 }}>
      <ManaPicker title="Choose mana" sourceId="land" choices={[parseManaAbility('{T}: Add {U}.')!]} onPick={vi.fn()} />
    </StageContext.Provider>,
  );
  const picker = screen.getByRole('dialog', { name: 'Choose mana' });
  expect(picker.style.left).toBe('1504px');
  expect(picker.style.top).toBe(expectedTop);
  rerender(
    <StageContext.Provider value={{ ...DEFAULT_STAGE, element: stage, scale: 0.5 }}>
      <ManaPicker title="Choose mana" sourceId="missing" choices={[parseManaAbility('{T}: Add {U}.')!]} onPick={vi.fn()} />
    </StageContext.Provider>,
  );
  expect(picker.style.left).toBe('');
  expect(picker.style.top).toBe('');
});

test('returns keyboard focus to the source when the picker closes', () => {
  const { rerender } = render(<button>Land</button>);
  const source = screen.getByRole('button', { name: 'Land' });
  source.focus();
  rerender(<>
    <button>Land</button>
    <ManaPicker title="Choose mana" choices={[parseManaAbility('{T}: Add {W}.')!]} onPick={vi.fn()} />
  </>);
  expect(document.activeElement).toBe(screen.getByRole('dialog', { name: 'Choose mana' }));
  rerender(<button>Land</button>);
  expect(document.activeElement).toBe(source);
});
