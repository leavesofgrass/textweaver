#!/usr/bin/env node
// The Guidepup question (ADR-0039): can Guidepup's NVDA read the Xilem GUI,
// a native winit window, on a Windows runner? It starts NVDA through
// Guidepup, opens fixtures/t/reading.md in textweaver-xilem (silent paced
// backend, in the foreground), and records what NVDA says at each step:
// the window opening, reading three sentences, Pause, the next heading (h),
// Settings (Ctrl+Comma), and Escape. The phrases are checked against
// fixtures/t/expected-phrases.json (the owner's session 1 design).
//
// The first line of the report answers the question: "Answer: yes" when
// NVDA spoke from the textweaver window, "Answer: no" with the reason.
//
// Runs only on a CI runner (`npx guidepup install nvda` first); it refuses
// to run anywhere else, so it can never take over someone's own NVDA.
//
//   node tools/a11y/nvda-session.mjs --exe target/debug/textweaver-xilem.exe --out nvda

import { execFileSync } from "node:child_process";
import guidepup from "@guidepup/guidepup";
import {
  answerFor, check, launchGui, loadExpected, parseArgs, report, requireRunner, sleep, withTimeout,
} from "./session-common.mjs";

requireRunner("nvda-session.mjs");
const { nvda } = guidepup;
const args = parseArgs("textweaver-xilem.exe");
const expected = loadExpected(args.expected);

// Whole-run limit: the runner job has its own, but this ends the session
// (and NVDA) cleanly first.
const RUN_LIMIT_MS = 6 * 60 * 1000;
const watchdog = setTimeout(() => {
  console.error("Fail: the NVDA session passed its six-minute limit.");
  process.exit(3);
}, RUN_LIMIT_MS);

/** Runs a short PowerShell script and returns its output. */
function ps(script) {
  return execFileSync("powershell", ["-NoProfile", "-NonInteractive", "-Command", script], {
    encoding: "utf8",
    timeout: 20000,
  }).trim();
}

const FOREGROUND = `
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class TwFg {
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
}
"@`;

/** The GUI's main window handle, waiting up to 30 seconds for it. */
async function waitForWindow(pid) {
  for (let i = 0; i < 30; i += 1) {
    const handle = ps(`(Get-Process -Id ${pid} -ErrorAction SilentlyContinue).MainWindowHandle`);
    if (handle && handle !== "0") return handle;
    await sleep(1000);
  }
  return null;
}

/** Brings the window forward and says whether it is in the foreground. */
function focus(pid) {
  return ps(`${FOREGROUND}
$h = (Get-Process -Id ${pid}).MainWindowHandle
[TwFg]::ShowWindow($h, 9) | Out-Null
[TwFg]::SetForegroundWindow($h) | Out-Null
$shell = New-Object -ComObject WScript.Shell
$null = $shell.AppActivate(${pid})
Start-Sleep -Milliseconds 500
$fg = [TwFg]::GetForegroundWindow(); $p = 0
[TwFg]::GetWindowThreadProcessId($fg, [ref]$p) | Out-Null
if ($p -eq ${pid}) { 'yes' } else { "no (the foreground belongs to process $p)" }`);
}

async function phrases() {
  return (await nvda.spokenPhraseLog()).filter((p) => p && p.trim());
}

async function runStep(step) {
  // The open step keeps what was said since the launch: the window's
  // name is spoken when it takes the focus, before the step starts.
  if (step.id !== "open") await nvda.clearSpokenPhraseLog();
  try {
    for (const key of step.keys) {
      await withTimeout(nvda.press(key), 15000, `pressing ${key}`);
      await sleep(step.gap ?? 600);
    }
    await sleep(step.wait ?? 2000);
  } catch (e) {
    step.error = `could not press the keys: ${e.message}`;
  }
  step.phrases = await phrases();
  console.log(`${step.id}: ${step.phrases.length} phrases`);
  return step;
}

const steps = [];
const notes = [];
let gui = null;
let answer = "Answer: no. The session did not reach the textweaver window.";
let exitCode = 1;
try {
  await withTimeout(nvda.start(), 120000, "starting NVDA");
  await sleep(3000);
  await nvda.clearSpokenPhraseLog();

  gui = launchGui(args);
  const handle = await waitForWindow(gui.pid);
  if (!handle) throw new Error("the GUI opened no window in 30 seconds");
  const fg = focus(gui.pid);
  notes.push(`The window took the foreground: ${fg}.`);

  steps.push(await runStep({ id: "open", keys: [], wait: 4000 }));
  // About three sentences at the default 265 words per minute.
  steps.push(await runStep({ id: "read", keys: ["Control+Shift+Space"], wait: 6000 }));
  steps.push(await runStep({ id: "pause", keys: ["Control+Shift+Space"], wait: 2500 }));
  steps.push(await runStep({ id: "heading", keys: ["h"], wait: 2500 }));
  steps.push(await runStep({ id: "settings", keys: ["Control+,"], wait: 3500 }));
  steps.push(await runStep({ id: "close", keys: ["Escape"], wait: 2500 }));

  answer = answerFor("NVDA", args, steps);
} catch (e) {
  notes.push(`Stopped: ${e.message}.`);
} finally {
  if (gui && gui.exitCode === null) gui.kill();
  try {
    await withTimeout(nvda.stop(), 60000, "stopping NVDA");
  } catch (e) {
    notes.push(`NVDA did not stop cleanly: ${e.message}.`);
  }
}

const result = check(expected, steps);
report(args, "NVDA through Guidepup (Windows runner)", answer, steps, result, notes);
exitCode = answer.startsWith("Answer: yes") && result.failed === 0 ? 0 : 1;
clearTimeout(watchdog);
process.exit(exitCode);
