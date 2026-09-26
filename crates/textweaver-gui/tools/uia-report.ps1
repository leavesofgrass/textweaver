<#
.SYNOPSIS
  Launches textweaver-gui out of the way and reports what assistive
  technology sees through UI Automation and MSAA: every element's role and
  name, the document's text and caret while it reads, the menus and their
  accelerators, the status bar, and the live-region notifications.

.DESCRIPTION
  The report approximates what NVDA reads:
    - NVDA reads most Win32 controls through MSAA (IAccessible): the report
      lists each window's MSAA name, role, state, and value beside UIA.
    - For the RichEdit document NVDA uses the control's own text model; the
      report checks the UIA TextPattern equivalent: the whole text, and the
      selection (the caret) while the silent `paced` backend reads.
    - Announcements are UIA notification events (NVDA speaks their display
      string); the report subscribes to them.
    - Menu items are read from the window's Win32 menu (GetMenu), which is
      what the menu bar shows, without opening any menu.

  The GUI never takes the foreground: with --background it shows its window
  minimized, inactive, and without a taskbar button (the report checks that
  the foreground window did not change), it is launched with
  SW_SHOWMINNOACTIVE as a second guard, it plays no audio (the `paced` backend times words like an engine and outputs
  nothing; `null` finishes at once), and the script closes it as soon as the
  probe is done. A screen-reader user at the machine is not disturbed.

  Commands are chosen from the menu with WM_COMMAND and the caret is moved
  with TextPattern, so no keys are typed and no focus is needed.

.EXAMPLE
  powershell -File crates/textweaver-gui/tools/uia-report.ps1 -Out uia.md
  powershell -File crates/textweaver-gui/tools/uia-report.ps1 -NameModes label,accessible
#>
param(
    [string] $Exe = '',
    [string] $Document = '',
    [ValidateSet('paced', 'null')] [string] $Backend = 'paced',
    # Comma-separated: label, accessible, both (one GUI launch each).
    [string] $NameModes = 'label',
    [string] $Out = ''
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
if (-not $Exe) { $Exe = Join-Path $repo 'target\debug\textweaver-gui.exe' }
if (-not $Document) { $Document = Join-Path $repo 'fixtures\sample.md' }
if (-not (Test-Path $Exe)) { throw "Not built: $Exe (run tools\build-windows.ps1 first)." }
$Exe = (Resolve-Path $Exe).Path
$modes = $NameModes -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ }
foreach ($m in $modes) { if ($m -notin @('label', 'accessible', 'both')) { throw "Unknown name mode: $m" } }
$Document = (Resolve-Path $Document).Path

Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, WindowsBase, Accessibility

$source = @'
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using System.Windows.Automation;
using System.Windows.Automation.Text;
using Accessibility;

public static class TwUia
{
    [DllImport("oleacc.dll")]
    static extern int AccessibleObjectFromWindow(IntPtr hwnd, uint id, ref Guid iid,
        [MarshalAs(UnmanagedType.Interface)] out object acc);
    [DllImport("oleacc.dll", CharSet = CharSet.Unicode)]
    static extern uint GetRoleText(uint role, StringBuilder text, uint max);
    [DllImport("oleacc.dll", CharSet = CharSet.Unicode)]
    static extern uint GetStateText(uint state, StringBuilder text, uint max);
    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")]
    static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll")]
    static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")]
    static extern IntPtr GetMenu(IntPtr hwnd);
    [DllImport("user32.dll")]
    static extern int GetMenuItemCount(IntPtr menu);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    static extern int GetMenuString(IntPtr menu, uint item, StringBuilder text, int max, uint flags);
    [DllImport("user32.dll")]
    static extern IntPtr GetSubMenu(IntPtr menu, int pos);

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

    public static readonly List<string> Notifications = new List<string>();
    static readonly Stopwatch Clock = Stopwatch.StartNew();

    /// Starts the GUI minimized and inactive: the first ShowWindow of a
    /// process started with STARTF_USESHOWWINDOW uses wShowWindow.
    public static int LaunchInactive(string exe, string args, string dir)
    {
        var si = new STARTUPINFO();
        si.cb = Marshal.SizeOf(typeof(STARTUPINFO));
        si.dwFlags = 0x1;          // STARTF_USESHOWWINDOW
        si.wShowWindow = 7;        // SW_SHOWMINNOACTIVE
        PROCESS_INFORMATION pi;
        var cmd = new StringBuilder("\"" + exe + "\" " + args);
        if (!CreateProcess(exe, cmd, IntPtr.Zero, IntPtr.Zero, false, 0x08000000 /* CREATE_NO_WINDOW */,
                IntPtr.Zero, dir, ref si, out pi))
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);
        return pi.dwProcessId;
    }

    delegate bool EnumProc(IntPtr hwnd, IntPtr lParam);
    [DllImport("user32.dll")]
    static extern bool EnumWindows(EnumProc proc, IntPtr lParam);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    static extern int GetWindowText(IntPtr hwnd, StringBuilder text, int max);
    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr hwnd);

    /// The GUI's frame, found by process and title among all top-level
    /// windows (a window without a taskbar button included).
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

    [DllImport("user32.dll")]
    static extern bool PostMessage(IntPtr hwnd, uint msg, IntPtr w, IntPtr l);

    /// Asks the window to close, as its close button does (WM_CLOSE).
    public static void Close(IntPtr hwnd) { PostMessage(hwnd, 0x0010, IntPtr.Zero, IntPtr.Zero); }

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

    static void OnNotification(object sender, AutomationEventArgs e)
    {
        var n = e as NotificationEventArgs;
        if (n == null) return;
        try
        {
            var el = sender as AutomationElement;
            if (el == null || el.Current.ProcessId != listenPid) return;
        }
        catch (Exception) { return; }
        lock (Notifications)
        {
            Notifications.Add(string.Format("{0,6} ms  [{1}, {2}]  {3}",
                Clock.ElapsedMilliseconds, n.NotificationKind, n.NotificationProcessing, n.DisplayString));
        }
    }

    public static string[] TakeNotifications()
    {
        lock (Notifications) return Notifications.ToArray();
    }

    public static string Msaa(IntPtr hwnd)
    {
        if (hwnd == IntPtr.Zero) return "";
        Guid iid = new Guid("618736E0-3C3D-11CF-810C-00AA00389B71");
        object o;
        if (AccessibleObjectFromWindow(hwnd, 0xFFFFFFFC /* OBJID_CLIENT */, ref iid, out o) != 0 || o == null) return "(no MSAA)";
        var acc = (IAccessible)o;
        try
        {
            string name = acc.get_accName(0);
            object role = acc.get_accRole(0);
            object state = acc.get_accState(0);
            string value = null;
            try { value = acc.get_accValue(0); } catch (Exception) { }
            var sb = new StringBuilder(256);
            string roleText = role is int ? (GetRoleText((uint)(int)role, sb, 256) > 0 ? sb.ToString() : role.ToString()) : Convert.ToString(role);
            string stateText = StateText(state is int ? (uint)(int)state : 0);
            if (value != null && value.Length > 60) value = value.Substring(0, 60) + "...";
            return string.Format("MSAA name={0} role={1} state={2}{3}", Q(name), roleText, stateText,
                value == null ? "" : " value=" + Q(value));
        }
        catch (Exception ex) { return "(MSAA error " + ex.GetType().Name + ")"; }
    }

    static string StateText(uint state)
    {
        var parts = new List<string>();
        for (int bit = 0; bit < 31; bit++)
        {
            uint flag = 1u << bit;
            if ((state & flag) == 0) continue;
            var sb = new StringBuilder(64);
            GetStateText(flag, sb, 64);
            parts.Add(sb.ToString());
        }
        return parts.Count == 0 ? "normal" : string.Join(", ", parts.ToArray());
    }

    public static string Q(string s) { return s == null ? "(null)" : "\"" + s.Replace("\r", "\\r").Replace("\n", "\\n").Replace("\t", "\\t") + "\""; }

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
        string labeledBy = "";
        try { if (c.LabeledBy != null) labeledBy = " labeledBy=" + Q(c.LabeledBy.Current.Name); } catch (Exception) { }
        string accel = string.IsNullOrEmpty(c.AcceleratorKey) ? "" : " accel=" + Q(c.AcceleratorKey);
        string access = string.IsNullOrEmpty(c.AccessKey) ? "" : " accessKey=" + Q(c.AccessKey);
        string focus = c.HasKeyboardFocus ? " FOCUSED" : (c.IsKeyboardFocusable ? " focusable" : "");
        return string.Format("{0} name={1} class={2}{3}{4}{5}{6} patterns=[{7}]",
            c.ControlType.ProgrammaticName.Replace("ControlType.", ""), Q(c.Name), c.ClassName,
            labeledBy, accel, access, focus, Patterns(e));
    }

    /// Caret offset (UTF-16 units from the start), the selected text, and
    /// the caret's line.
    public static string Caret(AutomationElement doc)
    {
        var tp = (TextPattern)doc.GetCurrentPattern(TextPattern.Pattern);
        var sel = tp.GetSelection();
        if (sel.Length == 0) return "no selection";
        var before = tp.DocumentRange.Clone();
        before.MoveEndpointByRange(TextPatternRangeEndpoint.End, sel[0], TextPatternRangeEndpoint.Start);
        int offset = before.GetText(-1).Length;
        string text = sel[0].GetText(80);
        var line = sel[0].Clone();
        line.ExpandToEnclosingUnit(TextUnit.Line);
        string lineText = line.GetText(60).TrimEnd('\r', '\n');
        return string.Format("offset {0,4}  selected {1,-14}  line {2}", offset, Q(text), Q(lineText));
    }

    public static string DocumentText(AutomationElement doc)
    {
        var tp = (TextPattern)doc.GetCurrentPattern(TextPattern.Pattern);
        return tp.DocumentRange.GetText(-1);
    }

    public static bool IsForeground(int pid)
    {
        uint fg;
        GetWindowThreadProcessId(GetForegroundWindow(), out fg);
        return fg == (uint)pid;
    }

    /// The Win32 menu bar: each menu and its items, with the text after
    /// the tab (the accelerator) as the menu shows it.
    public static List<string> Menus(IntPtr hwnd)
    {
        var lines = new List<string>();
        IntPtr bar = GetMenu(hwnd);
        if (bar == IntPtr.Zero) { lines.Add("(no Win32 menu bar)"); return lines; }
        int n = GetMenuItemCount(bar);
        for (int i = 0; i < n; i++)
        {
            lines.Add("- " + Q(MenuText(bar, i)));
            IntPtr sub = GetSubMenu(bar, i);
            if (sub == IntPtr.Zero) continue;
            int m = GetMenuItemCount(sub);
            for (int j = 0; j < m; j++)
            {
                string t = MenuText(sub, j);
                if (t.Length == 0) { lines.Add("    - (separator)"); continue; }
                int tab = t.IndexOf('\t');
                string label = tab < 0 ? t : t.Substring(0, tab);
                string accel = tab < 0 ? "" : "  accelerator " + Q(t.Substring(tab + 1));
                lines.Add("    - " + Q(label) + accel);
            }
        }
        return lines;
    }

    [DllImport("user32.dll")]
    static extern uint GetMenuItemID(IntPtr menu, int pos);
    [DllImport("user32.dll")]
    static extern IntPtr GetWindow(IntPtr hwnd, uint cmd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    static extern int GetClassName(IntPtr hwnd, StringBuilder text, int max);

    public static string ClassOf(IntPtr hwnd)
    {
        var sb = new StringBuilder(256);
        GetClassName(hwnd, sb, 256);
        return sb.ToString();
    }

    /// The frame's child windows, depth first, with their depth: the
    /// native controls a screen reader walks. (A minimized window's client
    /// area is empty in the UIA control view, so the report reaches each
    /// control through its handle.)
    public static List<KeyValuePair<IntPtr, int>> Children(IntPtr parent)
    {
        var list = new List<KeyValuePair<IntPtr, int>>();
        Walk(parent, 0, list);
        return list;
    }

    static void Walk(IntPtr parent, int depth, List<KeyValuePair<IntPtr, int>> list)
    {
        IntPtr child = GetWindow(parent, 5 /* GW_CHILD */);
        while (child != IntPtr.Zero)
        {
            list.Add(new KeyValuePair<IntPtr, int>(child, depth));
            Walk(child, depth + 1, list);
            child = GetWindow(child, 2 /* GW_HWNDNEXT */);
        }
    }

    public static List<string> DumpWindows(IntPtr frame)
    {
        var lines = new List<string>();
        var top = AutomationElement.FromHandle(frame);
        lines.Add("- " + Describe(top));
        lines.Add("    " + Msaa(frame));
        foreach (var kv in Children(frame))
        {
            string pad = new string(' ', (kv.Value + 1) * 2);
            string visible = IsWindowVisible(kv.Key) ? "" : " (hidden window)";
            try
            {
                var e = AutomationElement.FromHandle(kv.Key);
                lines.Add(pad + "- " + Describe(e) + visible);
            }
            catch (Exception ex) { lines.Add(pad + "- class=" + ClassOf(kv.Key) + " (UIA: " + ex.GetType().Name + ")" + visible); }
            lines.Add(pad + "    " + Msaa(kv.Key));
        }
        return lines;
    }

    public static AutomationElement ChildByClass(IntPtr frame, string prefix)
    {
        foreach (var kv in Children(frame))
            if (ClassOf(kv.Key).StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
                return AutomationElement.FromHandle(kv.Key);
        return null;
    }

    /// Chooses a menu item by its label (the text before the tab) with
    /// WM_COMMAND, as clicking it does; no focus or foreground needed.
    public static bool MenuCommand(IntPtr frame, string label)
    {
        IntPtr bar = GetMenu(frame);
        int n = GetMenuItemCount(bar);
        for (int i = 0; i < n; i++)
        {
            IntPtr sub = GetSubMenu(bar, i);
            int m = GetMenuItemCount(sub);
            for (int j = 0; j < m; j++)
            {
                string t = MenuText(sub, j);
                int tab = t.IndexOf('\t');
                string l = (tab < 0 ? t : t.Substring(0, tab)).Replace("&", "");
                if (l.StartsWith(label, StringComparison.OrdinalIgnoreCase))
                {
                    PostMessage(frame, 0x0111 /* WM_COMMAND */, new IntPtr((int)GetMenuItemID(sub, j)), IntPtr.Zero);
                    return true;
                }
            }
        }
        return false;
    }

    /// Puts the document's caret at the start of paragraph `n` (0-based;
    /// negative counts back from the end) through TextPattern, as arrow
    /// keys or a click would.
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

    /// Subscribes to notifications raised by `element` itself (the GUI's
    /// hidden live-region label).
    public static void ListenOn(AutomationElement element)
    {
        Automation.AddAutomationEventHandler(AutomationElement.NotificationEvent, element,
            TreeScope.Element, OnNotification);
    }

    /// The hidden live-region label: the frame's hidden Static child.
    public static AutomationElement LiveLabel(IntPtr frame)
    {
        foreach (var kv in Children(frame))
            if (ClassOf(kv.Key) == "Static" && !IsWindowVisible(kv.Key))
                return AutomationElement.FromHandle(kv.Key);
        return null;
    }

    static int listenPid;

    /// Keeps only notifications from `pid` (0: none yet).
    public static void ListenFor(int pid)
    {
        lock (Notifications) Notifications.Clear();
        listenPid = pid;
    }

    /// Listens desktop-wide, before the GUI starts, so none of its startup
    /// announcements are missed.
    public static void ListenDesktop()
    {
        Automation.AddAutomationEventHandler(AutomationElement.NotificationEvent, AutomationElement.RootElement,
            TreeScope.Subtree, OnNotification);
    }

    static string MenuText(IntPtr menu, int pos)
    {
        var sb = new StringBuilder(256);
        GetMenuString(menu, (uint)pos, sb, 256, 0x400 /* MF_BYPOSITION */);
        return sb.ToString();
    }
}
'@
Add-Type -TypeDefinition $source -ReferencedAssemblies UIAutomationClient, UIAutomationTypes, WindowsBase, Accessibility

$report = New-Object System.Collections.Generic.List[string]
function Say([string] $line) { $report.Add($line) }
function Fence([string[]] $lines) { Say '```'; foreach ($l in $lines) { Say $l }; Say '```'; Say '' }

$date = (Get-Date).ToString('dddd, MMMM d, yyyy HH:mm')
Say "# textweaver-gui UI Automation report"
Say ""
Say "- Generated: $date"
Say "- Windows: $([Environment]::OSVersion.VersionString)"
Say "- Executable: $Exe"
Say "- Document: $Document"
Say "- Backend: $Backend (silent: no audio output)"
Say "- Launch: --background (minimized, never activated, no taskbar button) with SW_SHOWMINNOACTIVE; closed when the probe ends"
Say ""

[TwUia]::ListenDesktop()
foreach ($mode in $modes) {
    $scratch = Join-Path ([IO.Path]::GetTempPath()) ("tw-uia-" + [Guid]::NewGuid().ToString('N').Substring(0, 8))
    New-Item -ItemType Directory -Force $scratch | Out-Null
    $logFile = Join-Path $scratch 'gui.log'
    $guiArgs = "`"$Document`" --backend $Backend --home `"$scratch`" --read --background --log-file `"$logFile`" --exit-after 90 --accessible-name $mode"
    $foregroundBefore = [TwUia]::GetForegroundWindow()
    [TwUia]::ListenFor(0)
    $guiPid = [TwUia]::LaunchInactive($Exe, $guiArgs, $repo)
    [TwUia]::ListenFor($guiPid)

    Say "## Accessible-name mode: $mode"
    Say ""
    try {
        $window = [TwUia]::WaitForWindow($guiPid, 20000)
        if (-not $window) { throw "The textweaver-gui window did not appear." }
        $frame = [IntPtr]$window.Current.NativeWindowHandle
        # Notifications: desktop-wide (subscribed before launch) and on the
        # live-region label itself.
        $live = $null
        for ($i = 0; $i -lt 20 -and -not $live; $i++) { $live = [TwUia]::LiveLabel($frame); if (-not $live) { Start-Sleep -Milliseconds 50 } }
        if ($live) { [TwUia]::ListenOn($live) }
        Start-Sleep -Milliseconds 1500   # the document opens and reading starts
        $stoleFocus = [TwUia]::IsForeground($guiPid)
        $fgSame = ([TwUia]::GetForegroundWindow() -eq $foregroundBefore)
        Say "- Took the foreground: $stoleFocus; foreground window unchanged: $fgSame"
        Say ""

        Say "### Controls (each native window through UIA and MSAA)"
        Say ""
        Fence ([TwUia]::DumpWindows($frame))

        $doc = [TwUia]::ChildByClass($frame, 'RICHEDIT')
        Say "### Document control"
        Say ""
        $tp = $null
        if (-not $doc -or -not $doc.TryGetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern, [ref]$tp)) {
            Say "The document control does not support TextPattern: its text is NOT exposed to UI Automation."
            Say ""
            $doc = $null
        } else {
            Say "- $([TwUia]::Describe($doc))"
            Say "- $([TwUia]::Msaa([IntPtr]$doc.Current.NativeWindowHandle))"
            $all = [TwUia]::DocumentText($doc)
            Say "- TextPattern document length: $($all.Length) UTF-16 units"
            Say "- First 120 units: $([TwUia]::Q($all.Substring(0, [Math]::Min(120, $all.Length))))"
            $vp = $null
            if ($doc.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$vp)) {
                Say "- ValuePattern read-only: $($vp.Current.IsReadOnly)"
            }
            Say ""
            Say "Caret and selection while reading, every 400 ms:"
            Say ""
            $samples = @()
            for ($i = 0; $i -lt 8; $i++) { $samples += [TwUia]::Caret($doc); Start-Sleep -Milliseconds 400 }
            Fence $samples
        }

        Say "### Menus (Win32 menu bar, read without opening it)"
        Say ""
        Fence ([TwUia]::Menus($frame))

        if ($doc) {
            Say "### Commands (menu items chosen with WM_COMMAND; no focus needed)"
            Say ""
            $steps = @(
                @{ item = 'Play/Pause'; wait = 700 },
                @{ item = 'Next sentence'; wait = 700 },
                @{ item = 'Next sentence'; wait = 700 },
                @{ item = 'Previous sentence'; wait = 700 },
                @{ item = 'Next heading'; wait = 1800 },
                @{ item = 'Stop'; wait = 700 },
                @{ paragraph = 8; wait = 300 },
                @{ item = 'Read current word'; wait = 1200 },
                @{ item = 'Say position'; wait = 700 },
                @{ item = 'Play/Pause'; wait = 1500 },
                @{ item = 'Stop'; wait = 700 },
                @{ paragraph = -6; wait = 300 },
                @{ item = 'Play/Pause'; wait = 1500 },
                @{ item = 'Stop'; wait = 700 }
            )
            $cmdLines = @()
            foreach ($s in $steps) {
                if ($s.ContainsKey('paragraph')) {
                    [TwUia]::PutCaretAtParagraph($doc, $s.paragraph)
                    $what = if ($s.paragraph -ge 0) { "caret to paragraph $($s.paragraph + 1) (TextPattern)" } else { "caret $(-$s.paragraph) paragraphs from the end" }
                } else {
                    if (-not [TwUia]::MenuCommand($frame, $s.item)) { $cmdLines += "(no menu item $($s.item))"; continue }
                    $what = $s.item
                }
                Start-Sleep -Milliseconds $s.wait
                $cmdLines += ("{0,-34} -> {1}" -f $what, [TwUia]::Caret($doc))
            }
            Fence $cmdLines
        }

        Say "### Status bar"
        Say ""
        $sb = [TwUia]::ChildByClass($frame, 'msctls_statusbar32')
        if ($sb) {
            Say "- $([TwUia]::Describe($sb))"
            foreach ($part in $sb.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)) {
                Say "  - $([TwUia]::Describe($part))"
            }
        } else { Say "No status bar." }
        Say ""

        # Close the window as its close button does; the GUI saves its state
        # and exits.
        [TwUia]::Close($frame)
    } finally {
        $p = Get-Process -Id $guiPid -ErrorAction SilentlyContinue
        if ($p -and -not $p.WaitForExit(10000)) {
            Stop-Process -Id $guiPid -Force
            Say "(the GUI did not exit when closed and was stopped)"
        }
    }

    Say "### Live-region notifications (UIA NotificationEvent)"
    Say ""
    $notes = [TwUia]::TakeNotifications()
    if ($notes.Count -eq 0) { $notes = @('(none)') }
    Fence $notes
    Say "### GUI log (announcements, commands, load timing)"
    Say ""
    if (Test-Path $logFile) { Fence (Get-Content $logFile) } else { Fence @('(no log)') }
    Remove-Item -Recurse -Force $scratch -ErrorAction SilentlyContinue
}

try { [System.Windows.Automation.Automation]::RemoveAllEventHandlers() } catch { }
$text = $report -join "`n"
if ($Out) { Set-Content -Path $Out -Value $text -Encoding UTF8 }
$text
