export function asciiBoardToFen(board: string[]): string {
  const rows: string[] = [];
  for (let i = 7; i >= 0; i--) {
    let fenRow = '';
    let emptyCount = 0;
    for (let j = 0; j < 8; j++) {
      const ch = board[i][j];
      if (ch === ' ' || ch === undefined) {
        emptyCount++;
      } else {
        if (emptyCount > 0) {
          fenRow += emptyCount;
          emptyCount = 0;
        }
        fenRow += ch;
      }
    }
    if (emptyCount > 0) {
      fenRow += emptyCount;
    }
    rows.push(fenRow);
  }
  return rows.join('/');
}
