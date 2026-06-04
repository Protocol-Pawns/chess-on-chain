export const PIECE_UNICODE: Record<string, string> = {
	K: '\u2654', Q: '\u2655', R: '\u2656', B: '\u2657', N: '\u2658', P: '\u2659',
	k: '\u265A', q: '\u265B', r: '\u265C', b: '\u265D', n: '\u265E', p: '\u265F'
};

export interface BoardPosition {
	row: number;
	col: number;
}

export interface Square {
	piece: string | null;
	isLight: boolean;
	position: BoardPosition;
}

export function parseBoard(board: string[]): Square[][] {
	return board.map((row, r) =>
		[...row].map((piece, c) => ({
			piece: piece === ' ' ? null : piece,
			isLight: (r + c) % 2 === 0,
			position: { row: r, col: c }
		}))
	);
}

export function parseFEN(fen: string): Square[][] {
	const placement = fen.split(' ')[0];
	const rows = placement.split('/');
	const board: Square[][] = [];
	for (let r = 0; r < rows.length; r++) {
		const row: Square[] = [];
		let c = 0;
		for (const ch of rows[r]) {
			if (/\d/.test(ch)) {
				for (let i = 0; i < parseInt(ch); i++) {
					row.push({ piece: null, isLight: (r + c) % 2 === 0, position: { row: r, col: c } });
					c++;
				}
			} else {
				row.push({ piece: ch, isLight: (r + c) % 2 === 0, position: { row: r, col: c } });
				c++;
			}
		}
		board.push(row);
	}
	return board;
}

export function boardFromInput(board?: string[], fen?: string): Square[][] {
	if (fen) return parseFEN(fen);
	if (board) return parseBoard(board);
	return parseBoard([
		'RNBQKBNR', 'PPPPPPPP', '        ', '        ',
		'        ', '        ', 'pppppppp', 'rnbqkbnr'
	]);
}

export function posToAlgebraic(row: number, col: number): string {
	return String.fromCharCode(97 + col) + (8 - row);
}

export function algebraicToPos(sq: string): BoardPosition {
	return { col: sq.charCodeAt(0) - 97, row: 8 - parseInt(sq[1]) };
}

export function isOwnPiece(piece: string | null, color: 'White' | 'Black'): boolean {
	if (!piece) return false;
	return color === 'White' ? piece === piece.toUpperCase() : piece === piece.toLowerCase();
}

export function isOpponentPiece(piece: string | null, color: 'White' | 'Black'): boolean {
	if (!piece) return false;
	return color === 'White' ? piece === piece.toLowerCase() : piece === piece.toUpperCase();
}

export function colorFromFEN(fen: string): 'White' | 'Black' {
	return fen.split(' ')[1] === 'w' ? 'White' : 'Black';
}
