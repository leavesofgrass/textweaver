#!/usr/bin/env node
// VoiceOver on a macOS runner (ADR-0039): after `npx guidepup setup --ci`,
// can Guidepup start VoiceOver and hear the Xilem GUI? The same steps as
// nvda-session.mjs, with the same expectations, and the answer on the
// report's first line. If the runner cannot enable VoiceOver, the report
// says so and the macOS tree dump in the GUI workflow is the macOS check.
//
// Runs only on a CI runner; it refuses to run anywhere else.
//
//   node tools/a11y/voiceover-session.mjs --exe target/debug/textweaver-xilem --out voiceover

import { execFileSync } from "node:child_process";
import guidepup from "@guidepup/guidepup";
import {
  check, launchGui, loadExpected, parseArgs, report, requireRunner, sleep, withTimeout,
} from "./session-common.mjs";

requireRunner("voiceover-session.mjs");
const { voiceOver } = guidepup;
const args = parseArgs("textweaver-xilem");
const expected = loadExpected(args.expected);

const watchdog = setTimeout(() => {
  console.error("Fail: the VoiceOver session passed its six-minute limit.");
  process.exit(3);
}, 6 * 60 * 1000);

/** Brings the GUI's process to the front through System Events. */
function focus(pid) {
  const script = `tell application "System Events"
  set frontmost of (first process whose unix id is ${pid}) to true
  return name of first process whose frontmost is true
end tell`;
  return execFileSync("osascript", ["-e", script], { encoding: "utf8", timeout: 20000 }).trim();
}

async function runStep(step) {
  await voiceOver.clearSpokenPhraseLog();
  try {
    for (const key of step.keys) {
      await withTimeout(voiceOver.press(key), 15000, `pressing ${key}`);
      await sleep(600);
    }
    await sleep(step.wait ?? 2000);
  } catch (e) {
    step.error = `could not press the keys: ${e.message}`;
  }
  step.phrases = (await voiceOver.spokenPhraseLog()).filter((p) => p && p.trim());
  console.log(`${step.id}: ${step.phrases.length} phrases`);
  return step;
}

const steps = [];
const notes = [];
let gui = null;
let answer = "Answer: no. VoiceOver did not reach the textweaver window.";
try {
  await withTimeout(voiceOver.start(), 120000, "starting VoiceOver");
  await sleep(3000);
  await voiceOver.clearSpokenPhraseLog();

  gui = launchGui(args);
  // The window takes a moment; there is no handle to wait for on macOS.
  let front = "";
  for (let i = 0; i < 15 && !/textweaver/i.test(front); i += 1) {
    await sleep(2000);
    try {
      front = focus(gui.pid);
    } catch (e) {
      front = `(System Events: ${e.message.split("\n")[0]})`;
    }
  }
  notes.push(`The frontmost process: ${front || "unknown"}.`);

  steps.push(await runStep({ id: "open", keys: [], wait: 4000 }));
  steps.push(await runStep({ id: "read", keys: ["Control+Shift+Space"], wait: 6000 }));
  steps.push(await runStep({ id: "pause", keys: ["Control+Shift+Space"], wait: 2500 }));
  steps.push(await runStep({ id: "heading", keys: ["h"], wait: 2500 }));
  steps.push(await runStep({ id: "settings", keys: ["Control+,"], wait: 3500 }));
  steps.push(await runStep({ id: "close", keys: ["Escape"], wait: 2500 }));

  const fromWindow = steps.some((s) => s.phrases.some((p) => /Reading check|textweaver/i.test(p)));
  const total = steps.reduce((n, s) => n + s.phrases.length, 0);
  answer = fromWindow
    ? `Answer: yes. VoiceOver spoke ${total} phrases in the session, some naming the textweaver window.`
    : `Answer: no. VoiceOver spoke ${total} phrases, none naming the textweaver window; see the steps and gui.log.`;
} catch (e) {
  notes.push(`Stopped: ${e.message}.`);
} finally {
  if (gui && gui.exitCode === null) gui.kill();
  try {
    await withTimeout(voiceOver.stop(), 60000, "stopping VoiceOver");
  } catch (e) {
    notes.push(`VoiceOver did not stop cleanly: ${e.message}.`);
  }
}

const result = check(expected, steps);
report(args, "VoiceOver through Guidepup (macOS runner)", answer, steps, result, notes);
clearTimeout(watchdog);
process.exit(answer.startsWith("Answer: yes") && result.failed === 0 ? 0 : 1);
