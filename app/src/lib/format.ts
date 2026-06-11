import { FixedNumber } from '@tarnadas/fixed-number';

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
