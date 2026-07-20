import { readFile, readdir } from "node:fs/promises";
import path from "node:path";

const root = process.cwd();
const localeFiles = ["zh.json", "zh-TW.json", "en.json"];

function flatten(value, prefix = "v3") {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return [[prefix, typeof value]];
  }
  return Object.entries(value).flatMap(([key, child]) => flatten(child, `${prefix}.${key}`));
}

const localeTrees = await Promise.all(localeFiles.map(async (file) => {
  const content = await readFile(path.join(root, "src", "locales", file), "utf8");
  return [file, new Map(flatten(JSON.parse(content).v3))];
}));

const [referenceFile, reference] = localeTrees[0];
const failures = [];
for (const [file, tree] of localeTrees.slice(1)) {
  for (const [key, type] of reference) {
    if (!tree.has(key)) failures.push(`${file}: missing ${key}`);
    else if (tree.get(key) !== type) failures.push(`${file}: ${key} has type ${tree.get(key)}, expected ${type}`);
  }
  for (const key of tree.keys()) {
    const pluralBase = key.replace(/_(one|other)$/, "");
    if (!reference.has(key) && !reference.has(pluralBase)) failures.push(`${file}: extra ${key} (not in ${referenceFile})`);
  }
}

async function collectTsx(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = await Promise.all(entries.map((entry) => {
    const target = path.join(directory, entry.name);
    if (entry.isDirectory()) return collectTsx(target);
    return entry.isFile() && entry.name.endsWith(".tsx") ? [target] : [];
  }));
  return files.flat();
}

for (const file of await collectTsx(path.join(root, "src", "v3"))) {
  const source = await readFile(file, "utf8");
  const withoutComments = source.replace(/\/\*[\s\S]*?\*\//g, "").replace(/^\s*\/\/.*$/gm, "");
  const match = withoutComments.match(/[\p{Script=Han}]/u);
  if (match) failures.push(`${path.relative(root, file)}: contains hard-coded Han UI text`);
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}

console.log(`V3 locale parity OK: ${reference.size} leaf keys across ${localeFiles.join(", ")}`);
console.log("V3 TSX hard-coded Han scan OK");
