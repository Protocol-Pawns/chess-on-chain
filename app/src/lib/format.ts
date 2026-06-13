import { FixedNumber } from '@tarnadas/fixed-number';

import type { Difficulty } from '$lib/near/contract-types';

const decimals = new Intl.NumberFormat('en', {
  minimumFractionDigits: 0,
  maximumFractionDigits: 2
});

const oneDecimal = new Intl.NumberFormat('en', {
  minimumFractionDigits: 1,
  maximumFractionDigits: 1
});

export function fmtDecimals(value: number): string {
  return decimals.format(value);
}

export function fmtOneDecimal(value: number): string {
  return oneDecimal.format(value);
}

export function truncateAddr(id: string, max = 20): string {
  if (id.length <= max) return id;
  return `${id.slice(0, 8)}...${id.slice(-4)}`;
}

export function fmtToken(raw: string, dec: number): string {
  if (!raw || raw === '0') return '0';
  return new FixedNumber(raw, dec).format({
    maximumFractionDigits: Math.min(dec, 6)
  });
}

export function fmtPPP(raw: string): string {
  return fmtToken(raw, 6);
}

export const AI_MOVE_GAS: Record<Difficulty, bigint> = {
  Easy: BigInt('100000000000000'),
  Medium: BigInt('300000000000000'),
  Hard: BigInt('500000000000000'),
  VeryHard: BigInt('800000000000000')
};

export const AI_MOVE_GAS_BUDGET: Record<Difficulty, bigint> = {
  Easy: BigInt('30000000000000'),
  Medium: BigInt('80000000000000'),
  Hard: BigInt('150000000000000'),
  VeryHard: BigInt('300000000000000')
};

export function fmtTGas(gas: bigint | string | number): string {
  const n = typeof gas === 'bigint' ? gas : BigInt(gas);
  const tgas = Number(n / BigInt(10 ** 12));
  return `${tgas.toLocaleString('en')} TGas`;
}
