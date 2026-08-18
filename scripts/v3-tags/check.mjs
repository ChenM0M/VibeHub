import { readFile } from "node:fs/promises";
import path from "node:path";
import { build } from "esbuild";

const root = process.cwd();
const source = path.join(root, "src", "lib", "tagLaunch.ts");

const bundle = await build({
  entryPoints: [source],
  bundle: true,
  write: false,
  platform: "node",
  format: "esm",
  logLevel: "error",
});
const moduleUrl = `data:text/javascript;base64,${Buffer.from(bundle.outputFiles[0].text).toString("base64")}`;
const { normalizeLaunchInput, tokenizeCommandLine, tagLaunchIssue } = await import(moduleUrl);

const failures = [];
function expect(label, actual, expected) {
  const actualJson = JSON.stringify(actual);
  const expectedJson = JSON.stringify(expected);
  if (actualJson !== expectedJson) failures.push(`${label}: got ${actualJson}, expected ${expectedJson}`);
}

// A whole launch command pasted into the arguments field, with the path stored
// including its literal quote characters. This is the shape that used to reach
// the backend with no executable and fail as "Failed to launch any tools".
const pastedCommand = 'claude --settings "/Users/demo/.claude/vibehub-profiles/stepfun.settings.json"';
expect("pasted command is split into an executable and clean arguments", normalizeLaunchInput("", pastedCommand), {
  executable: "claude",
  args: ["--settings", "/Users/demo/.claude/vibehub-profiles/stepfun.settings.json"],
  promotedFromArgs: true,
});

expect("a quoted path keeps its spaces and loses its quotes", tokenizeCommandLine('--settings "/Users/a b/c.json"'), [
  "--settings",
  "/Users/a b/c.json",
]);

expect("a command typed into the executable field is split too", normalizeLaunchInput('claude --settings "/p q/s.json"', ""), {
  executable: "claude",
  args: ["--settings", "/p q/s.json"],
  promotedFromArgs: false,
});

expect("a normal executable and argument pair is left alone", normalizeLaunchInput("code", "--new-window"), {
  executable: "code",
  args: ["--new-window"],
  promotedFromArgs: false,
});

expect("blank input resolves to no executable", normalizeLaunchInput("  ", "   "), {
  executable: "",
  args: [],
  promotedFromArgs: false,
});

expect("single quotes are honoured", tokenizeCommandLine("--settings '/Users/a b/c.json'"), [
  "--settings",
  "/Users/a b/c.json",
]);

expect(
  "a cli tag without an executable is not launchable",
  tagLaunchIssue({ id: "t", name: "Stepfun", color: "#000", category: "cli", config: { args: ["claude"] } }),
  "missing_executable",
);

expect(
  "a label-only category is never launchable",
  tagLaunchIssue({ id: "t", name: "Frontend", color: "#000", category: "custom" }),
  "not_launchable_category",
);

expect(
  "a configured cli tag is launchable",
  tagLaunchIssue({ id: "t", name: "CC", color: "#000", category: "cli", config: { executable: "claude", args: ["--settings", "/p/s.json"] } }),
  null,
);

// Every launch surface must agree with tagLaunchIssue rather than re-deriving
// its own launchability rule, which is what made the context menu and the
// custom launch dialog disagree.
const surfaces = [
  "src/components/ProjectCard.tsx",
  "src/components/LaunchDialog.tsx",
];
for (const surface of surfaces) {
  const content = await readFile(path.join(root, surface), "utf8");
  if (!content.includes("isTagLaunchable")) failures.push(`${surface}: does not use the shared isTagLaunchable rule`);
  if (/tag\.config\?\.executable/.test(content)) failures.push(`${surface}: still derives launchability from tag.config.executable directly`);
}

// The sidebar "+" must open the create dialog and persist through add_tag,
// instead of navigating away or reusing update_tag for a tag that does not
// exist yet.
const sidebar = await readFile(path.join(root, "src", "components", "Sidebar.tsx"), "utf8");
if (!sidebar.includes("openCreateTagDialog")) failures.push("Sidebar.tsx: no create-tag entry point");
if (!/invoke\(editingTag \? 'update_tag' : 'add_tag'/.test(sidebar)) {
  failures.push("Sidebar.tsx: tag save does not choose between add_tag and update_tag");
}

if (failures.length > 0) {
  console.error(`Tag launch check failed:\n${failures.map((line) => `  - ${line}`).join("\n")}`);
  process.exit(1);
}

console.log("Tag launch check passed: 9 assertions, 2 launch surfaces, sidebar create wiring.");
