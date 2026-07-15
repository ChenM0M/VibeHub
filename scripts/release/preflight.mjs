import { readFile } from "node:fs/promises";

const root = new URL("../../", import.meta.url);
const readText = (path) => readFile(new URL(path, root), "utf8");
const readJson = async (path) => JSON.parse(await readText(path));

const packageJson = await readJson("package.json");
const packageLock = await readJson("package-lock.json");
const tauriConfig = await readJson("src-tauri/tauri.conf.json");
const coreCargo = await readText("crates/vibehub-core/Cargo.toml");
const cliCargo = await readText("crates/vibehub-cli/Cargo.toml");
const tauriCargo = await readText("src-tauri/Cargo.toml");

const cargoVersion = (content, label) => {
  const match = content.match(/^version\s*=\s*"([^"]+)"/m);
  if (!match) throw new Error(`${label} does not declare a package version`);
  return match[1];
};

const expected = packageJson.version;
const versions = new Map([
  ["package.json", expected],
  ["package-lock.json", packageLock.version],
  ["package-lock root package", packageLock.packages?.[""]?.version],
  ["crates/vibehub-core/Cargo.toml", cargoVersion(coreCargo, "crates/vibehub-core/Cargo.toml")],
  ["crates/vibehub-cli/Cargo.toml", cargoVersion(cliCargo, "crates/vibehub-cli/Cargo.toml")],
  ["src-tauri/Cargo.toml", cargoVersion(tauriCargo, "src-tauri/Cargo.toml")],
  ["src-tauri/tauri.conf.json", tauriConfig.version],
]);

for (const [label, version] of versions) {
  if (version !== expected) {
    throw new Error(`version mismatch: ${label}=${version ?? "<missing>"}, expected ${expected}`);
  }
}

const releaseTag = process.env.RELEASE_TAG?.trim();
if (releaseTag) {
  const tagVersion = releaseTag.startsWith("v") ? releaseTag.slice(1) : releaseTag;
  if (tagVersion !== expected) {
    throw new Error(`release tag ${releaseTag} does not match configured version ${expected}`);
  }
}

console.log(JSON.stringify({ ok: true, version: expected, release_tag: releaseTag || null }));
