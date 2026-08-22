import { access, readFile } from "node:fs/promises";
import { Buffer } from "node:buffer";

const root = new URL("../../", import.meta.url);
const readText = (path) => readFile(new URL(path, root), "utf8");
const readJson = async (path) => JSON.parse(await readText(path));

const packageJson = await readJson("package.json");
const packageLock = await readJson("package-lock.json");
const tauriConfig = await readJson("src-tauri/tauri.conf.json");
const tauriWindowsConfig = await readJson("src-tauri/tauri.windows.conf.json");
const coreCargo = await readText("crates/vibehub-core/Cargo.toml");
const cliCargo = await readText("crates/vibehub-cli/Cargo.toml");
const tauriCargo = await readText("src-tauri/Cargo.toml");
const releaseWorkflow = await readText(".github/workflows/release.yml");
const homebrewWorkflow = await readText(".github/workflows/homebrew.yml");

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

const baseWindow = tauriConfig.app?.windows?.[0];
const windowsWindow = tauriWindowsConfig.app?.windows?.[0];
if (baseWindow?.decorations !== true) {
  throw new Error("base Tauri window must retain native decorations for non-Windows platforms");
}
if (windowsWindow?.decorations !== false) {
  throw new Error("Windows Tauri window must disable native decorations for the custom title bar");
}

const sharedWindowFields = [
  "title",
  "width",
  "height",
  "minWidth",
  "minHeight",
  "resizable",
  "fullscreen",
  "transparent",
  "center",
];
for (const field of sharedWindowFields) {
  if (windowsWindow[field] !== baseWindow[field]) {
    throw new Error(`Windows Tauri window ${field} must match the base window configuration`);
  }
}

const requiredReleaseWorkflowFragments = [
  "uses: ./.github/workflows/homebrew.yml",
  "require_tap_update: true",
  "secrets: inherit",
  "node scripts/release/notes.mjs",
  "--notes-file",
];
for (const fragment of requiredReleaseWorkflowFragments) {
  if (!releaseWorkflow.includes(fragment)) {
    throw new Error(`release workflow must retain the Homebrew recovery contract: ${fragment}`);
  }
}

const requiredHomebrewWorkflowFragments = [
  "workflow_call:",
  "require_tap_update:",
  "HOMEBREW_TAP_TOKEN is required for the release-triggered cask update.",
];
for (const fragment of requiredHomebrewWorkflowFragments) {
  if (!homebrewWorkflow.includes(fragment)) {
    throw new Error(`Homebrew workflow must retain the reusable release contract: ${fragment}`);
  }
}

if (releaseWorkflow.includes("--generate-notes")) {
  throw new Error("release workflow must use --notes-file instead of --generate-notes");
}

const pngSize = (bytes) => {
  if (bytes.length < 24 || bytes.subarray(1, 4).toString("ascii") !== "PNG") {
    return null;
  }
  return { width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) };
};

const releaseTag = process.env.RELEASE_TAG?.trim();
if (releaseTag) {
  const tagVersion = releaseTag.startsWith("v") ? releaseTag.slice(1) : releaseTag;
  if (tagVersion !== expected) {
    throw new Error(`release tag ${releaseTag} does not match configured version ${expected}`);
  }
  const tag = releaseTag.startsWith("v") ? releaseTag : `v${releaseTag}`;
  const notesPath = `docs/releases/${tag}.md`;
  const notes = await readText(notesPath);
  // Release screenshots are optional. When the notes reference assets under
  // assets/releases/, each must be a valid 1600x1000 PNG; a release with no
  // screenshots is fully allowed.
  const imageRefs = [...notes.matchAll(/assets\/releases\/[^)\s]+/g)].map((match) => match[0]);
  for (const relativePath of new Set(imageRefs)) {
    const bytes = Buffer.from(await readFile(new URL(relativePath, root)));
    const size = pngSize(bytes);
    if (!size) {
      throw new Error(`${relativePath} is not a PNG`);
    }
    if (size.width !== 1600 || size.height !== 1000) {
      throw new Error(`${relativePath} must be 1600x1000, got ${size.width}x${size.height}`);
    }
  }
  const { buildReleaseNotes, rewriteAssetUrls } = await import("./notes.mjs");
  const rewritten = rewriteAssetUrls("![demo](../../assets/releases/v0.0.0/demo.png)", {
    repo: "ChenM0M/VibeHub",
    tag: "v0.0.0",
  });
  if (rewritten !== "![demo](https://github.com/ChenM0M/VibeHub/raw/v0.0.0/assets/releases/v0.0.0/demo.png)") {
    throw new Error("scripts/release/notes.mjs did not rewrite relative screenshot URLs");
  }
  const rendered = await buildReleaseNotes(tag);
  if (rendered.trim().length === 0) {
    throw new Error(`rendered release notes for ${tag} were empty`);
  }
  await access(new URL("scripts/release/capture-macos.sh", root));
}

console.log(JSON.stringify({ ok: true, version: expected, release_tag: releaseTag || null }));
