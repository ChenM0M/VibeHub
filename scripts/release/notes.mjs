import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = fileURLToPath(new URL("../../", import.meta.url));
const readText = (relativePath) => readFile(path.join(root, relativePath), "utf8");

export const rewriteAssetUrls = (markdown, { repo, tag }) =>
  markdown.replace(
    /\]\((?:\.\.\/)*\/?(assets\/[^)\s]+)\)/g,
    (_match, assetPath) => `](https://github.com/${repo}/raw/${tag}/${assetPath})`,
  );

export const extractChangelogSection = (changelog, version) => {
  const heading = `## v${version.replace(/^v/, "")}`;
  const start = changelog.indexOf(heading);
  if (start === -1) return null;
  const after = changelog.indexOf("\n## ", start + heading.length);
  const body = changelog.slice(start, after === -1 ? undefined : after).trim();
  return body.replace(/^##\s+/, "# ");
};

const previousReleaseTag = (changelog, version) => {
  const heading = `## v${version.replace(/^v/, "")}`;
  const start = changelog.indexOf(heading);
  if (start === -1) return null;
  const after = changelog.indexOf("\n## ", start + heading.length);
  if (after === -1) return null;
  const match = changelog.slice(after).match(/^##\s+(v?[0-9][^\s]+)/m);
  return match ? (match[1].startsWith("v") ? match[1] : `v${match[1]}`) : null;
};

export const buildReleaseNotes = async (tag, { repo = "ChenM0M/VibeHub" } = {}) => {
  const normalizedTag = tag.startsWith("v") ? tag : `v${tag}`;
  const version = normalizedTag.slice(1);
  const notesPath = `docs/releases/${normalizedTag}.md`;
  let markdown;
  try {
    markdown = await readText(notesPath);
  } catch {
    const changelog = await readText("CHANGELOG.md");
    const section = extractChangelogSection(changelog, version);
    if (!section) {
      throw new Error(`missing ${notesPath} and no CHANGELOG section for ${normalizedTag}`);
    }
    const previous = previousReleaseTag(changelog, version);
    markdown = `${section}\n\n**Full Changelog**: https://github.com/${repo}/compare/${previous ?? "HEAD"}...${normalizedTag}\n`;
  }
  return rewriteAssetUrls(markdown.trim(), { repo, tag: normalizedTag }) + "\n";
};

const invokedDirectly = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invokedDirectly) {
  const tag = process.argv[2] || process.env.RELEASE_TAG;
  if (!tag) {
    console.error("usage: node scripts/release/notes.mjs <tag>");
    process.exit(2);
  }
  const notes = await buildReleaseNotes(tag, {
    repo: process.env.GITHUB_REPOSITORY || "ChenM0M/VibeHub",
  });
  process.stdout.write(notes);
}
