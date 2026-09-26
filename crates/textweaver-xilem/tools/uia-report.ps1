<#
.SYNOPSIS
  Launches the Xilem GUI (textweaver-xilem) out of the way and reports what
  assistive technology sees through UI Automation: every element's control
  type, name, and patterns; the document's text, caret, and highlight
  attribute while it reads; the buttons; the status bar; the live-region
  events; and whether the window ever took the foreground.

.DESCRIPTION
  The Xilem GUI draws its own widgets; AccessKit gives them to UI Automation
  as one provider per window. NVDA and JAWS read it through UIA, so this
  report checks UIA only (there are no Win32 child windows or menus).

  - The document: a Document control with TextPattern; its text, its
    selection (the caret) sampled every 400 ms while the silent `paced`
    backend reads, the background colour attribute at the caret (the
    spoken-word highlight), and caret moves made through TextPattern (as a
    screen reader's review cursor or a click would), which the app follows.
  - Buttons are pressed with InvokePattern; no keys are typed, no focus is
    needed.
  - Announcements are UIA LiveRegionChanged events from the live region's
    message nodes (AccessKit raises them; the report prints the element's
    name). UIA Notification events are listened to as well.
  - The GUI starts with --background (never activated, off screen, no
    taskbar button) and SW_SHOWNOACTIVATE; the report checks the foreground
    window did not change. It plays no audio (`paced` times words like an
    engine and outputs nothing) and is closed when the probe ends.

.EXAMPLE
  powershell -File crates/textweaver-xilem/tools/uia-report.ps1 -Out uia-xilem.md
#>
param(
    [string] $Exe = '',
    [string] $Document = '',
    [ValidateSet('paced', 'null')] [string] $Backend = 'paced',
    [string] $Out = ''
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
if (-not $Exe) {
    $target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $repo 'target' }
    $Exe = Join-Path $target 'debug\textweaver-xilem.exe'
}
if (-not $Document) { $Document = Join-Path $repo 'fixtures\sample.md' }
if (-not (Test-Path $Exe)) { throw "Not built: $Exe (cargo build -p textweaver-xilem)." }
$Exe = (Resolve-Path $Exe).Path
$Document = (Resolve-Path $Document).Path

Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, WindowsBase

$source = @'
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using System.Windows.Automation;
using System.Windows.Automation.Text;

public static class TwXUia
{
    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")]
    static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll")]
    static extern bool PostMessage(IntPtr hwnd, uint msg, IntPtr w, IntPtr l);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    struct STARTUPINFO
    {
        public int cb; public string lpReserved; public string lpDesktop; public string lpTitle;
        public int dwX; public int dwY; public int dwXSize; public int dwYSize;
        public int dwXCountChars; public int dwYCountChars; public int dwFillAttribute;
        public int dwFlags; public short wShowWindow; public short cbReserved2;
        public IntPtr lpReserved2; public IntPtr hStdInput; public IntPtr hStdOutput; public IntPtr hStdError;
    }
    [StructLayout(LayoutKind.Sequential)]
    struct PROCESS_INFORMATION { public IntPtr hProcess; public IntPtr hThread; public int dwProcessId; public int dwThreadId; }
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern bool CreateProcess(string app, StringBuilder cmd, IntPtr pa, IntPtr ta, bool inherit,
        uint flags, IntPtr env, string dir, ref STARTUPINFO si, out PROCESS_INFORMATION pi);
    [DllImport("kernel32.dll")]
    static extern bool CloseHandle(IntPtr h);

    delegate bool EnumProc(IntPtr hwnd, IntPtr lParam);
    [DllImport("user32.dll")]
    static extern bool EnumWindows(EnumProc proc, IntPtr lParam);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    static extern int GetWindowText(IntPtr hwnd, StringBuilder text, int max);

    public static readonly List<string> Events = new List<string>();
    static readonly Stopwatch Clock = Stopwatch.StartNew();
    static int listenPid;

    /// Starts the GUI without activating it: the first ShowWindow of a
    /// process started with STARTF_USESHOWWINDOW uses wShowWindow.
    public static int LaunchInactive(string exe, string args, string dir)
    {
        var si = new STARTUPINFO();
        si.cb = Marshal.SizeOf(typeof(STARTUPINFO));
        si.dwFlags = 0x1;          // STARTF_USESHOWWINDOW
        si.wShowWindow = 4;        // SW_SHOWNOACTIVATE
        PROCESS_INFORMATION pi;
        var cmd = new StringBuilder("\"" + exe + "\" " + args);
        if (!CreateProcess(exe, cmd, IntPtr.Zero, IntPtr.Zero, false, 0x08000000 /* CREATE_NO_WINDOW */,
                IntPtr.Zero, dir, ref si, out pi))
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);
        return pi.dwProcessId;
    }

    public static IntPtr FindFrame(int pid)
    {
        IntPtr found = IntPtr.Zero;
        EnumWindows(delegate (IntPtr hwnd, IntPtr l)
        {
            uint owner;
            GetWindowThreadProcessId(hwnd, out owner);
            if (owner != (uint)pid) return true;
            var sb = new StringBuilder(256);
            GetWindowText(hwnd, sb, 256);
            if (sb.ToString().Contains("textweaver")) { found = hwnd; return false; }
            return true;
        }, IntPtr.Zero);
        return found;
    }

    public static AutomationElement WaitForWindow(int pid, int timeoutMs)
    {
        var sw = Stopwatch.StartNew();
        while (sw.ElapsedMilliseconds < timeoutMs)
        {
            IntPtr hwnd = FindFrame(pid);
            if (hwnd != IntPtr.Zero) return AutomationElement.FromHandle(hwnd);
            Thread.Sleep(100);
        }
        return null;
    }

    public static void Close(IntPtr hwnd) { PostMessage(hwnd, 0x0010, IntPtr.Zero, IntPtr.Zero); }

    public static bool IsForeground(int pid)
    {
        uint fg;
        GetWindowThreadProcessId(GetForegroundWindow(), out fg);
        return fg == (uint)pid;
    }

    public static string Q(string s) { return s == null ? "(null)" : "\"" + s.Replace("\r", "\\r").Replace("\n", "\\n").Replace("\t", "\\t") + "\""; }

    static bool Ours(object sender)
    {
        try
        {
            var el = sender as AutomationElement;
            return el != null && el.Current.ProcessId == listenPid;
        }
        catch (Exception) { return false; }
    }

    static void Add(string kind, string text)
    {
        lock (Events) Events.Add(string.Format("{0,6} ms  {1,-12} {2}", Clock.ElapsedMilliseconds, kind, text));
    }

    static Thread poller;
    static volatile bool polling;

    /// Watches the window's live-region messages. Managed UI Automation
    /// cannot subscribe to UIA_LiveRegionChangedEventId (it predates it),
    /// so the report polls every 50 ms for new Text elements: in this GUI
    /// the only Text elements screen readers see are the announcer's
    /// messages, each a new element with its live setting (AccessKit raises
    /// LiveRegionChanged when such a node appears).
    public static void StartLivePolling(AutomationElement window)
    {
        polling = true;
        var seen = new HashSet<string>();
        poller = new Thread(delegate ()
        {
            var cond = new PropertyCondition(AutomationElement.ControlTypeProperty, ControlType.Text);
            while (polling)
            {
                try
                {
                    foreach (AutomationElement t in window.FindAll(TreeScope.Descendants, cond))
                    {
                        string id = string.Join(".", Array.ConvertAll(t.GetRuntimeId(), x => x.ToString()));
                        if (seen.Add(id))
                        {
                            Add("live-region", Q(t.Current.Name) + LiveOf(t));
                        }
                    }
                }
                catch (Exception) { }
                Thread.Sleep(50);
            }
        });
        poller.IsBackground = true;
        poller.Start();
    }

    public static void StopLivePolling() { polling = false; if (poller != null) poller.Join(2000); }

    static string LiveOf(AutomationElement t) { return ""; }

    static void OnNotification(object sender, AutomationEventArgs e)
    {
        var n = e as NotificationEventArgs;
        if (n == null || !Ours(sender)) return;
        Add("notification", string.Format("[{0}, {1}] {2}", n.NotificationKind, n.NotificationProcessing, Q(n.DisplayString)));
    }

    static void OnSelection(object sender, AutomationEventArgs e)
    {
        if (!Ours(sender)) return;
        Add("text-sel", "");
    }

    /// Listens desktop-wide before the GUI starts, so no startup event is
    /// missed.
    public static void Listen()
    {
        Automation.AddAutomationEventHandler(AutomationElement.NotificationEvent, AutomationElement.RootElement,
            TreeScope.Subtree, OnNotification);
    }

    public static void ListenFor(int pid) { lock (Events) Events.Clear(); listenPid = pid; }

    public static string[] TakeEvents() { lock (Events) return Events.ToArray(); }

    public static string Patterns(AutomationElement e)
    {
        var names = new List<string>();
        foreach (var p in e.GetSupportedPatterns())
            names.Add(p.ProgrammaticName.Replace("PatternIdentifiers.Pattern", ""));
        return string.Join(",", names.ToArray());
    }

    public static string Describe(AutomationElement e)
    {
        var c = e.Current;
        string accel = string.IsNullOrEmpty(c.AcceleratorKey) ? "" : " accel=" + Q(c.AcceleratorKey);
        string help = string.IsNullOrEmpty(c.HelpText) ? "" : " help=" + Q(c.HelpText);
        string focus = c.HasKeyboardFocus ? " FOCUSED" : (c.IsKeyboardFocusable ? " focusable" : "");
        string off = c.IsOffscreen ? " offscreen" : "";
        string live = "";
        try
        {
            var prop = AutomationProperty.LookupById(30135); // UIA_LiveSettingPropertyId
            object lv = prop == null ? null : e.GetCurrentPropertyValue(prop, true);
            if (lv is int && (int)lv != 0) live = " live=" + ((int)lv == 1 ? "polite" : "assertive");
        }
        catch (Exception) { }
        return string.Format("{0} name={1}{2}{3}{4}{5}{6} patterns=[{7}]",
            c.ControlType.ProgrammaticName.Replace("ControlType.", ""), Q(c.Name),
            accel, help, focus, off, live, Patterns(e));
    }

    /// The control view, depth first, text runs left out (they are the
    /// document's text, reported through TextPattern).
    public static List<string> Tree(AutomationElement root)
    {
        var lines = new List<string>();
        Walk(TreeWalker.ControlViewWalker, root, 0, lines);
        return lines;
    }

    static void Walk(TreeWalker w, AutomationElement e, int depth, List<string> lines)
    {
        if (depth > 12 || lines.Count > 400) return;
        lines.Add(new string(' ', depth * 2) + "- " + Describe(e));
        if (e.Current.ControlType == ControlType.Document) return;
        var child = w.GetFirstChild(e);
        while (child != null)
        {
            Walk(w, child, depth + 1, lines);
            child = w.GetNextSibling(child);
        }
    }

    /// The first element of `type` (and `name`, unless it is empty; from
    /// PowerShell, $null arrives as an empty string).
    public static AutomationElement Find(AutomationElement root, ControlType type, string name)
    {
        Condition c = new PropertyCondition(AutomationElement.ControlTypeProperty, type);
        if (!string.IsNullOrEmpty(name))
            c = new AndCondition(c, new PropertyCondition(AutomationElement.NameProperty, name));
        return root.FindFirst(TreeScope.Descendants, c);
    }

    public static bool HasText(AutomationElement e)
    {
        if (e == null) return false;
        foreach (var p in e.GetSupportedPatterns())
            if (p.Id == TextPattern.Pattern.Id) return true;
        return false;
    }

    public static bool Press(AutomationElement root, string name)
    {
        var b = Find(root, ControlType.Button, name);
        if (b == null) return false;
        ((InvokePattern)b.GetCurrentPattern(InvokePattern.Pattern)).Invoke();
        return true;
    }

    /// Caret offset (UTF-16 units), the selected text, the word at the
    /// caret, its background colour, and its line.
    public static string Caret(AutomationElement doc)
    {
        // While reading, the spoken word's runs are replaced as it moves,
        // so a range can go stale between two calls: try again.
        for (int i = 0; ; i++)
        {
            try { return CaretOnce(doc); }
            catch (ElementNotAvailableException) { if (i >= 5) return "(the text changed while it was read: 6 tries)"; }
            Thread.Sleep(20);
        }
    }

    static string CaretOnce(AutomationElement doc)
    {
        var tp = (TextPattern)doc.GetCurrentPattern(TextPattern.Pattern);
        var sel = tp.GetSelection();
        if (sel.Length == 0) return "no selection";
        var before = tp.DocumentRange.Clone();
        before.MoveEndpointByRange(TextPatternRangeEndpoint.End, sel[0], TextPatternRangeEndpoint.Start);
        int offset = before.GetText(-1).Length;
        string text = sel[0].GetText(80);
        var word = sel[0].Clone();
        word.ExpandToEnclosingUnit(TextUnit.Word);
        string wordText = word.GetText(40).Trim();
        object bg = word.GetAttributeValue(TextPattern.BackgroundColorAttribute);
        // UIA colours are 0x00BBGGRR; show them as #rrggbb.
        string bgText = bg is int ? string.Format("#{0:x2}{1:x2}{2:x2}", (int)bg & 0xff, ((int)bg >> 8) & 0xff, ((int)bg >> 16) & 0xff) : (bg == TextPattern.MixedAttributeValue ? "mixed" : "none");
        var line = sel[0].Clone();
        line.ExpandToEnclosingUnit(TextUnit.Line);
        string lineText = line.GetText(60).TrimEnd('\r', '\n');
        return string.Format("offset {0,4}  selected {1,-10}  word {2,-14} background {3,-8} line {4}",
            offset, Q(text), Q(wordText), bgText, Q(lineText));
    }

    public static string DocumentText(AutomationElement doc)
    {
        var tp = (TextPattern)doc.GetCurrentPattern(TextPattern.Pattern);
        return tp.DocumentRange.GetText(-1);
    }

    /// Puts the caret at the start of paragraph `n` (0-based; negative
    /// counts from the end) through TextPattern.
    public static void PutCaretAtParagraph(AutomationElement doc, int n)
    {
        var tp = (TextPattern)doc.GetCurrentPattern(TextPattern.Pattern);
        var r = tp.DocumentRange.Clone();
        if (n >= 0)
            r.MoveEndpointByRange(TextPatternRangeEndpoint.End, r, TextPatternRangeEndpoint.Start);
        else
            r.MoveEndpointByRange(TextPatternRangeEndpoint.Start, r, TextPatternRangeEndpoint.End);
        r.Move(TextUnit.Paragraph, n);
        r.MoveEndpointByRange(TextPatternRangeEndpoint.End, r, TextPatternRangeEndpoint.Start);
        r.Select();
    }

    /// Lines of the document as UIA's line unit gives them (the first `n`).
    public static List<string> Lines(AutomationElement doc, int n)
    {
        var tp = (TextPattern)doc.GetCurrentPattern(TextPattern.Pattern);
        var r = tp.DocumentRange.Clone();
        r.MoveEndpointByRange(TextPatternRangeEndpoint.End, r, TextPatternRangeEndpoint.Start);
        var lines = new List<string>();
        for (int i = 0; i < n; i++)
        {
            var l = r.Clone();
            l.ExpandToEnclosingUnit(TextUnit.Line);
            lines.Add(Q(l.GetText(100)));
            if (r.Move(TextUnit.Line, 1) == 0) break;
        }
        return lines;
    }
}
'@
Add-Type -TypeDefinition $source -ReferencedAssemblies UIAutomationClient, UIAutomationTypes, WindowsBase

$report = New-Object System.Collections.Generic.List[string]
function Say([string] $line) { $report.Add($line) }
function Fence([string[]] $lines) { Say '```'; foreach ($l in $lines) { Say $l }; Say '```'; Say '' }

$date = (Get-Date).ToString('dddd, MMMM d, yyyy HH:mm')
Say "# textweaver-xilem UI Automation report"
Say ""
Say "- Generated: $date"
Say "- Windows: $([Environment]::OSVersion.VersionString)"
Say "- Executable: $Exe"
Say "- Document: $Document"
Say "- Backend: $Backend (silent: no audio output)"
Say "- Launch: --background (never activated, off screen, no taskbar button) with SW_SHOWNOACTIVATE; closed when the probe ends"
Say ""

[TwXUia]::Listen()
$scratch = Join-Path ([IO.Path]::GetTempPath()) ("tw-xuia-" + [Guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Force $scratch | Out-Null
$logFile = Join-Path $scratch 'gui.log'
$guiArgs = "`"$Document`" --backend $Backend --home `"$scratch`" --read --background --log-file `"$logFile`" --exit-after 90"
$foregroundBefore = [TwXUia]::GetForegroundWindow()
[TwXUia]::ListenFor(0)
$guiPid = [TwXUia]::LaunchInactive($Exe, $guiArgs, $repo)
[TwXUia]::ListenFor($guiPid)
$failures = New-Object System.Collections.Generic.List[string]

try {
    $window = [TwXUia]::WaitForWindow($guiPid, 30000)
    if (-not $window) { throw "The textweaver-xilem window did not appear." }
    $frame = [IntPtr]$window.Current.NativeWindowHandle
    [TwXUia]::StartLivePolling($window)
    $doc = $null
    for ($i = 0; $i -lt 60 -and -not $doc; $i++) {
        $doc = [TwXUia]::Find($window, [System.Windows.Automation.ControlType]::Document, $null)
        if (-not $doc) { Start-Sleep -Milliseconds 100 }
    }
    Start-Sleep -Milliseconds 1200   # the document opens and reading starts
    $stoleFocus = [TwXUia]::IsForeground($guiPid)
    $fgSame = ([TwXUia]::GetForegroundWindow() -eq $foregroundBefore)
    Say "- Window: $([TwXUia]::Describe($window))"
    Say "- Took the foreground: $stoleFocus; foreground window unchanged: $fgSame"
    Say ""
    if ($stoleFocus) { $failures.Add('the GUI took the foreground') }

    Say "### Controls (UIA control view; the document's text runs are left out)"
    Say ""
    Fence ([TwXUia]::Tree($window))

    Say "### Document"
    Say ""
    if (-not [TwXUia]::HasText($doc)) {
        Say "No Document control with TextPattern: the text is NOT exposed to UI Automation."
        Say ""
        $failures.Add('no Document with TextPattern')
        $doc = $null
    } else {
        Say "- $([TwXUia]::Describe($doc))"
        $all = [TwXUia]::DocumentText($doc)
        Say "- TextPattern document length: $($all.Length) UTF-16 units"
        Say "- First 120 units: $([TwXUia]::Q($all.Substring(0, [Math]::Min(120, $all.Length))))"
        $vp = $null
        if ($doc.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$vp)) {
            Say "- ValuePattern read-only: $($vp.Current.IsReadOnly)"
        }
        Say ""
        Say "The first lines, by UIA's line unit:"
        Say ""
        Fence ([TwXUia]::Lines($doc, 8))
        Say "Caret, word, and highlight while reading, every 400 ms:"
        Say ""
        $samples = @()
        for ($i = 0; $i -lt 8; $i++) { $samples += [TwXUia]::Caret($doc); Start-Sleep -Milliseconds 400 }
        Fence $samples
        $moving = ($samples | Select-Object -Unique).Count -gt 2
        if (-not $moving) { $failures.Add('the caret did not follow the reading') }
        if (-not ($samples -match 'background #')) { $failures.Add('no background colour on the spoken word') }

        Say "### Commands (buttons pressed with InvokePattern; caret moved with TextPattern)"
        Say ""
        $steps = @(
            @{ button = 'Pause'; wait = 700 },
            @{ button = 'Next sentence'; wait = 700 },
            @{ button = 'Next sentence'; wait = 700 },
            @{ button = 'Previous sentence'; wait = 700 },
            @{ button = 'Stop'; wait = 700 },
            @{ paragraph = 8; wait = 400 },
            @{ button = 'Play'; wait = 1500 },
            @{ button = 'Stop'; wait = 700 },
            @{ paragraph = -3; wait = 400 },
            @{ button = 'Play'; wait = 1500 },
            @{ button = 'Stop'; wait = 700 }
        )
        $cmdLines = @()
        foreach ($s in $steps) {
            if ($s.ContainsKey('paragraph')) {
                [TwXUia]::PutCaretAtParagraph($doc, $s.paragraph)
                $what = if ($s.paragraph -ge 0) { "caret to paragraph $($s.paragraph + 1)" } else { "caret $(-$s.paragraph) paragraphs from the end" }
            } else {
                if (-not [TwXUia]::Press($window, $s.button)) { $cmdLines += "(no button $($s.button))"; $failures.Add("no button $($s.button)"); continue }
                $what = "press $($s.button)"
            }
            Start-Sleep -Milliseconds $s.wait
            $cmdLines += ("{0,-34} -> {1}" -f $what, [TwXUia]::Caret($doc))
        }
        Fence $cmdLines
    }

    Say "### Status bar"
    Say ""
    $sb = [TwXUia]::Find($window, [System.Windows.Automation.ControlType]::StatusBar, $null)
    if ($sb) { Say "- $([TwXUia]::Describe($sb))" } else { Say "No status bar."; $failures.Add('no status bar') }
    Say ""

    Start-Sleep -Milliseconds 300
    [TwXUia]::StopLivePolling()
    [TwXUia]::Close($frame)
} finally {
    $p = Get-Process -Id $guiPid -ErrorAction SilentlyContinue
    if ($p -and -not $p.WaitForExit(15000)) {
        Stop-Process -Id $guiPid -Force
        Say "(the GUI did not exit when closed and was stopped)"
        $failures.Add('did not exit when closed')
    }
}

Say "### Live-region messages (new live Text elements, polled every 50 ms) and notification events"
Say ""
$events = [TwXUia]::TakeEvents()
if ($events.Count -eq 0) { $events = @('(none)'); $failures.Add('no live-region events') }
Fence $events
Say "### GUI log (announcements, commands, load timing)"
Say ""
if (Test-Path $logFile) { Fence (Get-Content $logFile) } else { Fence @('(no log)') }
Remove-Item -Recurse -Force $scratch -ErrorAction SilentlyContinue

Say "### Result"
Say ""
if ($failures.Count -eq 0) { Say "PASS: every check passed." } else { Say "FAIL:"; foreach ($f in $failures) { Say "- $f" } }

try { [System.Windows.Automation.Automation]::RemoveAllEventHandlers() } catch { }
$text = $report -join "`n"
if ($Out) { Set-Content -Path $Out -Value $text -Encoding UTF8 }
$text
if ($failures.Count -gt 0) { exit 1 }
