export function formatTokenCount(value: number): string {
  if (value >= 1_000_000) return `${formatScaled(value / 1_000_000)}M`;
  if (value >= 1_000) return `${formatScaled(value / 1_000)}K`;
  return value.toLocaleString();
}

function formatScaled(value: number): string {
  return value.toFixed(1).replace(/\.0$/, "");
}
