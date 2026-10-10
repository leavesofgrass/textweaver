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
  report checks UIA only. The menu bar is native (a Win32 menu, W6a6): the
  report checks its seven menus and their access keys, and the items the
  GUI read back from it, each with its key after a tab (AcceleratorKey).

  - The document: a Document control with TextPattern; its text, its
    selection (the caret) sampled every 400 ms while the silent `paced`
    backend reads, the background colour attribute at the caret (the
    spoken-word highlight), and caret moves made through TextPattern (as a
    screen reader's review cursor or a click would), which the app follows.
  - Buttons are pressed with InvokePattern; no keys are typed, no focus is
    needed.
  - Announcements, the two ways the GUI offers (-Announce, passed on as
    --announce). With `live` (the default) they are UIA LiveRegionChanged
    events from the live region's message nodes (AccessKit raises them; the
    report prints each new element's name and live setting). With `uia` the
    GUI raises a UIA Notification event for each message and keeps the
    message nodes with their live setting off; the report subscribes to
    Notification events, and checks that each one arrived and that no
    message node is live.
  - A long list: the Font button opens the font family list (every
    installed family, at least 40 on Windows). Every option must be in the
    tree, not only those in view; the report scrolls the last one into view
    with ScrollItemPattern and checks the whole list again.
  - The GUI starts with --background (never activated, off screen, no
    taskbar button) and SW_SHOWNOACTIVATE; the report checks the foreground
    window did not change. It plays no audio (`paced` times words like an
    engine and outputs nothing) and is closed when the probe ends.

.EXAMPLE
  powershell -File crates/textweaver-xilem/tools/uia-report.ps1 -Out uia-xilem.md

  - With -WindowEdge, instead of the commands, the settings, and the list:
    a generated document three windows long, read at 900 words per minute
    from just before the point where the GUI's document window slides. The
    caret must stay on the spoken word through the slide, the GUI's log
    must show the slide, and the Pause announcement made after it must
    arrive.

.EXAMPLE
  powershell -File crates/textweaver-xilem/tools/uia-report.ps1 -Announce uia -Out uia-notify.md

.EXAMPLE
  powershell -File crates/textweaver-xilem/tools/uia-report.ps1 -WindowEdge -Out uia-edge.md
#>
param(
    [string] $Exe = '',
    [string] $Document = '',
    [ValidateSet('paced', 'null')] [string] $Backend = 'paced',
    # How the GUI announces: a live region, or UIA Notification events.
    [ValidateSet('live', 'uia')] [string] $Announce = 'live',
    [string] $Out = '',
    # More arguments for the GUI, such as --edit-role or --select-spoken.
    [string] $GuiArgs = '',
    # Read past the document window's edge instead of the usual checks: a
    # generated document three windows long, read at 900 words per minute
    # from just before the point where the window slides (W4a2).
    [switch] $WindowEdge,
    # Also open the first menu with ExpandCollapsePattern and read its items'
    # AcceleratorKey through UI Automation. Opening a menu may bring the
    # window to the front, so this is for a CI runner, not a desktop in use.
    [switch] $Menus
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

    /// Watches the window's announcer messages by polling every 50 ms for
    /// new Text elements: in this GUI the only Text elements screen readers
    /// see are the announcer's messages, each a new element. The
    /// LiveRegionChanged events themselves are subscribed to in Listen
    /// (managed UI Automation has them since .NET Framework 4.7.1; it
    /// cannot read the LiveSetting property, which shows as unknown).
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
                            Add("message", Q(t.Current.Name) + LiveOf(t));
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

    /// The element's live setting, as " live=off", " live=polite", or
    /// " live=assertive" (UIA_LiveSettingPropertyId).
    public static string LiveOf(AutomationElement t)
    {
        int v = IntProperty(t, AutomationElementIdentifiers.LiveSettingProperty);
        return v < 0 ? " live=(unknown)" : (v == 0 ? " live=off" : (v == 1 ? " live=polite" : " live=assertive"));
    }

    static void OnNotification(object sender, AutomationEventArgs e)
    {
        var n = e as NotificationEventArgs;
        if (n == null || !Ours(sender)) return;
        Add("notification", string.Format("[{0}, {1}, activity {2}] {3}", n.NotificationKind,
            n.NotificationProcessing, Q(n.ActivityId), Q(n.DisplayString)));
    }

    static void OnLiveRegion(object sender, AutomationEventArgs e)
    {
        if (!Ours(sender)) return;
        string name = "(unknown)";
        try { name = Q(((AutomationElement)sender).Current.Name); } catch (Exception) { }
        Add("live-changed", name);
    }

    /// An integer (or enum) property, such as SizeOfSet or LiveSetting
    /// (managed UIA has them since .NET Framework 4.7.1 and 4.8), or -1.
    public static int IntProperty(AutomationElement e, AutomationProperty prop)
    {
        try
        {
            object v = e.GetCurrentPropertyValue(prop, true);
            if (v is int) return (int)v;
            if (v is Enum) return Convert.ToInt32(v);
            return -1;
        }
        catch (Exception) { return -1; }
    }

    /// The list's options as a screen reader's object navigation finds
    /// them (every ListItem under the list).
    public static AutomationElement[] Options(AutomationElement list)
    {
        var found = list.FindAll(TreeScope.Children,
            new PropertyCondition(AutomationElement.ControlTypeProperty, ControlType.ListItem));
        var items = new AutomationElement[found.Count];
        found.CopyTo(items, 0);
        return items;
    }

    /// One option: its name, position, selection, and whether UIA says it
    /// is off screen.
    public static string OptionLine(AutomationElement o)
    {
        object p;
        bool selected = o.TryGetCurrentPattern(SelectionItemPattern.Pattern, out p)
            && ((SelectionItemPattern)p).Current.IsSelected;
        return string.Format("{0}, {1} of {2}{3}{4}", Q(o.Current.Name),
            IntProperty(o, AutomationElement.PositionInSetProperty), IntProperty(o, AutomationElement.SizeOfSetProperty),
            selected ? ", selected" : "", o.Current.IsOffscreen ? ", offscreen" : "");
    }

    /// SizeOfSet: how many options the list says it has.
    public static int SizeOfSet(AutomationElement o)
    {
        return IntProperty(o, AutomationElement.SizeOfSetProperty);
    }

    /// Scrolls an option into view with ScrollItemPattern (a screen
    /// reader's object navigation does this).
    public static bool ScrollIntoView(AutomationElement o)
    {
        object p;
        if (!o.TryGetCurrentPattern(ScrollItemPattern.Pattern, out p)) return false;
        ((ScrollItemPattern)p).ScrollIntoView();
        return true;
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
        // LiveRegionChanged, where managed UIA supports it (.NET Framework
        // 4.7.1 and later); the message polling stays as the fallback.
        try
        {
            Automation.AddAutomationEventHandler(AutomationElementIdentifiers.LiveRegionChangedEvent,
                AutomationElement.RootElement, TreeScope.Subtree, OnLiveRegion);
        }
        catch (Exception) { }
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
        int lv = IntProperty(e, AutomationElementIdentifiers.LiveSettingProperty);
        string live = lv > 0 ? " live=" + (lv == 1 ? "polite" : "assertive") : "";
        return string.Format("{0} name={1}{2}{3}{4}{5}{6} patterns=[{7}]",
            c.ControlType.ProgrammaticName.Replace("ControlType.", ""), Q(c.Name),
            accel, help, focus, off, live, Patterns(e));
    }

    /// The window's menu bar (W6a6): the MenuBar whose items are the menus
    /// (the system menu's bar has one item, System), each item's name and
    /// access key, read without opening a menu.
    public static List<string> MenuBar(AutomationElement window)
    {
        var lines = new List<string>();
        var bars = window.FindAll(TreeScope.Children,
            new PropertyCondition(AutomationElement.ControlTypeProperty, ControlType.MenuBar));
        foreach (AutomationElement bar in bars)
        {
            var items = bar.FindAll(TreeScope.Children,
                new PropertyCondition(AutomationElement.ControlTypeProperty, ControlType.MenuItem));
            if (items.Count < 2) continue;
            foreach (AutomationElement it in items)
                lines.Add("MenuItem " + Q(it.Current.Name) + " access key " + Q(it.Current.AccessKey));
        }
        return lines;
    }

    /// With -Menus: opens the menu bar item `name` with ExpandCollapsePattern,
    /// lists the Menu's items with their AcceleratorKey, and closes it.
    public static List<string> OpenMenu(AutomationElement window, string name)
    {
        var lines = new List<string>();
        var item = window.FindFirst(TreeScope.Descendants, new AndCondition(
            new PropertyCondition(AutomationElement.ControlTypeProperty, ControlType.MenuItem),
            new PropertyCondition(AutomationElement.NameProperty, name)));
        object p;
        if (item == null || !item.TryGetCurrentPattern(ExpandCollapsePattern.Pattern, out p))
        {
            lines.Add("(no menu " + Q(name) + " to open)");
            return lines;
        }
        var ec = (ExpandCollapsePattern)p;
        ec.Expand();
        Thread.Sleep(600);
        var menus = AutomationElement.RootElement.FindAll(TreeScope.Children,
            new PropertyCondition(AutomationElement.ControlTypeProperty, ControlType.Menu));
        foreach (AutomationElement m in menus)
        {
            lines.Add("Menu " + Q(m.Current.Name));
            var items = m.FindAll(TreeScope.Children,
                new PropertyCondition(AutomationElement.ControlTypeProperty, ControlType.MenuItem));
            foreach (AutomationElement it in items)
                lines.Add("  MenuItem " + Q(it.Current.Name) + " accelerator " + Q(it.Current.AcceleratorKey)
                    + " access key " + Q(it.Current.AccessKey));
        }
        try { ec.Collapse(); } catch (Exception) { }
        return lines;
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

    /// Presses the first button whose name starts with `prefix` ("Settings"
    /// finds "Settings..." with its ellipsis character).
    public static bool PressStartingWith(AutomationElement root, string prefix)
    {
        var cond = new PropertyCondition(AutomationElement.ControlTypeProperty, ControlType.Button);
        foreach (AutomationElement b in root.FindAll(TreeScope.Descendants, cond))
        {
            if (b.Current.Name.StartsWith(prefix, StringComparison.Ordinal))
            {
                ((InvokePattern)b.GetCurrentPattern(InvokePattern.Pattern)).Invoke();
                return true;
            }
        }
        return false;
    }

    /// The first element named `name`, of any type.
    public static AutomationElement Named(AutomationElement root, string name)
    {
        return root.FindFirst(TreeScope.Descendants, new PropertyCondition(AutomationElement.NameProperty, name));
    }

    /// Every element of `type` under `root`, described.
    public static List<string> All(AutomationElement root, ControlType type)
    {
        var lines = new List<string>();
        foreach (AutomationElement e in root.FindAll(TreeScope.Descendants,
            new PropertyCondition(AutomationElement.ControlTypeProperty, type)))
            lines.Add(Describe(e) + ValueOf(e));
        return lines;
    }

    /// The value a screen reader reads: RangeValue, Toggle, or Value.
    public static string ValueOf(AutomationElement e)
    {
        object p;
        if (e.TryGetCurrentPattern(RangeValuePattern.Pattern, out p))
        {
            var r = ((RangeValuePattern)p).Current;
            return string.Format(" range={0} ({1} to {2}, step {3})", r.Value, r.Minimum, r.Maximum, r.SmallChange);
        }
        if (e.TryGetCurrentPattern(TogglePattern.Pattern, out p))
            return " toggle=" + ((TogglePattern)p).Current.ToggleState;
        if (e.TryGetCurrentPattern(ValuePattern.Pattern, out p))
            return " value=" + Q(((ValuePattern)p).Current.Value);
        return "";
    }

    /// Sets a slider through RangeValuePattern.
    public static void SetRange(AutomationElement e, double v)
    {
        ((RangeValuePattern)e.GetCurrentPattern(RangeValuePattern.Pattern)).SetValue(v);
    }

    /// Toggles a check box through TogglePattern.
    public static void Toggle(AutomationElement e)
    {
        ((TogglePattern)e.GetCurrentPattern(TogglePattern.Pattern)).Toggle();
    }

    /// Presses the button named `name` (or, for older builds, `name`
    /// followed by its key, as in "Play, Space").
    public static bool Press(AutomationElement root, string name)
    {
        var b = Find(root, ControlType.Button, name);
        if (b == null)
        {
            var cond = new PropertyCondition(AutomationElement.ControlTypeProperty, ControlType.Button);
            foreach (AutomationElement e in root.FindAll(TreeScope.Descendants, cond))
            {
                if (e.Current.Name.StartsWith(name + ", ", StringComparison.Ordinal)) { b = e; break; }
            }
        }
        if (b == null) return false;
        ((InvokePattern)b.GetCurrentPattern(InvokePattern.Pattern)).Invoke();
        return true;
    }

    /// Buttons with no AcceleratorKey (their keyboard shortcut from the
    /// keymap), or whose name carries the key too.
    public static List<string> WithoutKeys(AutomationElement root)
    {
        var missing = new List<string>();
        var cond = new PropertyCondition(AutomationElement.ControlTypeProperty, ControlType.Button);
        foreach (AutomationElement e in root.FindAll(TreeScope.Descendants, cond))
        {
            var n = e.Current.Name;
            // The window frame's own buttons are the system's.
            if (n == "Minimize" || n == "Maximize" || n == "Restore" || n == "Close" || n == "System") continue;
            var key = e.Current.AcceleratorKey;
            if (string.IsNullOrEmpty(key)) missing.Add(n + " (no AcceleratorKey)");
            else if (n.Contains(", ")) missing.Add(n + " (the key is in the name)");
        }
        return missing;
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
        // UIA's word unit takes in the spaces after the word, which the
        // highlight does not cover ("mixed"): read the colour of the word
        // alone.
        string raw = word.GetText(40);
        int trailing = raw.Length - raw.TrimEnd().Length;
        if (trailing > 0 && trailing < raw.Length)
            word.MoveEndpointByUnit(TextPatternRangeEndpoint.End, TextUnit.Character, -trailing);
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

    /// Puts the caret `units` characters before the end of the document's
    /// text (the GUI's window) through TextPattern.
    public static void PutCaretBeforeEnd(AutomationElement doc, int units)
    {
        var tp = (TextPattern)doc.GetCurrentPattern(TextPattern.Pattern);
        var r = tp.DocumentRange.Clone();
        r.MoveEndpointByRange(TextPatternRangeEndpoint.Start, r, TextPatternRangeEndpoint.End);
        r.Move(TextUnit.Character, -units);
        r.MoveEndpointByRange(TextPatternRangeEndpoint.End, r, TextPatternRangeEndpoint.Start);
        r.Select();
    }

    /// The first `n` units of the document's text: where the window starts.
    public static string Head(AutomationElement doc, int n)
    {
        for (int i = 0; ; i++)
        {
            try
            {
                var tp = (TextPattern)doc.GetCurrentPattern(TextPattern.Pattern);
                // The provider may return more than asked for.
                string s = tp.DocumentRange.GetText(n);
                return s.Length > n ? s.Substring(0, n) : s;
            }
            catch (ElementNotAvailableException) { if (i >= 5) return "(unavailable)"; }
            Thread.Sleep(20);
        }
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
Say "- Announcements: --announce $Announce ($(if ($Announce -eq 'uia') { 'UIA Notification events' } else { 'a live region' }))"
Say "- Extra GUI arguments: $(if ($GuiArgs) { $GuiArgs } else { '(none)' })"
Say "- Launch: --background (never activated, off screen, no taskbar button) with SW_SHOWNOACTIVATE; closed when the probe ends"
Say ""

[TwXUia]::Listen()
# The GUI's home for the run, under the repository's own ignored `target`
# folder (never the system's temporary folder, which is on the system
# drive).
$scratch = Join-Path (Join-Path $repo 'target') ("tw-xuia-" + [Guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Force $scratch | Out-Null
# The GUI's temporary files go there too. (A debug build of Masonry wrote
# a full trace log to the temporary folder on every start; since W7x it
# writes one only where MASONRY_DENSE_LOG_DIR points.)
$env:TMP = $scratch
$env:TEMP = $scratch
$logFile = Join-Path $scratch 'gui.log'
# The menu bar is hidden by default on Windows; show it so the menus can be read.
New-Item -ItemType Directory -Force (Join-Path $scratch 'config') | Out-Null
[IO.File]::WriteAllText((Join-Path $scratch 'config\settings.toml'), "[gui]`nauto_hide_menu = false`n")
if ($WindowEdge) {
    # Three GUI windows (120,000 units each) of plain paragraphs, about 240
    # characters each, and the fastest rate, so the paced reading reaches
    # the slide point in seconds.
    $edgeDoc = Join-Path $scratch 'long.md'
    $sentence = 'Reading on and on, sentence after sentence, past the edge of the window. '
    $sb = New-Object System.Text.StringBuilder
    $n = 0
    while ($sb.Length -lt 360000) {
        $n++
        [void]$sb.Append("Paragraph $n. ").Append($sentence).Append($sentence).Append($sentence).Append("`n`n")
    }
    [IO.File]::WriteAllText($edgeDoc, $sb.ToString())
    $Document = $edgeDoc
    New-Item -ItemType Directory -Force (Join-Path $scratch 'config') | Out-Null
    [IO.File]::WriteAllText((Join-Path $scratch 'config\settings.toml'), "[speech]`nrate = 900`n`n[gui]`nauto_hide_menu = false`n")
    Say "- Window edge probe: $Document ($($sb.Length) characters, $n paragraphs), read at 900 words per minute"
    Say ""
}
$guiArgs = "`"$Document`" --backend $Backend --home `"$scratch`" --read --background --log-file `"$logFile`" --exit-after 90 --announce $Announce $GuiArgs"
$foregroundBefore = [TwXUia]::GetForegroundWindow()
[TwXUia]::ListenFor(0)
$guiPid = [TwXUia]::LaunchInactive($Exe, $guiArgs, $repo)
[TwXUia]::ListenFor($guiPid)
$failures = New-Object System.Collections.Generic.List[string]
# Known problems that are not failures: an activation by UI Automation
# that the window gave back at once (W7x). The window still being in front
# when checked (W6a6's finding) is a failure again.
$warnings = New-Object System.Collections.Generic.List[string]
$launchedAt = Get-Date
# Held from the start, so the exit code can still be read if the GUI ends
# early; reading Handle now keeps the process handle open.
$guiProc = Get-Process -Id $guiPid -ErrorAction SilentlyContinue
if ($guiProc) { $null = $guiProc.Handle }

try {
    $window = [TwXUia]::WaitForWindow($guiPid, 30000)
    if (-not $window) { throw "The textweaver-xilem window did not appear." }
    $frame = [IntPtr]$window.Current.NativeWindowHandle
    [TwXUia]::StartLivePolling($window)
    $doc = $null
    for ($i = 0; $i -lt 60 -and -not $doc; $i++) {
        $doc = [TwXUia]::Find($window, [System.Windows.Automation.ControlType]::Document, $null)
        # With --edit-role the document is a read-only Edit.
        if (-not $doc) { $doc = [TwXUia]::Find($window, [System.Windows.Automation.ControlType]::Edit, 'Document') }
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
    # Every button has its key from the keymap as its AcceleratorKey, and a
    # name that is its label only.
    $noKeys = [TwXUia]::WithoutKeys($window)
    if ($noKeys.Count -gt 0) {
        Say "- Buttons without their key as AcceleratorKey: $($noKeys -join '; ')"
        $failures.Add('a button has no AcceleratorKey')
    } else {
        Say "- Every button has its key as its AcceleratorKey, and its label as its name."
    }
    Say ""

    # W6a6: the native menu bar, from the app's menu model. Its menus are
    # read without opening them (opening one would need the foreground);
    # their items are checked from what the GUI read back from its menu
    # (the log, below). -Menus opens File to read its items through UI
    # Automation too, on a machine where the foreground may move.
    Say "### Menus (the native menu bar)"
    Say ""
    $bar = @([TwXUia]::MenuBar($window))
    if ($bar.Count -eq 0) { Fence @('(no menu bar)') } else { Fence $bar }
    if ($bar.Count -ne 7) { $failures.Add("the menu bar has $($bar.Count) menus, not 7") }
    $noAccess = @($bar | Where-Object { $_ -notmatch 'access key "Alt\+' })
    if ($noAccess.Count -gt 0) { $failures.Add("a menu has no access key: $($noAccess[0])") }
    if ($Menus) {
        $fileName = if ($bar.Count -gt 0) { ([regex]::Match($bar[0], '^MenuItem "([^"]*)"')).Groups[1].Value } else { 'File' }
        $opened = @([TwXUia]::OpenMenu($window, $fileName))
        Say "The first menu, opened with ExpandCollapsePattern:"
        Say ""
        Fence $opened
        if (-not ($opened -match '^Menu ')) { $failures.Add('opening the first menu showed no Menu') }
        if (-not ($opened -match 'accelerator "[^"]+"')) { $failures.Add('no menu item has an AcceleratorKey') }
    }

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
    }

    if ($doc -and $WindowEdge) {
        Say "### Reading past the window's edge (caret put 15,600 units before the window's end with TextPattern; Play pressed)"
        Say ""
        [void][TwXUia]::Press($window, 'Stop')
        Start-Sleep -Milliseconds 500
        $headBefore = [TwXUia]::Head($doc, 40)
        $lengthBefore = ([TwXUia]::DocumentText($doc)).Length
        [TwXUia]::PutCaretBeforeEnd($doc, 15600)
        Start-Sleep -Milliseconds 500
        $startCaret = [TwXUia]::Caret($doc)
        Say "- Before: the window starts $([TwXUia]::Q($headBefore)), $lengthBefore units; caret: $startCaret"
        [void][TwXUia]::Press($window, 'Play')
        $edgeLines = @()
        $slidAt = -1
        for ($i = 0; $i -lt 60; $i++) {
            Start-Sleep -Milliseconds 300
            $head = [TwXUia]::Head($doc, 40)
            $caret = [TwXUia]::Caret($doc)
            $mark = if ($head -ne $headBefore) { 'slid' } else { 'same' }
            $edgeLines += ("{0,5} ms  window {1}  {2}" -f (($i + 1) * 300), $mark, $caret)
            if ($mark -eq 'slid' -and $slidAt -lt 0) { $slidAt = $i }
            # Five more samples after the slide.
            if ($slidAt -ge 0 -and $i -ge $slidAt + 5) { break }
        }
        Fence $edgeLines
        if ($slidAt -lt 0) {
            $failures.Add('the window did not slide while reading past its edge')
        } else {
            $after = $edgeLines[$slidAt..($edgeLines.Count - 1)]
            $lost = @($after | Where-Object { $_ -match 'no selection|the text changed' })
            if ($lost.Count -gt 0) { $failures.Add("the caret was lost after the slide: $($lost[0])") }
            if (-not ($after -match 'word "[A-Za-z]')) { $failures.Add('no word at the caret after the slide') }
            if (($after | Select-Object -Unique).Count -lt 3) { $failures.Add('the caret did not keep following the reading after the slide') }
            $headAfter = [TwXUia]::Head($doc, 40)
            $lengthAfter = ([TwXUia]::DocumentText($doc)).Length
            Say "- After: the window starts $([TwXUia]::Q($headAfter)), $lengthAfter units"
            # An announcement after the slide still arrives.
            [void][TwXUia]::Press($window, 'Pause')
            Start-Sleep -Milliseconds 800
            Say "- Pause pressed after the slide: caret $([TwXUia]::Caret($doc))"
            [void][TwXUia]::Press($window, 'Stop')
            Start-Sleep -Milliseconds 500
        }
        Say ""
    }

    if ($doc -and -not $WindowEdge) {
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

    # The edge probe checks only the document and the announcements.
    if (-not $WindowEdge) {
        Say "### Settings dialog (Settings button pressed; settings changed with RangeValuePattern and TogglePattern)"
        Say ""
    }
    if ($WindowEdge) {
    } elseif (-not [TwXUia]::PressStartingWith($window, 'Settings')) {
        Say "No Settings button."
        $failures.Add('no Settings button')
    } else {
        Start-Sleep -Milliseconds 800
        $dlg = [TwXUia]::Named($window, 'Settings')
        $form = [TwXUia]::Named($window, 'Speech settings')
        if (-not $dlg) { $failures.Add('no Settings dialog') } else { Say "- Dialog: $([TwXUia]::Describe($dlg))" }
        $sections = [TwXUia]::Find($window, [System.Windows.Automation.ControlType]::List, 'Sections')
        if ($sections) {
            Say "- Sections: $([TwXUia]::Describe($sections))"
            $items = [TwXUia]::All($sections, [System.Windows.Automation.ControlType]::ListItem)
            Say "- $($items.Count) sections:"
            Say ""
            Fence $items
            if ($items.Count -lt 4) { $failures.Add('too few settings sections') }
        } else { $failures.Add('no Sections list') }
        if ($form) { Say "- Form: $([TwXUia]::Describe($form))" } else { $failures.Add('no settings form') }
        Say ""
        Say "The settings a screen reader finds in the first section:"
        Say ""
        $setLines = @()
        foreach ($t in @('Slider', 'CheckBox', 'ComboBox', 'Edit')) {
            $ct = [System.Windows.Automation.ControlType]::$t
            $setLines += [TwXUia]::All($window, $ct)
        }
        Fence $setLines
        $rate = [TwXUia]::Find($window, [System.Windows.Automation.ControlType]::Slider, 'Rate')
        if ($rate) {
            $before = [TwXUia]::ValueOf($rate)
            [TwXUia]::SetRange($rate, 300)
            Start-Sleep -Milliseconds 500
            $rate = [TwXUia]::Find($window, [System.Windows.Automation.ControlType]::Slider, 'Rate')
            $after = [TwXUia]::ValueOf($rate)
            Say "- Rate set to 300 through RangeValuePattern: before$before; after$after"
            if ($after -notmatch 'range=300 ') { $failures.Add('the Rate slider did not take a value set through RangeValuePattern') }
        } else { $failures.Add('no Rate slider') }
        $box = [TwXUia]::Find($window, [System.Windows.Automation.ControlType]::CheckBox, $null)
        if ($box) {
            $name = $box.Current.Name
            $before = [TwXUia]::ValueOf($box)
            [TwXUia]::Toggle($box)
            Start-Sleep -Milliseconds 500
            $box = [TwXUia]::Find($window, [System.Windows.Automation.ControlType]::CheckBox, $name)
            $after = [TwXUia]::ValueOf($box)
            Say "- $name toggled through TogglePattern: before$before; after$after"
            if ($before -eq $after) { $failures.Add('a settings check box did not toggle through TogglePattern') }
            # Back as it was.
            [TwXUia]::Toggle($box)
            Start-Sleep -Milliseconds 300
        } else { $failures.Add('no settings check box') }
        # The dialog's own Close button (the window frame has one too).
        if (-not ($dlg -and [TwXUia]::Press($dlg, 'Close'))) { $failures.Add('no Close button in the settings') }
        Start-Sleep -Milliseconds 500
        if ([TwXUia]::Named($window, 'Sections')) { $failures.Add('the settings dialog did not close') }
        else { Say "- Close pressed: the dialog is gone." }
        Say ""
    }

    Say "### Status bar"
    Say ""
    $sb = [TwXUia]::Find($window, [System.Windows.Automation.ControlType]::StatusBar, $null)
    if ($sb) { Say "- $([TwXUia]::Describe($sb))" } else {
        Say "No status bar. The controls now:"
        Say ""
        Fence ([TwXUia]::Tree($window))
        $failures.Add('no status bar')
    }
    Say ""

    # Last, because a list dialog has no Close button (Escape closes it,
    # and the report types no keys); the window is closed with it open.
    if (-not $WindowEdge) {
        Say "### A long list (Font button; the last option scrolled into view with ScrollItemPattern)"
        Say ""
    }
    if ($WindowEdge) {
    } elseif (-not [TwXUia]::PressStartingWith($window, 'Font')) {
        Say "No Font button."
        $failures.Add('no Font button')
    } else {
        Start-Sleep -Milliseconds 800
        $list = [TwXUia]::Find($window, [System.Windows.Automation.ControlType]::List, $null)
        if (-not $list) {
            Say "No list opened."
            $failures.Add('the Font button opened no list')
        } else {
            Say "- List: $([TwXUia]::Describe($list))"
            $opts = [TwXUia]::Options($list)
            $size = if ($opts.Count -gt 0) { [TwXUia]::SizeOfSet($opts[$opts.Count - 1]) } else { 0 }
            $off = @($opts | Where-Object { $_.Current.IsOffscreen }).Count
            Say "- Before scrolling: $($opts.Count) options in the tree, of $size in the list; $off of them off screen"
            if ($opts.Count -lt 40) { $failures.Add("the list has fewer than 40 options in the tree ($($opts.Count))") }
            if ($opts.Count -ne $size) { $failures.Add("options left out of the tree before scrolling: $($opts.Count) of $size") }
            if ($opts.Count -gt 0) {
                $lastName = $opts[$opts.Count - 1].Current.Name
                $scrolled = [TwXUia]::ScrollIntoView($opts[$opts.Count - 1])
                if (-not $scrolled) { $failures.Add('the last option has no ScrollItemPattern') }
                Start-Sleep -Milliseconds 600
                $opts = [TwXUia]::Options($list)
                $off = @($opts | Where-Object { $_.Current.IsOffscreen }).Count
                Say "- After scrolling the last option ($([TwXUia]::Q($lastName))) into view: $($opts.Count) options in the tree, of $size; $off of them off screen"
                if ($opts.Count -ne $size) { $failures.Add("options left out of the tree after scrolling: $($opts.Count) of $size") }
                $sample = @()
                foreach ($i in @(0, 1, 2)) { if ($i -lt $opts.Count) { $sample += [TwXUia]::OptionLine($opts[$i]) } }
                $sample += '...'
                foreach ($i in @(3, 2, 1)) { if ($opts.Count -ge $i) { $sample += [TwXUia]::OptionLine($opts[$opts.Count - $i]) } }
                Say ""
                Say "The first and last options, as a screen reader finds them:"
                Say ""
                Fence $sample
                $last = $opts[$opts.Count - 1]
                if ([TwXUia]::OptionLine($last) -notmatch ', selected') { $failures.Add('the last option was not selected after ScrollIntoView') }
                if ($last.Current.IsOffscreen) { $failures.Add('the last option is still off screen after ScrollIntoView') }
            }
        }
    }
    Say ""

    Start-Sleep -Milliseconds 300
    [TwXUia]::StopLivePolling()
    # The dialogs, the menus, and the list must not have brought the
    # window forward either.
    $stoleLater = [TwXUia]::IsForeground($guiPid)
    Say "- Took the foreground by the end: $stoleLater"
    Say ""
    if ($stoleLater -and -not $stoleFocus) { $failures.Add('the GUI came to the foreground after buttons were pressed through UI Automation') }
    [TwXUia]::Close($frame)
} catch {
    # The probe stopped early, most often because the GUI exited. Say why,
    # then carry on so the GUI log below is still printed.
    $failures.Add("the probe stopped: $($_.Exception.Message)")
    Say ""
    Say "### The probe stopped early"
    Say ""
    Say "- Error: $($_.Exception.Message)"
    if ($guiProc -and $guiProc.HasExited) {
        $code = $guiProc.ExitCode
        Say "- The GUI had exited, exit code $code (hexadecimal 0x$('{0:X8}' -f $code)), $([int]((Get-Date) - $launchedAt).TotalMilliseconds) ms or less after launch"
    } elseif ($guiProc) {
        Say "- The GUI was still running"
    } else {
        Say "- The GUI process was not found just after launch"
    }
    $crashes = @(Get-WinEvent -FilterHashtable @{ LogName = 'Application'; StartTime = $launchedAt } -ErrorAction SilentlyContinue |
        Where-Object { $_.ProviderName -in @('Application Error', 'Windows Error Reporting', '.NET Runtime') -and $_.Message -match 'textweaver' } |
        Select-Object -First 3)
    if ($crashes.Count -gt 0) {
        Say "- Windows recorded a crash:"
        Say ""
        Fence @($crashes | ForEach-Object { ($_.Message -split "`r?`n" | Select-Object -First 12) -join "`n" })
    } else {
        Say "- Windows recorded no crash for textweaver in the Application log"
    }
    Say ""
} finally {
    $p = Get-Process -Id $guiPid -ErrorAction SilentlyContinue
    if ($p -and -not $p.WaitForExit(15000)) {
        Stop-Process -Id $guiPid -Force
        Say "(the GUI did not exit when closed and was stopped)"
        $failures.Add('did not exit when closed')
    }
}

# W6a6: the command palette, in a second, silent run (a dialog closes only
# with Escape, and the report types no keys, so the first run's font list
# would stand in front of it). The Commands button opens it; its list is
# the app's candidates, each read as its short name then its key ("Find next,
# F3"; the explanation is the row's description), and one row is the selected option.
if (-not $WindowEdge) {
    Say "### The command palette (a second run, silent; Commands button pressed; its list read with UI Automation)"
    Say ""
    $palHome = Join-Path $scratch 'palette'
    $palLog = Join-Path $scratch 'palette.log'
    $palArgs = "`"$Document`" --backend null --home `"$palHome`" --background --log-file `"$palLog`" --exit-after 60"
    $palPid = [TwXUia]::LaunchInactive($Exe, $palArgs, $repo)
    # Held from the start, so an early exit's code can be read.
    $palProc = Get-Process -Id $palPid -ErrorAction SilentlyContinue
    if ($palProc) { $null = $palProc.Handle }
    try {
        $pw = [TwXUia]::WaitForWindow($palPid, 30000)
        if (-not $pw) {
            $failures.Add('the palette run''s window did not appear')
            $code = if ($palProc -and $palProc.HasExited) { $palProc.ExitCode } else { '(running)' }
            Say "- The palette run's window did not appear; exit code: $code; arguments: $palArgs"
            if (Test-Path -LiteralPath $palLog) { Fence @(Get-Content -LiteralPath $palLog | Select-Object -Last 15) }
        } else {
            Start-Sleep -Milliseconds 1200
            $palFgStart = [TwXUia]::IsForeground($palPid)
            Say "- Took the foreground on starting: $palFgStart"
            if ($palFgStart) { $failures.Add('the palette run took the foreground on starting') }
            if (-not [TwXUia]::PressStartingWith($pw, 'Commands')) {
                Say "No Commands button."
                $failures.Add('no Commands button')
            } else {
                Start-Sleep -Milliseconds 800
                $plist = [TwXUia]::Find($pw, [System.Windows.Automation.ControlType]::List, 'Commands')
                if (-not $plist) {
                    Say "No list of commands."
                    $failures.Add('the Commands button opened no list of commands')
                } else {
                    Say "- List: $([TwXUia]::Describe($plist))"
                    $popts = [TwXUia]::Options($plist)
                    $sample = @()
                    foreach ($i in @(0, 1, 2, 3)) { if ($i -lt $popts.Count) { $sample += [TwXUia]::OptionLine($popts[$i]) } }
                    Say "- $($popts.Count) commands; the first, as a screen reader finds them:"
                    Say ""
                    Fence $sample
                    if ($popts.Count -lt 100) { $failures.Add("the palette lists only $($popts.Count) commands") }
                    $selected = @($popts | Where-Object { [TwXUia]::OptionLine($_) -match ', selected' })
                    if ($selected.Count -ne 1) { $failures.Add("the palette has $($selected.Count) selected rows, not 1") }
                    $unnamed = @($popts | Select-Object -First 20 | Where-Object { $_.Current.Name -notmatch '^[^,:]+(, [^:]+)?$' })
                    if ($unnamed.Count -gt 0) { $failures.Add("a palette row is not its short name, then its key (no explanation): $([TwXUia]::Q($unnamed[0].Current.Name))") }
                }
            }
            $palFgEnd = [TwXUia]::IsForeground($palPid)
            Say "- Took the foreground with the palette open: $palFgEnd"
            if ($palFgEnd -and -not $palFgStart) { $failures.Add('the palette run came to the foreground after the Commands button was pressed through UI Automation') }
            [TwXUia]::Close([IntPtr]$pw.Current.NativeWindowHandle)
        }
    } finally {
        $pp = Get-Process -Id $palPid -ErrorAction SilentlyContinue
        if ($pp -and -not $pp.WaitForExit(15000)) {
            Stop-Process -Id $palPid -Force
            $failures.Add('the palette run did not exit when closed')
        }
    }
    Say ""
}

Say "### Announcements: message elements (new Text elements, polled every 50 ms, with their live setting) and Notification events"
Say ""
$events = [TwXUia]::TakeEvents()
$messages = @($events | Where-Object { $_ -match '\bmessage\b' })
$liveMessages = @($messages | Where-Object { $_ -match 'live=(polite|assertive)' })
$notifications = @($events | Where-Object { $_ -match '\bnotification\b' })
$liveChanged = @($events | Where-Object { $_ -match '\blive-changed\b' })
Say "- $($messages.Count) message elements, $($liveMessages.Count) of them live; $($liveChanged.Count) LiveRegionChanged events; $($notifications.Count) Notification events"
Say ""
if ($events.Count -eq 0) { $events = @('(none)') }
Fence $events
$guiLog = if (Test-Path -LiteralPath $logFile) { @(Get-Content -LiteralPath $logFile) } else { @() }
# W6a6: the menu items as the window's menu holds them (read back with
# GetMenuStringW, which is what UI Automation's MenuItem reads): each with
# its access key (&) and, for a command with a key, the key after a tab
# (the item's AcceleratorKey).
$menuItems = @($guiLog | Where-Object { $_ -match '^menu item: ' } | ForEach-Object { $_.Substring(11) })
$withKeys = @($menuItems | Where-Object { $_ -match "`t." })
$tops = @($menuItems | ForEach-Object { ($_ -split ' > ')[0] } | Select-Object -Unique)
Say "### Menu items (read back from the window's menu)"
Say ""
Say "- $($menuItems.Count) items in $($tops.Count) menus; $($withKeys.Count) show a key after a tab (AcceleratorKey)"
Say ""
if ($menuItems.Count -gt 0) { Fence ($menuItems | Select-Object -First 30 | ForEach-Object { [TwXUia]::Q($_) }) }
if ($tops.Count -ne 7) { $failures.Add("the window's menu holds $($tops.Count) menus, not 7") }
if ($withKeys.Count -lt 20) { $failures.Add("only $($withKeys.Count) menu items show a key") }
$noAccessItem = @($menuItems | Where-Object { ($_ -split ' > ')[-1] -notmatch '&' -and $_ -notmatch "`t" -and ($_ -split ' > ')[-1] -notmatch '^\d' })
if ($noAccessItem.Count -gt 3) { $failures.Add("menu items without an access key: $($noAccessItem[0..2] -join '; ')") }
if ($WindowEdge) {
    # The window slid (the GUI's log says so), and the announcement made
    # after it (Pause) reached UI Automation.
    $slides = @($guiLog | Where-Object { $_ -match '\((Forward|Backward)' })
    Say "- Window slides in the GUI's log: $($slides.Count)"
    Say ""
    if ($slides.Count -eq 0) { $failures.Add('the GUI log shows no window slide') }
    if (-not ($events -match 'Paused')) { $failures.Add('the announcement after the slide (Paused) did not arrive') }
}
if ($Announce -eq 'uia') {
    $raisedTexts = @($guiLog | Where-Object { $_ -match '^notify uia: ' } | ForEach-Object { $_.Substring(12) })
    $notifyFailed = @($guiLog | Where-Object { $_ -match '^notify uia failed' })
    if ($notifications.Count -eq 0) { $failures.Add('no Notification events with --announce uia') }
    else {
        # Notifications raised before any UI Automation client has asked
        # for the window (at startup) reach nobody; count from the first
        # one received.
        $first = ([regex]::Match($notifications[0], '\] "(.*)"$')).Groups[1].Value
        $start = [Array]::IndexOf($raisedTexts, $first)
        if ($start -lt 0) { $start = 0 }
        $expected = $raisedTexts.Count - $start
        Say "- The GUI raised $($raisedTexts.Count) notifications; $start at startup, before the first one received: $(if ($start -gt 0) { ($raisedTexts[0..($start - 1)] | ForEach-Object { [TwXUia]::Q($_) }) -join ', ' } else { 'none' })"
        Say ""
        if ($notifications.Count -lt $expected) { $failures.Add("the GUI raised $expected notifications after the first one received, and the report received $($notifications.Count)") }
    }
    if ($notifyFailed.Count -gt 0) { $failures.Add("a notification could not be raised: $($notifyFailed[0])") }
    if ($liveMessages.Count -gt 0 -or $liveChanged.Count -gt 0) { $failures.Add('LiveRegionChanged or a live message element with --announce uia (a screen reader would hear it twice)') }
    if ($messages.Count -eq 0) { $failures.Add('no message elements in the tree') }
} else {
    if ($liveChanged.Count -eq 0 -and $liveMessages.Count -eq 0) { $failures.Add('no LiveRegionChanged events and no live message elements') }
    if ($notifications.Count -gt 0) { $failures.Add('Notification events with --announce live (a screen reader would hear messages twice)') }
}
# W7x: the window is guarded (WS_EX_NOACTIVATE). UI Automation still
# activates it on the first InvokePattern.Invoke; the window then gives the
# foreground back at once and logs it (where from, the first time). Being in
# front when checked is a failure; an activation given back is a warning.
$guarded = @($guiLog | Where-Object { $_ -match '^background: guarded \(WS_EX_NOACTIVATE, foreground given back\): yes' })
$palGuiLog = if ($palLog -and (Test-Path -LiteralPath $palLog)) { @(Get-Content -LiteralPath $palLog) } else { @() }
$bothLogs = @(@($guiLog) + @($palGuiLog))
$activated = @($bothLogs | Where-Object { $_ -match '^background: the window was activated' })
$givenBack = @($bothLogs | Where-Object { $_ -match '^background: (foreground given back|the window was in front at a tick; foreground given back): yes' })
Say "### Foreground"
Say ""
Say "- Guarded against activation (WS_EX_NOACTIVATE): $(if ($guarded.Count -gt 0) { 'yes' } else { 'no' })"
Say "- Activated by UI Automation all the same: $($activated.Count) times; foreground given back: $($givenBack.Count) times"
Say ""
if ($guarded.Count -eq 0) { $failures.Add('the window was not guarded against activation') }
if ($activated.Count -gt 0) { $warnings.Add("UI Automation activated the window $($activated.Count) times; it gave the foreground back $($givenBack.Count) times (the GUI log says where the first came from)") }
Say "### GUI log (announcements, commands, load timing)"
Say ""
if ($guiLog.Count -gt 0) { Fence $guiLog } else { Fence @('(no log)') }
# The scratch folder this run made under the temporary folder.
if ($scratch -and (Split-Path -Leaf $scratch) -like 'tw-xuia-*') {
    Remove-Item -LiteralPath $scratch -Recurse -Force -ErrorAction SilentlyContinue
}

Say "### Result"
Say ""
if ($failures.Count -eq 0) { Say "PASS: every check passed." } else { Say "FAIL:"; foreach ($f in $failures) { Say "- $f" } }
if ($warnings.Count -gt 0) { Say ""; Say "Warnings (known, not yet fixed):"; foreach ($w in $warnings) { Say "- $w" } }

try { [System.Windows.Automation.Automation]::RemoveAllEventHandlers() } catch { }
$text = $report -join "`n"
if ($Out) { Set-Content -Path $Out -Value $text -Encoding UTF8 }
$text
if ($failures.Count -gt 0) { exit 1 }
