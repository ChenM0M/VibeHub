const viteEnv = (
  import.meta as ImportMeta & {
    env: { DEV?: boolean; VITE_V3_DEBUG?: string };
  }
).env;

export const V3_DEBUG_ENABLED =
  viteEnv.DEV === true || viteEnv.VITE_V3_DEBUG === "true";

export function isV3PlaygroundRequested(): boolean {
  return (
    V3_DEBUG_ENABLED &&
    new URLSearchParams(window.location.search).get("v3-playground") === "1"
  );
}
