import { describe, expect, it } from 'vitest';

import { asciiBoardToFen } from '../fen.js';

const EMPTY_ROW = '        ';

const STARTING_BOARD = [
  'RNBQKBNR',
  'PPPPPPPP',
  EMPTY_ROW,
  EMPTY_ROW,
  EMPTY_ROW,
  EMPTY_ROW,
  'pppppppp',
  'rnbqkbnr'
];

describe('asciiBoardToFen', () => {
  it('converts starting position', () => {
    expect(asciiBoardToFen(STARTING_BOARD)).toBe(
      'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR'
    );
  });

  it('converts empty board', () => {
    const board = Array(8).fill(EMPTY_ROW);
    expect(asciiBoardToFen(board)).toBe('8/8/8/8/8/8/8/8');
  });

  it('converts board after e4', () => {
    const board = [...STARTING_BOARD.map(row => row)];
    board[1] = 'PPPP PPP';
    board[3] = '    P   ';
    expect(asciiBoardToFen(board)).toBe(
      'rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR'
    );
  });

  it('converts board with single piece on e4', () => {
    const board = Array(8).fill(EMPTY_ROW) as string[];
    board[3] = '    P   ';
    expect(asciiBoardToFen(board)).toBe('8/8/8/8/4P3/8/8/8');
  });

  it('converts board with consecutive empty squares', () => {
    const board = Array(8).fill(EMPTY_ROW) as string[];
    board[7] = 'r   k  r';
    expect(asciiBoardToFen(board)).toBe('r3k2r/8/8/8/8/8/8/8');
  });
});
