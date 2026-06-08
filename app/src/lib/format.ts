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

export function fmtPPP(raw: string): string {
  const val = BigInt(raw);
  const whole = val / BigInt(1000000);
  const frac = val % BigInt(1000000);
  const fractional = Number(frac) / 1e6;
  return decimals.format(Number(whole) + fractional);
}
