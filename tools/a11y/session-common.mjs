// Shared parts of the screen-reader sessions run through Guidepup on CI
// runners (nvda-session.mjs, voiceover-session.mjs; ADR-0039): the
// command line, launching the GUI, time limits, checking the spoken
// phrases against fixtures/t/expected-phrases.json, and the report.
//
// Runs only on CI runners. It never drives a screen reader on a
// developer's machine: the owner's NVDA and JAWS are their working tools.

import { spawn } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const here = path.dirname(fileURLToPath(import.meta.url));
export const repo = path.resolve(here, "..", "..");

/** Refuses to run anywhere but a CI runner, so no one's own screen reader is taken over. */
export function requireRunner(name) {
  if (process.env.GITHUB_ACTIONS !== "true" && process.env.TW_A11Y_ALLOW_LOCAL !== "yes") {
    console.error(
      `${name} drives a screen reader and runs only on CI runners (GITHUB_ACTIONS=true). ` +
        "It must never take over the screen reader of the person at the machine.",
    );
    process.exit(2);
  }
}

/** --exe, --doc, --expected, --out, with defaults from the repository. */
export function parseArgs(defaultExe) {
  const args = {
    exe: path.join(repo, "target", "debug", defaultExe),
    doc: path.join(repo, "fixtures", "t", "reading.md"),
    expected: path.join(repo, "fixtures", "t", "expected-phrases.json"),
    out: path.join(process.cwd(), "a11y-session"),
  };
  const argv = process.argv.slice(2);
  for (let i = 0; i < argv.length; i += 2) {
    const key = argv[i].replace(/^--/, "");
    if (!(key in args) || argv[i + 1] === undefined) {
      console.error(`Unknown or incomplete option ${argv[i]}. Options: --exe --doc --expected --out`);
      process.exit(2);
    }
    args[key] = path.resolve(argv[i + 1]);
  }
  fs.mkdirSync(args.out, { recursive: true });
  return args;
}

export const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** A promise that fails after `ms`, so no wait runs forever. */
export function withTimeout(promise, ms, what) {
  let timer;
  const limit = new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error(`${what} took longer than ${ms / 1000} seconds`)), ms);
  });
  return Promise.race([promise, limit]).finally(() => clearTimeout(timer));
}

/** Starts the GUI in the foreground (a runner has no one to disturb), silent. */
export function launchGui(args, extra = []) {
  const home = fs.mkdtempSync(path.join(os.tmpdir(), "tw-a11y-"));
  const guiArgs = [
    args.doc,
    "--backend", "paced",
    "--home", home,
    "--log-file", path.join(args.out, "gui.log"),
    "--exit-after", "300",
    ...extra,
  ];
  const child = spawn(args.exe, guiArgs, { stdio: "ignore", detached: false });
  child.on("error", (e) => console.error(`Could not start the GUI: ${e.message}`));
  return child;
}

/** Checks each step's phrases against the expectations; returns the result lines. */
export function check(expected, steps) {
  const lines = [];
  let failed = 0;
  for (const step of steps) {
    const rules = expected.steps[step.id] || [];
    const heard = step.phrases.join(" | ");
    for (const rule of rules) {
      const re = new RegExp(rule.pattern, "i");
      const count = step.phrases.filter((p) => re.test(p)).length;
      if (count === 0) {
        if (rule.level === "must") {
          failed += 1;
          lines.push({ step: step.id, word: "Fail", text: `nothing matched "${rule.pattern}": ${rule.why}` });
        } else {
          lines.push({ step: step.id, word: "Warning", text: `nothing matched "${rule.pattern}": ${rule.why}` });
        }
      } else if (rule.once && count > 1) {
        lines.push({ step: step.id, word: "Warning", text: `"${rule.pattern}" was said ${count} times, not once` });
      } else {
        lines.push({ step: step.id, word: "Pass", text: `"${rule.pattern}" heard${rule.once ? " once" : ""}` });
      }
    }
    if (step.error) {
      failed += 1;
      lines.push({ step: step.id, word: "Fail", text: step.error });
    }
    if (!heard && rules.length === 0) {
      lines.push({ step: step.id, word: "Warning", text: "nothing was said" });
    }
  }
  return { lines, failed };
}

/**
 * Writes the phrases (JSON) and the report (Markdown, meaning first on
 * every line, in words), and appends the report to the run summary.
 */
export function report(args, title, answer, steps, result, notes = []) {
  fs.writeFileSync(path.join(args.out, "phrases.json"), JSON.stringify({ answer, steps, notes }, null, 2) + "\n");
  const md = [`## ${title}`, "", answer, ""];
  for (const note of notes) md.push(`- ${note}`);
  if (notes.length) md.push("");
  for (const step of steps) {
    md.push(`### Step: ${step.id}`, "");
    md.push(`- Keys: ${step.keys.length ? step.keys.join(", ") : "none"}`);
    for (const line of result.lines.filter((l) => l.step === step.id)) {
      md.push(`- ${line.word}: ${line.text}`);
    }
    if (step.phrases.length) {
      for (const p of step.phrases) md.push(`- Heard: ${JSON.stringify(p)}`);
    } else {
      md.push("- Heard: nothing");
    }
    md.push("");
  }
  md.push(result.failed ? `Fail: ${result.failed} must-hear checks failed.` : "Pass: every must-hear check passed.", "");
  const text = md.join("\n");
  fs.writeFileSync(path.join(args.out, "report.md"), text);
  if (process.env.GITHUB_STEP_SUMMARY) fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, text + "\n");
  console.log(text);
}

/**
 * The answer line: "yes" when the screen reader named the window or spoke
 * one of the GUI's own announcements, which the GUI's log lists
 * ("announce Polite: Paused."), so the phrase can only have come from it.
 */
export function answerFor(reader, args, steps) {
  let announced = [];
  try {
    announced = fs
      .readFileSync(path.join(args.out, "gui.log"), "utf8")
      .split(/\r?\n/)
      .map((l) => l.match(/^announce \w+: (.+)$/))
      .filter(Boolean)
      .map((m) => m[1].trim());
  } catch {
    // No log: only the window's name can answer.
  }
  const all = steps.flatMap((s) => s.phrases);
  const named = all.filter((p) => /Reading check|textweaver/i.test(p)).length;
  const spoken = announced.filter((a) => all.some((p) => p.includes(a.replace(/\.$/, "")))).length;
  if (named || spoken) {
    return `Answer: yes. ${reader} spoke ${all.length} phrases from the textweaver window: ` +
      `${spoken} of its ${announced.length} announcements, and its name ${named} times.`;
  }
  return `Answer: no. ${reader} spoke ${all.length} phrases, none from the textweaver window; see the steps and gui.log.`;
}

/** Loads the expectations file. */
export function loadExpected(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
}
