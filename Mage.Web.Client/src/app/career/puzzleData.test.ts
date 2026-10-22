import { describe, expect, test } from 'vitest';
import { clock, parsePuzzleFile, puzzleFileName } from './puzzleData';

describe('puzzle files', () => {
  test('are named after the puzzle', () => {
    expect(puzzleFileName('Lethal Through the Wall!')).toBe('lethal-through-the-wall.json');
    expect(puzzleFileName('Él último turno')).toBe('el-ultimo-turno.json');
    expect(puzzleFileName('  ')).toBe('puzzle.json');
    expect(puzzleFileName(undefined)).toBe('puzzle.json');
  });

  test('read as a JSON object, anything else refused', () => {
    expect(parsePuzzleFile('{"name":"Ten to go"}').name).toBe('Ten to go');
    expect(() => parsePuzzleFile('[1, 2]')).toThrow();
    expect(() => parsePuzzleFile('null')).toThrow();
    expect(() => parsePuzzleFile('not json')).toThrow();
  });
});

describe('solve times', () => {
  test('read as m:ss, h:mm:ss past an hour', () => {
    expect(clock(7)).toBe('0:07');
    expect(clock(65)).toBe('1:05');
    expect(clock(3_725)).toBe('1:02:05');
    expect(clock(-3)).toBe('0:00');
  });
});
