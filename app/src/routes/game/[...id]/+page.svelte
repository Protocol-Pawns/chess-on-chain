<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { Chess } from 'chess.js';
  import {
    api,
    type GameMove,
    type Bet,
    type GameOverview
  } from '$lib/api/client';
  import type { PageData } from './$types';
  import { contract, getTxLogs } from '$lib/near/connector';
  import type { Difficulty } from '$lib/near/contract-types';
  import { accountStore } from '$lib/near/account';
  import { colorFromFEN } from '$lib/chess/board';
  import { showToast } from '$lib/toast';
  import { truncateAddr } from '$lib/format';
  import { loadGameFromContract, boardToFen, parseGamePath } from '$lib/game';
  import { renderGameCard } from '$lib/chess/render-card';
  import type { GameId, ContractGameData } from '$lib/game';
  import type { SSEEventData } from '$lib/sse';
  import {
    subscribe,
    updateWatermark,
    onReconnect,
    connectSSE
  } from '$lib/sse';
  import { get } from 'svelte/store';
  import Board from '$lib/components/Board.svelte';
  import CapturedPieces from '$lib/components/CapturedPieces.svelte';
  import MoveHistory from '$lib/components/MoveHistory.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import ConfirmModal from '$lib/components/ConfirmModal.svelte';
  import { getCapturedPieces } from '$lib/chess/captured';

  let { data } = $props<{ data: PageData }>();

  import dayjs from 'dayjs';

  function gameUrlPath(gameId: GameId): string {
    const [num, p1, p2] = gameId;
    return p2 ? `/game/${num}/${p1}/${p2}` : `/game/${num}/${p1}`;
  }

  function parseMoveNotation(
    notation: string,
    color?: string
  ): { from: string; to: string } | null {
    if (/^O-O-O$/i.test(notation) || /^0-0-0$/.test(notation)) {
      const rank = color === 'Black' ? '8' : '1';
      return { from: `e${rank}`, to: `c${rank}` };
    }
    if (/^O-O$/i.test(notation) || /^0-0$/.test(notation)) {
      const rank = color === 'Black' ? '8' : '1';
      return { from: `e${rank}`, to: `g${rank}` };
    }
    const parts = notation.split(' to ');
    if (parts.length >= 2) {
      const from = parts[0].trim();
      const to = parts[1].trim().split(/\s/)[0];
      if (/^[a-h][1-8]$/.test(from) && /^[a-h][1-8]$/.test(to)) {
        return { from, to };
      }
    }
    return null;
  }

  let game = $state<GameOverview | null>(null);
  let moves = $state<GameMove[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    game = data.game ?? null;
    loading = !data.game;
  });
  let waiting = $state(false);
  let submitting = $state(false);
  let indexed = $state(false);
  let gameBets = $state<Bet[]>([]);
  let showResignModal = $state(false);
  let showCancelModal = $state(false);
  let showPublicCancelModal = $state(false);
  let pendingLastMove = $state<{ from: string; to: string } | null>(null);
  let viewingMoveIndex = $state<number | null>(null);
  let appliedMoveSigs = $state<Set<string>>(new Set());

  let showShareModal = $state(false);
  let shareImageBlob = $state<Blob | null>(null);
  let copyingImage = $state(false);
  let wasInProgress = $state(false);
  let eloMap = $state<Record<string, number>>({});

  $effect(() => {
    if (game?.status === 'in_progress') {
      wasInProgress = true;
    }
    if (
      wasInProgress &&
      game?.status === 'finished' &&
      game?.outcome &&
      !showShareModal
    ) {
      setTimeout(() => {
        showShareModal = true;
        generateShareBlob();
      }, 600);
      wasInProgress = false;
    }
  });

  function moveSig(color: string, mv: string): string {
    return `${color}:${mv}`;
  }
  const STARTING_FEN =
    'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';

  const gameId: GameId = parseGamePath(page.params.id ?? '');
  const gameIdStr = JSON.stringify(gameId);

  let lastMove = $derived(
    pendingLastMove ??
      (moves.length > 0
        ? parseMoveNotation(
            moves[moves.length - 1].move_notation,
            moves[moves.length - 1].color
          )
        : null)
  );

  let isViewingCurrent = $derived(
    viewingMoveIndex === null || viewingMoveIndex === moves.length - 1
  );

  let displayFen = $derived.by(() => {
    if (isViewingCurrent) return game?.fen ?? undefined;
    if (viewingMoveIndex === -1) return STARTING_FEN;
    if (
      viewingMoveIndex != null &&
      viewingMoveIndex >= 0 &&
      viewingMoveIndex < moves.length
    )
      return moves[viewingMoveIndex].fen;
    return game?.fen ?? undefined;
  });

  let displayBoard = $derived.by(() => {
    if (isViewingCurrent) return game?.board;
    return undefined;
  });

  let displayLastMove = $derived.by(() => {
    if (isViewingCurrent) return lastMove;
    if (viewingMoveIndex === -1) return null;
    if (
      viewingMoveIndex != null &&
      viewingMoveIndex >= 0 &&
      viewingMoveIndex < moves.length
    ) {
      return parseMoveNotation(
        moves[viewingMoveIndex].move_notation,
        moves[viewingMoveIndex].color
      );
    }
    return null;
  });

  let captured = $derived.by(() => {
    const src = displayFen ?? displayBoard;
    if (!src) return { whiteCaptured: [], blackCaptured: [] };
    return getCapturedPieces(src);
  });

  function selectMove(index: number | null) {
    viewingMoveIndex = index;
  }

  let isMyTurn = $derived.by(() => {
    if (!game || game.status !== 'in_progress' || !$accountStore) return false;
    const turn = game.fen ? colorFromFEN(game.fen) : 'White';
    const myColor =
      game.white.value === $accountStore
        ? 'White'
        : game.black?.value === $accountStore
          ? 'Black'
          : null;
    return turn === myColor;
  });

  let contractTurnColor = $state<string | null>(null);

  let currentTurn = $derived(
    game?.status === 'in_progress'
      ? game.fen
        ? colorFromFEN(game.fen)
        : contractTurnColor
      : null
  );

  let flipped = $derived(
    !!(game && $accountStore && game.black?.value === $accountStore)
  );

  let topCaptured = $derived(
    flipped ? captured.whiteCaptured : captured.blackCaptured
  );
  let bottomCaptured = $derived(
    flipped ? captured.blackCaptured : captured.whiteCaptured
  );

  let isInCheck = $derived.by(() => {
    if (!displayFen || game?.status !== 'in_progress') return false;
    try {
      const c = new Chess(displayFen);
      return c.inCheck();
    } catch {
      return false;
    }
  });

  let canResign = $derived(
    game?.status === 'in_progress' &&
      $accountStore &&
      (game.white.value === $accountStore ||
        game.black?.value === $accountStore)
  );
  let canCancel = $derived(
    game?.status === 'waiting' &&
      $accountStore &&
      (game.white.value === $accountStore ||
        game.black?.value === $accountStore)
  );
  let isSpectating = $derived(
    $accountStore &&
      game &&
      game.white.value !== $accountStore &&
      game.black?.value !== $accountStore
  );
  let canPublicCancel = $derived(
    game?.status === 'in_progress' &&
      $accountStore &&
      isSpectating &&
      game.created_at &&
      dayjs().diff(dayjs(game.created_at), 'day') >= 14
  );

  async function loadFromContract(): Promise<ContractGameData> {
    return loadGameFromContract(gameId);
  }

  let localMoveCount = $state(0);
  async function load() {
    try {
      const [g, m] = await Promise.all([
        api.game(gameIdStr),
        api.gameMoves(gameIdStr)
      ]);
      error = null;
      waiting = false;
      indexed = true;
      if (game && game.status !== 'in_progress' && game.status !== g.status) {
        return;
      }
      if (m.length < localMoveCount && game?.status === 'in_progress') {
        return;
      }
      const localAhead = localMoveCount > m.length;
      if (localAhead) {
        const { board: localBoard, fen: localFen } = game ?? {};
        game = {
          ...g,
          board: localBoard ?? g.board,
          fen: localFen ?? g.fen
        };
      } else {
        game = g;
      }
      moves = m;
      localMoveCount = Math.max(localMoveCount, m.length);
      if (!localAhead && m.length > 0) pendingLastMove = null;
      contractTurnColor = null;
      const sigs = new Set<string>();
      for (const mv of m) sigs.add(moveSig(mv.color, mv.move_notation));
      appliedMoveSigs = sigs;
      if (g.status === 'finished' || g.status === 'cancelled') {
        stopPolling();
      }
    } catch (e) {
      if (game?.status !== 'in_progress' && game?.status !== undefined) return;

      console.warn('[game] API load failed, falling back to contract:', e);
      try {
        const contractGame = await loadFromContract();
        game = { ...contractGame } as GameOverview;
        contractTurnColor = contractGame.turn_color;
        moves = [];
        localMoveCount = 0;
        gameBets = [];
        waiting = false;
        indexed = false;
      } catch (e2) {
        console.warn('[game] contract fallback also failed, will retry:', e2);
        if (!game) waiting = true;
      }
    } finally {
      loading = false;
    }
  }

  async function loadElos() {
    if (!game) return;
    const ids: string[] = [];
    if (game.white.type === 'Human') ids.push(game.white.value);
    if (game.black?.type === 'Human' && game.black.value)
      ids.push(game.black.value);
    if (ids.length === 0) return;
    try {
      const ratings = await contract.getEloRatingsByIds(ids);
      const map: Record<string, number> = {};
      for (const [id, elo] of ratings) map[id] = elo;
      eloMap = map;
    } catch {
      // ELOs are non-critical
    }
  }

  function parseTxLogs(logs: string[]) {
    const parsedMoves: {
      from: string;
      to: string;
      promotion?: string;
      color: string;
      mv: string;
    }[] = [];
    let outcome: Record<string, unknown> | null = null;
    let board: string[] | null = null;
    let resigner: string | null = null;
    let cancelled = false;
    for (const log of logs) {
      try {
        const json = log.startsWith('EVENT_JSON:') ? log.slice(11) : log;
        const event = JSON.parse(json);
        if (event.standard !== 'chess-game') continue;
        if (event.event === 'play_move') {
          const parts: string[] = event.data.mv.split(' to ');
          if (parts.length === 2) {
            const toParts = parts[1].trim().split(/\s+/);
            parsedMoves.push({
              from: parts[0],
              to: toParts[0],
              promotion: toParts.length > 1 ? toParts[1] : undefined,
              color: event.data.color,
              mv: event.data.mv
            });
          }
          if (event.data.mv === 'O-O' || event.data.mv === 'O-O-O') {
            const color = event.data.color as string;
            const isKingside = event.data.mv === 'O-O';
            const from = color === 'White' ? 'e1' : 'e8';
            const to = isKingside
              ? color === 'White'
                ? 'g1'
                : 'g8'
              : color === 'White'
                ? 'c1'
                : 'c8';
            parsedMoves.push({ from, to, color, mv: event.data.mv });
          }
          if (event.data.board) {
            board = event.data.board as string[];
          }
          if (event.data.outcome) {
            outcome = event.data.outcome as Record<string, unknown>;
          }
        } else if (event.event === 'resign_game') {
          if (event.data.outcome) {
            outcome = event.data.outcome as Record<string, unknown>;
          }
          if (event.data.resigner) {
            resigner = event.data.resigner as string;
          }
        } else if (event.event === 'cancel_game') {
          cancelled = true;
        }
      } catch {
        // skip
      }
    }
    const moves = parsedMoves.map(m => ({ from: m.from, to: m.to }));
    return { moves, outcome, board, resigner, cancelled, parsedMoves };
  }

  function handleMove(from: string, to: string, promotion?: string) {
    if (!game || submitting || !indexed) return;
    submitting = true;
    const isAiGame = game.white.type === 'Ai' || game.black?.type === 'Ai';
    const aiDifficulty: Difficulty | undefined =
      game.white.type === 'Ai'
        ? (game.white.value as Difficulty)
        : game.black?.type === 'Ai'
          ? (game.black.value as Difficulty)
          : undefined;
    const isCastling =
      (from === 'e1' && to === 'g1') ||
      (from === 'e8' && to === 'g8') ||
      (from === 'e1' && to === 'c1') ||
      (from === 'e8' && to === 'c8');
    let moveStr: string;
    if (isCastling) {
      moveStr = to === 'g1' || to === 'g8' ? 'O-O' : 'O-O-O';
    } else if (promotion) {
      moveStr = `${from} to ${to} ${promotion}`;
    } else {
      moveStr = from + to;
    }
    contract
      .playMove($state.snapshot(game.game_id), moveStr, aiDifficulty)
      .then(async txResult => {
        submitting = false;
        const tx = txResult as {
          receipts_outcome?: { outcome: { logs: string[] } }[];
          transaction?: { hash?: string };
          transaction_outcome?: { id?: string };
        };
        const txLogs: string[] = [];
        if (tx.receipts_outcome) {
          for (const r of tx.receipts_outcome) {
            txLogs.push(...(r.outcome?.logs ?? []));
          }
        }
        let parsed = parseTxLogs(txLogs);
        const pMap: Record<string, string> = {
          queen: 'q',
          rook: 'r',
          bishop: 'b',
          knight: 'n'
        };

        if (game) {
          for (const m of parsed.parsedMoves) {
            appliedMoveSigs.add(moveSig(m.color, m.mv));
          }
          localMoveCount += parsed.parsedMoves.length;
          const preMoveFen = game.fen;
          if (parsed.outcome && parsed.board) {
            game = {
              ...game,
              board: parsed.board,
              fen: undefined,
              status: 'finished' as const,
              outcome: parsed.outcome as GameOverview['outcome']
            };
            pendingLastMove = null;
            contractTurnColor = null;
          } else {
            const aiMove =
              isAiGame && parsed.moves.length > 1
                ? parsed.moves[parsed.moves.length - 1]
                : null;
            const aiPromotion =
              isAiGame && parsed.parsedMoves.length > 1
                ? parsed.parsedMoves[parsed.parsedMoves.length - 1].promotion
                : undefined;
            pendingLastMove = aiMove ?? { from, to };
            if (game.fen) {
              try {
                const c = new Chess(game.fen);
                c.move({
                  from,
                  to,
                  promotion: promotion ? pMap[promotion] : 'q'
                });
                if (aiMove) {
                  c.move({
                    from: aiMove.from,
                    to: aiMove.to,
                    promotion: aiPromotion ? pMap[aiPromotion] : 'q'
                  });
                }
                game = { ...game, fen: c.fen() };
              } catch {
                if (parsed.board) {
                  const lastMove =
                    parsed.parsedMoves[parsed.parsedMoves.length - 1];
                  const nextTurn = lastMove
                    ? lastMove.color === 'White'
                      ? 'Black'
                      : 'White'
                    : game.fen
                      ? colorFromFEN(game.fen) === 'White'
                        ? 'Black'
                        : 'White'
                      : 'White';
                  game = {
                    ...game,
                    board: parsed.board,
                    fen: boardToFen(parsed.board, nextTurn)
                  };
                }
              }
            } else if (parsed.board) {
              const lastMove =
                parsed.parsedMoves[parsed.parsedMoves.length - 1];
              const nextTurn = lastMove
                ? lastMove.color === 'White'
                  ? 'Black'
                  : 'White'
                : game.fen
                  ? colorFromFEN(game.fen) === 'White'
                    ? 'Black'
                    : 'White'
                  : 'White';
              game = {
                ...game,
                board: parsed.board,
                fen: boardToFen(parsed.board, nextTurn)
              };
            }
          }
          const newMoves: GameMove[] = [];
          let runningFen = preMoveFen;
          for (let i = 0; i < parsed.parsedMoves.length; i++) {
            const pm = parsed.parsedMoves[i];
            let moveFen: string;
            if (runningFen) {
              try {
                const c = new Chess(runningFen);
                c.move({
                  from: pm.from,
                  to: pm.to,
                  promotion: pm.promotion ? pMap[pm.promotion] : undefined
                });
                moveFen = c.fen();
                runningFen = moveFen;
              } catch {
                moveFen =
                  game.fen ??
                  (parsed.board
                    ? boardToFen(
                        parsed.board,
                        pm.color === 'White' ? 'Black' : 'White'
                      )
                    : STARTING_FEN);
              }
            } else {
              moveFen =
                game.fen ??
                (parsed.board
                  ? boardToFen(
                      parsed.board,
                      pm.color === 'White' ? 'Black' : 'White'
                    )
                  : STARTING_FEN);
            }
            newMoves.push({
              move_number: moves.length + i + 1,
              color: pm.color,
              move_notation: pm.mv,
              fen: moveFen,
              outcome: null
            });
          }
          if (parsed.outcome && newMoves.length > 0) {
            newMoves[newMoves.length - 1].outcome =
              parsed.outcome as GameMove['outcome'];
          }
          moves = [...moves, ...newMoves];
        }
      })
      .catch((err: unknown) => {
        submitting = false;
        pendingLastMove = null;
        showToast(
          'error',
          'Move failed',
          err instanceof Error ? err.message : String(err)
        );
      });
  }

  function handleResign() {
    showResignModal = true;
  }

  function confirmResign() {
    if (!game) return;
    showResignModal = false;
    submitting = true;
    showToast('info', 'Resigning...');
    contract
      .resign($state.snapshot(game.game_id))
      .then(async txResult => {
        const tx = txResult as {
          receipts_outcome?: { outcome: { logs: string[] } }[];
        };
        const txLogs: string[] = [];
        if (tx.receipts_outcome) {
          for (const r of tx.receipts_outcome) {
            txLogs.push(...(r.outcome?.logs ?? []));
          }
        }
        const parsed = parseTxLogs(txLogs);
        if (parsed.outcome && game) {
          game = {
            ...game,
            status: 'finished' as const,
            outcome: parsed.outcome as GameOverview['outcome'],
            resigner: (parsed.resigner as GameOverview['resigner']) ?? null
          };
          showToast('success', 'Game resigned');
        } else {
          showToast('success', 'Game resigned');
        }
      })
      .catch((err: unknown) => {
        showToast(
          'error',
          'Resign failed',
          err instanceof Error ? err.message : String(err)
        );
      })
      .finally(() => {
        submitting = false;
      });
  }

  function handleCancel() {
    showCancelModal = true;
  }

  function confirmCancel() {
    if (!game) return;
    showCancelModal = false;
    submitting = true;
    showToast('info', 'Cancelling...');
    contract
      .cancel($state.snapshot(game.game_id))
      .then(async txResult => {
        const tx = txResult as {
          receipts_outcome?: { outcome: { logs: string[] } }[];
        };
        const txLogs: string[] = [];
        if (tx.receipts_outcome) {
          for (const r of tx.receipts_outcome) {
            txLogs.push(...(r.outcome?.logs ?? []));
          }
        }
        const parsed = parseTxLogs(txLogs);
        if (parsed.cancelled && game) {
          game = {
            ...game,
            status: 'cancelled' as const
          };
          showToast('success', 'Game cancelled');
        } else {
          showToast('success', 'Game cancelled');
        }
      })
      .catch((err: unknown) => {
        showToast(
          'error',
          'Cancel failed',
          err instanceof Error ? err.message : String(err)
        );
      })
      .finally(() => {
        submitting = false;
      });
  }

  function handlePublicCancel() {
    showPublicCancelModal = true;
  }

  function confirmPublicCancel() {
    if (!game) return;
    showPublicCancelModal = false;
    submitting = true;
    showToast('info', 'Cancelling stale game...');
    contract
      .cancel($state.snapshot(game.game_id))
      .then(async txResult => {
        const tx = txResult as {
          receipts_outcome?: { outcome: { logs: string[] } }[];
        };
        const txLogs: string[] = [];
        if (tx.receipts_outcome) {
          for (const r of tx.receipts_outcome) {
            txLogs.push(...(r.outcome?.logs ?? []));
          }
        }
        const parsed = parseTxLogs(txLogs);
        if (parsed.cancelled && game) {
          game = {
            ...game,
            status: 'cancelled' as const
          };
          showToast('success', 'Stale game cancelled');
        } else {
          showToast('success', 'Game cancelled');
        }
      })
      .catch((err: unknown) => {
        const msg = err instanceof Error ? err.message : String(err);
        if (msg.includes('publicly cancellable')) {
          showToast('error', 'Not yet cancellable', msg);
        } else {
          showToast('error', 'Cancel failed', msg);
        }
      })
      .finally(() => {
        submitting = false;
      });
  }

  function openShareModal() {
    showShareModal = true;
    if (!shareImageBlob) generateShareBlob();
  }

  async function generateShareBlob() {
    if (!game) return;
    try {
      shareImageBlob = await renderGameCard({
        board: game.board,
        fen: game.fen,
        whiteName:
          game.white.type === 'Human'
            ? truncateAddr(game.white.value)
            : 'AI (' + game.white.value + ')',
        whiteElo:
          game.white.type === 'Human'
            ? (eloMap[game.white.value] ?? null)
            : null,
        blackName:
          game.black?.type === 'Human'
            ? truncateAddr(game.black.value ?? '')
            : game.black?.type === 'Ai'
              ? 'AI (' + game.black.value + ')'
              : 'Unknown',
        blackElo:
          game.black?.type === 'Human'
            ? (eloMap[game.black.value ?? ''] ?? null)
            : null,
        result: resultText(),
        lastMove: lastMove ?? undefined
      });
    } catch (e) {
      console.error('Failed to generate share image:', e);
    }
  }

  function resultText(): string {
    if (!game?.outcome) return '';
    if (game.outcome.result === 'Stalemate') return 'Draw \u2014 Stalemate';
    if (game.resigner) return game.outcome.color + ' wins by resignation!';
    return game.outcome.color + ' wins by checkmate!';
  }

  function shareOnX() {
    if (!game) return;
    const wn =
      game.white.type === 'Human'
        ? truncateAddr(game.white.value)
        : 'AI (' + game.white.value + ')';
    const bn =
      game.black?.type === 'Human'
        ? truncateAddr(game.black.value ?? '')
        : game.black?.type === 'Ai'
          ? 'AI (' + game.black.value + ')'
          : 'Unknown';
    const we =
      game.white.type === 'Human' ? (eloMap[game.white.value] ?? null) : null;
    const be =
      game.black?.type === 'Human'
        ? (eloMap[game.black.value ?? ''] ?? null)
        : null;

    const movesN = Math.ceil(moves.length / 2);
    const url = shareUrl();
    let text: string;

    if (game.outcome?.result === 'Stalemate') {
      const wText = wn + (we != null ? ' (' + we + ')' : '');
      const bText = bn + (be != null ? ' (' + be + ')' : '');
      text =
        'Stalemate after ' +
        movesN +
        ' moves between ' +
        wText +
        ' and ' +
        bText +
        ' on @ProtocolPawns. Replay here: ' +
        url;
    } else {
      const isWhite = game.outcome?.color === 'White';
      const winner = isWhite ? wn : bn;
      const loser = isWhite ? bn : wn;
      const wElo = isWhite ? we : be;
      const lElo = isWhite ? be : we;
      const wText = winner + (wElo != null ? ' (' + wElo + ')' : '');
      const lText = loser + (lElo != null ? ' (' + lElo + ')' : '');
      if (game.resigner) {
        text =
          lText +
          ' resigned after ' +
          movesN +
          ' moves on @ProtocolPawns \u2014 ' +
          wText +
          ' wins. Replay here: ' +
          url;
      } else {
        text =
          'Checkmate in ' +
          movesN +
          '! ' +
          wText +
          ' defeated ' +
          lText +
          ' on @ProtocolPawns. Replay here: ' +
          url;
      }
    }

    window.open(
      'https://x.com/intent/tweet?text=' + encodeURIComponent(text),
      'share-x',
      'width=550,height=420'
    );
  }

  async function copyImage() {
    if (!shareImageBlob) await generateShareBlob();
    if (!shareImageBlob) {
      showToast('error', 'Failed to generate image');
      return;
    }
    copyingImage = true;
    try {
      await navigator.clipboard.write([
        new ClipboardItem({ 'image/png': shareImageBlob })
      ]);
      showToast('success', 'Image copied to clipboard');
    } catch {
      showToast('error', 'Failed to copy image');
    } finally {
      copyingImage = false;
    }
  }

  function applySSEMove(data: Record<string, unknown>) {
    const board = data.board as string[] | undefined;
    if (!board || !game) return;
    const color = data.color as string;
    const nextTurn: 'White' | 'Black' = color === 'White' ? 'Black' : 'White';
    const fen = boardToFen(board, nextTurn);
    const outcome = data.outcome as { result: string; color?: string } | null;
    const mv = data.mv as string;

    moves = [
      ...moves,
      {
        move_number: moves.length + 1,
        color,
        move_notation: mv,
        fen,
        outcome: outcome ?? null
      }
    ];
    localMoveCount = moves.length;

    game = {
      ...game,
      board,
      fen,
      ...(outcome
        ? {
            status: 'finished' as const,
            outcome,
            ...(data.resigner ? { resigner: data.resigner as string } : {})
          }
        : {})
    };

    pendingLastMove = null;
    contractTurnColor = null;
  }

  function handleSSEPlayMove(event: SSEEventData) {
    const data = event.event_data;
    const eventGameId =
      typeof data.game_id === 'string'
        ? data.game_id
        : JSON.stringify(data.game_id);
    if (eventGameId !== gameIdStr) return;
    if (game && game.status !== 'in_progress') return;

    const sig = moveSig(data.color as string, data.mv as string);
    if (appliedMoveSigs.has(sig)) return;

    updateWatermark(event.trigger_block_height);
    appliedMoveSigs.add(sig);
    applySSEMove(data);
  }

  function handleSSEResignGame(event: SSEEventData) {
    const data = event.event_data;
    const eventGameId = gameIdFromData(data);
    if (eventGameId !== gameIdStr) return;
    if (!game || game.status !== 'in_progress') return;

    updateWatermark(event.trigger_block_height);
    const outcome = data.outcome as { result: string; color: string };
    game = {
      ...game,
      status: 'finished' as const,
      outcome,
      resigner: data.resigner as string
    };
    showToast('info', 'Opponent resigned!');
  }

  function handleSSECancelGame(event: SSEEventData) {
    const data = event.event_data;
    const eventGameId = gameIdFromData(data);
    if (eventGameId !== gameIdStr) return;
    if (!game || game.status === 'cancelled') return;

    updateWatermark(event.trigger_block_height);
    game = { ...game, status: 'cancelled' as const };
    showToast('info', 'Game was cancelled');
  }

  function handleSSECreateGame(event: SSEEventData) {
    const data = event.event_data;
    const eventGameId = gameIdFromData(data);
    if (eventGameId !== gameIdStr) return;

    updateWatermark(event.trigger_block_height);
    load().then(() => {
      api
        .gameBets(gameIdStr)
        .then(b => (gameBets = b))
        .catch(() => []);
      loadElos();
    });
  }

  function gameIdFromData(data: Record<string, unknown>): string {
    return typeof data.game_id === 'string'
      ? data.game_id
      : JSON.stringify(data.game_id);
  }

  function shareUrl(): string {
    return `${window.location.origin}${gameUrlPath(gameId)}`;
  }

  let pollInterval: ReturnType<typeof setInterval> | null = null;
  let fastPollRemaining = 0;

  function stopPolling() {
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
  }

  onMount(() => {
    const account = get(accountStore);
    if (account) connectSSE(account);

    const unsubAccount = accountStore.subscribe(a => {
      if (a) connectSSE(a);
    });

    load().then(() => {
      api
        .gameBets(gameIdStr)
        .then(b => (gameBets = b))
        .catch(() => []);
      loadElos();
    });

    fastPollRemaining = 6;
    pollInterval = setInterval(() => {
      if (game?.status === 'finished' || game?.status === 'cancelled') {
        stopPolling();
        return;
      }
      load();
      if (fastPollRemaining > 0) {
        fastPollRemaining--;
        if (fastPollRemaining === 0 && pollInterval) {
          clearInterval(pollInterval);
          pollInterval = setInterval(load, 5_000);
        }
      }
    }, 2_500);

    const unsubs = [
      subscribe('play_move', handleSSEPlayMove),
      subscribe('resign_game', handleSSEResignGame),
      subscribe('cancel_game', handleSSECancelGame),
      subscribe('create_game', handleSSECreateGame),
      onReconnect(() => load())
    ];

    return () => {
      stopPolling();
      unsubAccount();
      for (const u of unsubs) u();
    };
  });
</script>

{#if loading}
  <div class="flex flex-col gap-4 animate-pulse">
    <div class="card">
      <div class="flex justify-between items-center mb-2">
        <div class="h-4 w-24 rounded bg-white/10"></div>
        <div class="h-5 w-16 rounded bg-white/10"></div>
        <div class="h-4 w-24 rounded bg-white/10"></div>
      </div>
      <div
        class="mx-auto bg-board-dark rounded aspect-square max-lg:max-w-[26rem]"
        style="width: 100%;"
      ></div>
    </div>
    <div class="card">
      <div class="h-4 w-16 rounded bg-white/10 mb-2"></div>
      <div class="grid grid-cols-2 gap-x-4 gap-y-1">
        {#each Array(6) as _}
          <div class="h-3 rounded bg-white/5"></div>
        {/each}
      </div>
    </div>
  </div>
{:else if error}
  <div class="text-center py-12 text-primary-err">{error}</div>
{:else if waiting}
  <div class="text-center py-12 text-white/50">
    <div class="animate-pulse">Waiting for game to be indexed...</div>
  </div>
{:else if game}
  <div class="flex flex-col gap-4">
    <button
      class="text-sm text-white/60 hover:text-white self-start"
      onclick={() => goto('/')}
    >
      &larr; Back
    </button>
    <div class="flex flex-col lg:flex-row lg:items-start lg:gap-6">
      <div class="card lg:flex-1">
        <div class="flex justify-between items-center mb-2 gap-1">
          <span
            class="min-w-0 inline-flex items-center gap-1.5 text-sm rounded transition-all {currentTurn ===
            'White'
              ? 'bg-white/10'
              : 'text-white/60'}"
          >
            <span class="shrink-0 w-3 h-3 rounded-full bg-white"></span>
            {#if game.white.type === 'Human'}
              <a
                href="/profile/{game.white.value}"
                class="text-primary hover:underline truncate max-w-24 sm:max-w-none"
              >
                {truncateAddr(game.white.value)}
              </a>
              {#if eloMap[game.white.value] != null}
                <span class="text-xs text-white/40 tabular-nums"
                  >({eloMap[game.white.value]})</span
                >
              {/if}
            {:else}
              <span class="truncate max-w-24 sm:max-w-none"
                >AI ({game.white.value})</span
              >
            {/if}
          </span>
          <span
            class="shrink-0 text-xs sm:text-sm px-1.5 sm:px-2 py-0.5 rounded whitespace-nowrap {game.status ===
            'in_progress'
              ? 'bg-primary-bgOk text-primary-green'
              : game.status === 'finished'
                ? 'bg-white/10 text-white/50'
                : 'bg-primary-bgErr text-primary-err'}"
          >
            {#if isSpectating}
              <span class="sm:hidden">Spec.</span><span class="hidden sm:inline"
                >Spectating</span
              >
            {:else if game.status === 'in_progress'}
              <span class="sm:hidden">Live</span><span class="hidden sm:inline"
                >in progress</span
              >
            {:else if game.status === 'waiting'}
              <span class="sm:hidden">Wait</span><span class="hidden sm:inline"
                >waiting</span
              >
            {:else if game.status === 'finished'}
              <span class="sm:hidden">Done</span><span class="hidden sm:inline"
                >finished</span
              >
            {:else}
              {game.status?.replace('_', ' ') ?? 'unknown'}
            {/if}
          </span>
          <span
            class="min-w-0 inline-flex items-center gap-1.5 text-sm rounded transition-all {currentTurn ===
            'Black'
              ? 'bg-white/10'
              : 'text-white/60'}"
          >
            {#if game.black?.type === 'Human'}
              <a
                href="/profile/{game.black.value}"
                class="text-primary hover:underline truncate max-w-24 sm:max-w-none"
              >
                {truncateAddr(game.black.value ?? '')}
              </a>
              {#if eloMap[game.black.value ?? ''] != null}
                <span class="text-xs text-white/40 tabular-nums"
                  >({eloMap[game.black.value ?? '']})</span
                >
              {/if}
            {:else if game.black?.type?.toLowerCase() === 'ai'}
              <span class="truncate max-w-24 sm:max-w-none"
                >AI ({game.black.value})</span
              >
            {:else}
              <span class="truncate max-w-24 sm:max-w-none">...</span>
            {/if}
            <span
              class="shrink-0 w-3 h-3 rounded-full bg-gray-700 border border-gray-500"
            ></span>
          </span>
        </div>

        {#if game.created_at}
          <div class="text-xs text-white/40 text-center mt-1">
            Started {dayjs(game.created_at).format('lll')}
          </div>
        {/if}

        <div class="flex flex-col items-center gap-1 relative">
          <div
            class="flex justify-center w-full max-lg:max-w-[26rem]"
            style="width: 100%;"
          >
            <CapturedPieces pieces={topCaptured} />
          </div>

          <Board
            board={displayBoard}
            fen={displayFen}
            onMove={handleMove}
            disabled={game.status !== 'in_progress' ||
              submitting ||
              !indexed ||
              !isMyTurn ||
              !isViewingCurrent}
            loading={submitting}
            {flipped}
            lastMove={displayLastMove}
          />

          <div
            class="flex justify-center w-full max-lg:max-w-[26rem]"
            style="width: 100%;"
          >
            <CapturedPieces pieces={bottomCaptured} />
          </div>

          {#if !indexed}
            <div
              class="absolute inset-0 flex items-center justify-center bg-black/60 rounded z-10"
            >
              <div class="flex flex-col items-center gap-2">
                <div
                  class="w-6 h-6 border-2 border-white/30 border-t-white rounded-full animate-spin"
                ></div>
                <span class="text-sm text-white/70">Setting up game...</span>
              </div>
            </div>
          {/if}
        </div>

        {#if game.outcome}
          <div class="flex justify-between items-center mt-2 gap-2">
            <span class="font-semibold">
              {#if game.outcome.result === 'Stalemate'}
                Draw &mdash; Stalemate
              {:else if game.resigner}
                {game.outcome.color} wins by resignation!
              {:else}
                {game.outcome.color} wins by checkmate!
              {/if}
            </span>
            <button class="btn-ghost text-sm shrink-0" onclick={openShareModal}>
              Share
            </button>
          </div>
        {:else if game.status === 'in_progress'}
          <div class="flex justify-between items-center mt-2 gap-2">
            <span>
              {#if isMyTurn}
                <span
                  class="inline-flex items-center gap-2 text-sm font-bold px-3 py-1 rounded bg-primary-bgOk text-primary-green animate-pulse"
                >
                  Your turn!
                  {#if isInCheck}
                    <span
                      class="text-red-400 bg-red-500/20 px-1.5 py-0.5 rounded text-xs"
                    >
                      Check!
                    </span>
                  {/if}
                </span>
              {:else if $accountStore}
                <span
                  class="inline-flex items-center gap-2 text-sm text-white/50"
                >
                  Waiting for {currentTurn ?? 'opponent'}...
                  {#if isInCheck}
                    <span
                      class="text-red-400 bg-red-500/20 px-1.5 py-0.5 rounded text-xs font-bold animate-pulse"
                    >
                      Check!
                    </span>
                  {/if}
                </span>
              {:else}
                <span
                  class="inline-flex items-center gap-2 text-sm text-white/50"
                >
                  {currentTurn ?? 'Unknown'}'s turn
                  {#if isInCheck}
                    <span
                      class="text-red-400 bg-red-500/20 px-1.5 py-0.5 rounded text-xs font-bold animate-pulse"
                    >
                      Check!
                    </span>
                  {/if}
                </span>
              {/if}
            </span>
            {#if canResign || canCancel || canPublicCancel}
              <div class="flex gap-2 shrink-0">
                {#if canResign}
                  <button
                    class="btn text-sm text-primary-err border border-primary-err hover:bg-primary-bgErr"
                    onclick={handleResign}
                  >
                    Resign
                  </button>
                {/if}
                {#if canCancel}
                  <button class="btn-secondary text-sm" onclick={handleCancel}>
                    Cancel Game
                  </button>
                {/if}
                {#if canPublicCancel}
                  <button
                    class="btn-secondary text-sm text-primary-warn border border-primary-warn/50"
                    onclick={handlePublicCancel}
                  >
                    Cancel Stale Game
                  </button>
                {/if}
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <div class="flex flex-col gap-4 lg:w-80 lg:shrink-0">
        <MoveHistory
          {moves}
          selectedMoveIndex={viewingMoveIndex}
          onSelectMove={selectMove}
          {isViewingCurrent}
          isLiveMyTurn={isMyTurn}
        />

        {#if gameBets.length > 0}
          <div class="card space-y-2">
            <h3 class="text-sm font-semibold">Game Bets ({gameBets.length})</h3>
            <div class="space-y-1.5">
              {#each gameBets as bet}
                <div class="flex items-center justify-between text-xs">
                  <div class="truncate mr-2">
                    <span class="text-white/70">{truncateAddr(bet.bettor)}</span
                    >
                    <span class="text-white/40 ml-1">bet {bet.amount} on</span>
                    <span class="text-primary ml-1"
                      >{truncateAddr(bet.winner)}</span
                    >
                  </div>
                  <div class="shrink-0 flex items-center gap-2">
                    <span
                      class="px-1.5 py-0.5 rounded {bet.status === 'pending'
                        ? 'bg-yellow-400/20 text-yellow-400'
                        : bet.status === 'locked'
                          ? 'bg-blue-400/20 text-blue-400'
                          : 'bg-green-400/20 text-green-400'}"
                      >{bet.status}</span
                    >
                    {#if bet.payout}
                      <span class="text-green-400">+{bet.payout}</span>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<ConfirmModal
  open={showResignModal}
  title="Resign Game?"
  message="Are you sure you want to resign? This cannot be undone and will count as a loss."
  confirmLabel="Resign"
  onconfirm={confirmResign}
  onclose={() => (showResignModal = false)}
/>

<ConfirmModal
  open={showCancelModal}
  title="Cancel Game?"
  message="Are you sure you want to cancel this game?"
  confirmLabel="Yes, Cancel"
  onconfirm={confirmCancel}
  onclose={() => (showCancelModal = false)}
/>

<ConfirmModal
  open={showPublicCancelModal}
  title="Cancel Stale Game?"
  message="This game has been inactive for over 14 days. Cancelling will refund all wagers and bets to the players. This action cannot be undone."
  confirmLabel="Cancel Stale Game"
  onconfirm={confirmPublicCancel}
  onclose={() => (showPublicCancelModal = false)}
/>

<Modal open={showShareModal} onclose={() => (showShareModal = false)}>
  <div
    class="w-80 sm:w-96 p-6 rounded-lg bg-surface shadow-xl border border-white/20"
  >
    <h2 class="text-lg font-semibold text-white mb-4">Share Game</h2>

    <button
      class="btn-ghost w-full mb-2 flex items-center justify-center gap-2"
      onclick={shareOnX}
    >
      Share on X
    </button>

    <button
      class="btn-ghost w-full flex items-center justify-center gap-2"
      onclick={copyImage}
      disabled={copyingImage}
    >
      {copyingImage ? 'Generating...' : 'Copy Image'}
    </button>

    <p class="text-xs text-white/40 mt-4 text-center leading-relaxed">
      Share the game result on X, or copy an image of the final position to
      share anywhere.
    </p>
  </div>
</Modal>
