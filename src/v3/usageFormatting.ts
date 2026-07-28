export function formatTokenCount(value: number): string {
  if (value >= 1_000_000) return `${formatScaled(value / 1_000_000)}M`;
  if (value >= 1_000) return `${formatScaled(value / 1_000)}K`;
  return value.toLocaleString();
}

export function formatTokenCountExact(value: number): string {
  return value.toLocaleString();
}

export function formatCost(value: number, currency: string = "USD"): string {
  if (value >= 1000) return `${currency} ${value.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  if (value >= 1) return `${currency} ${value.toFixed(2)}`;
  if (value >= 0.01) return `${currency} ${value.toFixed(4)}`;
  return `${currency} ${value.toFixed(6)}`;
}

export function formatCostExact(value: number, currency: string = "USD"): string {
  return `${currency} ${value.toLocaleString(undefined, { minimumFractionDigits: 6, maximumFractionDigits: 6 })}`;
}

function formatScaled(value: number): string {
  return value.toFixed(1).replace(/\.0$/, "");
}
