<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { api, type Game, type GameMove } from '$lib/api/client';
	import { contract } from '$lib/near/connector';
	import { accountStore } from '$lib/near/account';
	import { colorFromFEN } from '$lib/chess/board';
	import { showTxToast } from '$lib/toast';
	import Board from '$lib/components/Board.svelte';
	import MoveHistory from '$lib/components/MoveHistory.svelte';

	let game = $state<Game | null>(null);
	let moves = $state<GameMove[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let submitting = $state(false);
	let pollInterval: ReturnType<typeof setInterval>;

	const gameId = decodeURIComponent(page.params.id ?? '');

	let lastMove = $derived(
		moves.length > 0
			? { from: moves[moves.length - 1].move_notation.slice(0, 2), to: moves[moves.length - 1].move_notation.slice(2, 4) }
			: null
	);

	let isMyTurn = $derived.by(() => {
		if (!game || game.status !== 'in_progress' || !$accountStore) return false;
		const turn = game.fen ? colorFromFEN(game.fen) : 'white';
		const myColor = game.white.value === $accountStore ? 'white' : game.black?.value === $accountStore ? 'black' : null;
		return turn === myColor;
	});

	let canResign = $derived(game?.status === 'in_progress' && $accountStore && (game.white.value === $accountStore || game.black?.value === $accountStore));
	let canCancel = $derived(game?.status === 'waiting' && $accountStore && (game.white.value === $accountStore || game.black?.value === $accountStore));
	let isSpectating = $derived($accountStore && game && game.white.value !== $accountStore && game.black?.value !== $accountStore);

	async function load() {
		try {
			const [g, m] = await Promise.all([
				api.game(gameId),
				api.gameMoves(gameId)
			]);
			game = g;
			moves = m;
		} catch (e) {
			error = 'Failed to load game';
			console.error(e);
		} finally {
			loading = false;
		}
	}

	function handleMove(from: string, to: string) {
		if (!game || submitting) return;
		submitting = true;
		showTxToast(
			contract.playMove(game.game_id, from + to)
				.finally(() => { submitting = false; })
		);
		setTimeout(load, 4000);
	}

	function handleResign() {
		if (!game) return;
		showTxToast(contract.resign(game.game_id));
		setTimeout(load, 4000);
	}

	function handleCancel() {
		if (!game) return;
		showTxToast(contract.cancel(game.game_id));
		setTimeout(load, 4000);
	}

	onMount(() => {
		load();
		pollInterval = setInterval(load, 15000);
		return () => clearInterval(pollInterval);
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
			<div class="mx-auto bg-board-dark rounded aspect-square" style="width: min(100%, 30rem);"></div>
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
{:else if game}
	<div class="flex flex-col gap-4">
		<div class="card">
			<div class="flex justify-between items-center mb-2">
				<span class="text-sm">
					<span class="inline-block w-3 h-3 rounded-full bg-white mr-1 align-middle"></span>
					{game.white.type === 'Human' ? game.white.value : 'AI'}
				</span>
				<span class="text-sm px-2 py-0.5 rounded {
					game.status === 'in_progress' ? 'bg-primary-bgOk text-primary-green' :
					game.status === 'finished' ? 'bg-white/10 text-white/50' :
					'bg-primary-bgErr text-primary-err'
				}">
					{#if isSpectating}
						Spectating
					{:else}
						{game.status?.replace('_', ' ') ?? 'unknown'}
					{/if}
				</span>
				<span class="text-sm">
					{game.black?.type === 'Human' ? game.black.value : game.black?.type === 'AI' ? 'AI' : '...'}
					<span class="inline-block w-3 h-3 rounded-full bg-gray-700 border border-gray-500 ml-1 align-middle"></span>
				</span>
			</div>

			<div class="flex justify-center">
				<Board
					board={game.board}
					fen={game.fen ?? undefined}
					onMove={handleMove}
					disabled={game.status !== 'in_progress' || submitting || !isMyTurn}
					{lastMove}
				/>
			</div>

			{#if game.fen && game.status === 'in_progress'}
				<div class="text-center mt-2 text-sm text-white/70">
					Turn: {colorFromFEN(game.fen)}
					{#if !isMyTurn && $accountStore}
						<span class="text-white/40">(opponent's move)</span>
					{:else if isMyTurn}
						<span class="text-primary-green font-semibold">— your turn!</span>
					{/if}
				</div>
			{/if}

			{#if game.outcome}
				<div class="text-center mt-2 font-semibold">
					{game.outcome.result === 'Victory' ? `${game.outcome.color} wins!` : 'Draw - Stalemate'}
				</div>
			{/if}

			{#if canResign || canCancel}
				<div class="flex gap-2 mt-3 justify-center">
					{#if canResign}
						<button class="btn text-sm text-primary-err border-primary-err hover:bg-primary-bgErr" onclick={handleResign}>
							Resign
						</button>
					{/if}
					{#if canCancel}
						<button class="btn-secondary text-sm" onclick={handleCancel}>
							Cancel Game
						</button>
					{/if}
				</div>
			{/if}
		</div>

		<MoveHistory {moves} />
	</div>
{/if}
