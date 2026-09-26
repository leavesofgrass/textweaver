<#
.SYNOPSIS
  Checks the Fonts dialog (View, Fonts) through UI Automation and MSAA,
  without taking the foreground or typing a key.

.DESCRIPTION
  Launches textweaver-gui with --background (minimized, never activated,
  no taskbar button; with --background the chooser's controls open in a
  window of their own that is minimized before it is shown, instead of a
  modal dialog, which would take the foreground), then:
    1. reads the document control's font through TextPattern;
    2. chooses View, Fonts with WM_COMMAND;
    3. lists every control of the dialog with its UIA and MSAA name, role,
       label, and tab stop, in tab order;
    4. picks OpenDyslexic in the family list, sets the size to 16, checks
       Bold (UIA SelectionItem, Value, and Toggle patterns), and reads the
       preview's font;
    5. presses OK (Invoke) and reads the document's font again, the live
       region announcement, and the saved [display.font] settings;
    6. opens the dialog again and presses Cancel;
    7. checks the foreground window never changed, and closes the GUI.
  The silent null backend is used: no audio.

.EXAMPLE
  powershell -File crates/textweaver-gui/tools/font-dialog-report.ps1 -Out fonts-uia.md
#>
param(
    [string] $Exe = '',
    [string] $Document = '',
    [string] $Family = 'OpenDyslexic',
    [int] $Size = 16,
    [string] $Out = ''
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
if (-not $Exe) { $Exe = Join-Path $repo 'target\debug\textweaver-gui.exe' }
if (-not $Document) { $Document = Join-Path $repo 'fixtures\sample.md' }
if (-not (Test-Path $Exe)) { throw "Not built: $Exe (run tools\build-windows.ps1 first)." }
$Exe = (Resolve-Path $Exe).Path
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

public static class TwFont
{
    [DllImport("oleacc.dll")]
    static extern int AccessibleObjectFromWindow(IntPtr hwnd, uint id, ref Guid iid,
        [MarshalAs(UnmanagedType.Interface)] out object acc);
    [DllImport("oleacc.dll", CharSet = CharSet.Unicode)]
    static extern uint GetRoleText(uint role, StringBuilder text, uint max);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll")] static extern IntPtr GetMenu(IntPtr hwnd);
    [DllImport("user32.dll")] static extern int GetMenuItemCount(IntPtr menu);
    [DllImport("user32.dll")] static extern IntPtr GetSubMenu(IntPtr menu, int pos);
    [DllImport("user32.dll")] static extern uint GetMenuItemID(IntPtr menu, int pos);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    static extern int GetMenuString(IntPtr menu, uint item, StringBuilder text, int max, uint flags);
    [DllImport("user32.dll")] static extern bool PostMessage(IntPtr hwnd, uint msg, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] static extern IntPtr GetWindow(IntPtr hwnd, uint cmd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr hwnd, StringBuilder text, int max);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr hwnd, StringBuilder text, int max);
    [DllImport("user32.dll")] static extern int GetWindowLong(IntPtr hwnd, int index);
    [DllImport("user32.dll")] static extern int GetDlgCtrlID(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hwnd);
    delegate bool EnumProc(IntPtr hwnd, IntPtr lParam);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc proc, IntPtr lParam);

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
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr h);

    public static readonly List<string> Notifications = new List<string>();
    static int listenPid;

    public static int LaunchInactive(string exe, string args, string dir)
    {
        var si = new STARTUPINFO();
        si.cb = Marshal.SizeOf(typeof(STARTUPINFO));
        si.dwFlags = 0x1;          // STARTF_USESHOWWINDOW
        si.wShowWindow = 7;        // SW_SHOWMINNOACTIVE
        PROCESS_INFORMATION pi;
        var cmd = new StringBuilder("\"" + exe + "\" " + args);
        if (!CreateProcess(exe, cmd, IntPtr.Zero, IntPtr.Zero, false, 0x08000000, IntPtr.Zero, dir, ref si, out pi))
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
        CloseHandle(pi.hThread); CloseHandle(pi.hProcess);
        return pi.dwProcessId;
    }

    public static IntPtr FindTop(int pid, string titlePart)
    {
        IntPtr found = IntPtr.Zero;
        EnumWindows(delegate (IntPtr hwnd, IntPtr l)
        {
            uint owner; GetWindowThreadProcessId(hwnd, out owner);
            if (owner != (uint)pid) return true;
            if (Text(hwnd).Contains(titlePart)) { found = hwnd; return false; }
            return true;
        }, IntPtr.Zero);
        return found;
    }

    public static IntPtr WaitTop(int pid, string titlePart, int ms)
    {
        var sw = Stopwatch.StartNew();
        while (sw.ElapsedMilliseconds < ms)
        {
            IntPtr h = FindTop(pid, titlePart);
            if (h != IntPtr.Zero) return h;
            Thread.Sleep(100);
        }
        return IntPtr.Zero;
    }

    public static string Text(IntPtr hwnd) { var sb = new StringBuilder(512); GetWindowText(hwnd, sb, 512); return sb.ToString(); }
    public static string ClassOf(IntPtr hwnd) { var sb = new StringBuilder(256); GetClassName(hwnd, sb, 256); return sb.ToString(); }
    public static string ForegroundInfo()
    {
        IntPtr h = GetForegroundWindow();
        uint pid; GetWindowThreadProcessId(h, out pid);
        string name = "?";
        try { name = Process.GetProcessById((int)pid).ProcessName; } catch (Exception) { }
        return string.Format("{0} (process {1}, {2})", Q(Text(h)), pid, name);
    }
    public static bool IsForeground(int pid) { uint fg; GetWindowThreadProcessId(GetForegroundWindow(), out fg); return fg == (uint)pid; }
    public static void Close(IntPtr hwnd) { PostMessage(hwnd, 0x0010, IntPtr.Zero, IntPtr.Zero); }

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
                var sb = new StringBuilder(256);
                GetMenuString(sub, (uint)j, sb, 256, 0x400);
                string t = sb.ToString(); int tab = t.IndexOf('\t');
                string l = (tab < 0 ? t : t.Substring(0, tab)).Replace("&", "");
                if (l.StartsWith(label, StringComparison.OrdinalIgnoreCase))
                {
                    PostMessage(frame, 0x0111, new IntPtr((int)GetMenuItemID(sub, j)), IntPtr.Zero);
                    return true;
                }
            }
        }
        return false;
    }

    /// Child windows in tab order (z-order), depth first.
    public static List<IntPtr> Children(IntPtr parent)
    {
        var list = new List<IntPtr>();
        IntPtr c = GetWindow(parent, 5);
        while (c != IntPtr.Zero) { list.Add(c); list.AddRange(Children(c)); c = GetWindow(c, 2); }
        return list;
    }

    public static IntPtr ChildByClass(IntPtr parent, string prefix, int nth)
    {
        foreach (var h in Children(parent))
            if (ClassOf(h).StartsWith(prefix, StringComparison.OrdinalIgnoreCase) && nth-- == 0) return h;
        return IntPtr.Zero;
    }

    public static string Q(string s) { return s == null ? "(null)" : "\"" + s.Replace("\r", "\\r").Replace("\n", "\\n") + "\""; }

    public static string Msaa(IntPtr hwnd)
    {
        Guid iid = new Guid("618736E0-3C3D-11CF-810C-00AA00389B71");
        object o;
        if (AccessibleObjectFromWindow(hwnd, 0xFFFFFFFC, ref iid, out o) != 0 || o == null) return "(no MSAA)";
        var acc = (IAccessible)o;
        try
        {
            object role = acc.get_accRole(0);
            var sb = new StringBuilder(256);
            string roleText = role is int ? (GetRoleText((uint)(int)role, sb, 256) > 0 ? sb.ToString() : role.ToString()) : Convert.ToString(role);
            string value = null; try { value = acc.get_accValue(0); } catch (Exception) { }
            return "MSAA name=" + Q(acc.get_accName(0)) + " role=" + roleText + (value == null ? "" : " value=" + Q(value));
        }
        catch (Exception ex) { return "(MSAA error " + ex.GetType().Name + ")"; }
    }

    public static string Describe(IntPtr hwnd)
    {
        var e = AutomationElement.FromHandle(hwnd);
        var c = e.Current;
        string labeledBy = "";
        try { if (c.LabeledBy != null) labeledBy = " labeledBy=" + Q(c.LabeledBy.Current.Name); } catch (Exception) { }
        string access = string.IsNullOrEmpty(c.AccessKey) ? "" : " accessKey=" + Q(c.AccessKey);
        bool tabStop = (GetWindowLong(hwnd, -16) & 0x00010000) != 0;
        return string.Format("{0} name={1} class={2}{3}{4}{5}", c.ControlType.ProgrammaticName.Replace("ControlType.", ""),
            Q(c.Name), ClassOf(hwnd), labeledBy, access, tabStop ? " TABSTOP" : "");
    }

    /// Font name, size, and weight of a rich edit's whole text.
    public static string Font(IntPtr hwnd)
    {
        var e = AutomationElement.FromHandle(hwnd);
        object p;
        if (!e.TryGetCurrentPattern(TextPattern.Pattern, out p)) return "(no TextPattern)";
        var r = ((TextPattern)p).DocumentRange;
        return string.Format("font {0}, {1} pt, weight {2}",
            r.GetAttributeValue(TextPattern.FontNameAttribute),
            r.GetAttributeValue(TextPattern.FontSizeAttribute),
            r.GetAttributeValue(TextPattern.FontWeightAttribute));
    }

    public static bool SelectItem(IntPtr list, string startsWith)
    {
        var e = AutomationElement.FromHandle(list);
        foreach (AutomationElement item in e.FindAll(TreeScope.Children, Condition.TrueCondition))
        {
            if (!item.Current.Name.StartsWith(startsWith, StringComparison.OrdinalIgnoreCase)) continue;
            ((SelectionItemPattern)item.GetCurrentPattern(SelectionItemPattern.Pattern)).Select();
            // A programmatic selection sends no LBN_SELCHANGE; send it, as a
            // click or an arrow key would.
            IntPtr parent = GetParent(list);
            int id = GetDlgCtrlID(list);
            PostMessage(parent, 0x0111, new IntPtr((1 /* LBN_SELCHANGE */ << 16) | (id & 0xFFFF)), list);
            return true;
        }
        return false;
    }
    [DllImport("user32.dll")] static extern IntPtr GetParent(IntPtr hwnd);

    public static string GetValue(IntPtr hwnd)
    {
        return ((ValuePattern)AutomationElement.FromHandle(hwnd).GetCurrentPattern(ValuePattern.Pattern)).Current.Value;
    }

    /// Waits for a button to exist in a window (its controls are created
    /// just after the window itself).
    public static IntPtr WaitButton(IntPtr parent, string name, int ms)
    {
        var sw = Stopwatch.StartNew();
        while (sw.ElapsedMilliseconds < ms)
        {
            IntPtr h = ButtonNamed(parent, name);
            if (h != IntPtr.Zero) return h;
            Thread.Sleep(50);
        }
        return IntPtr.Zero;
    }

    public static void SetValue(IntPtr hwnd, string value)
    {
        ((ValuePattern)AutomationElement.FromHandle(hwnd).GetCurrentPattern(ValuePattern.Pattern)).SetValue(value);
    }

    public static string Toggle(IntPtr hwnd)
    {
        var t = (TogglePattern)AutomationElement.FromHandle(hwnd).GetCurrentPattern(TogglePattern.Pattern);
        t.Toggle();
        return t.Current.ToggleState.ToString();
    }

    public static void Invoke(IntPtr hwnd)
    {
        ((InvokePattern)AutomationElement.FromHandle(hwnd).GetCurrentPattern(InvokePattern.Pattern)).Invoke();
    }

    public static IntPtr ButtonNamed(IntPtr parent, string name)
    {
        foreach (var h in Children(parent))
            if (ClassOf(h) == "Button" && Text(h).Replace("&", "") == name) return h;
        return IntPtr.Zero;
    }

    static void OnNotification(object sender, AutomationEventArgs e)
    {
        var n = e as NotificationEventArgs;
        if (n == null) return;
        try { var el = sender as AutomationElement; if (el == null || el.Current.ProcessId != listenPid) return; }
        catch (Exception) { return; }
        lock (Notifications) Notifications.Add(n.DisplayString);
    }

    public static void Listen(int pid)
    {
        listenPid = pid;
        Automation.AddAutomationEventHandler(AutomationElement.NotificationEvent, AutomationElement.RootElement,
            TreeScope.Subtree, OnNotification);
    }

    /// The notifications heard, each once (two subscriptions can hear the
    /// same event).
    public static string[] Taken()
    {
        lock (Notifications)
        {
            var seen = new List<string>();
            for (int i = 0; i < Notifications.Count; i++)
                if (i == 0 || Notifications[i] != Notifications[i - 1]) seen.Add(Notifications[i]);
            return seen.ToArray();
        }
    }

    /// Also listens on the frame's hidden live-region label itself (a
    /// desktop-wide subscription can miss events from a minimized window).
    public static bool ListenOnLiveLabel(IntPtr frame)
    {
        foreach (var h in Children(frame))
        {
            if (ClassOf(h) == "Static" && !IsWindowVisible(h))
            {
                Automation.AddAutomationEventHandler(AutomationElement.NotificationEvent,
                    AutomationElement.FromHandle(h), TreeScope.Element, OnNotification);
                return true;
            }
        }
        return false;
    }
}
'@
Add-Type -TypeDefinition $source -ReferencedAssemblies UIAutomationClient, UIAutomationTypes, WindowsBase, Accessibility

$report = New-Object System.Collections.Generic.List[string]
function Say([string] $line) { $report.Add($line) }
function Fence([string[]] $lines) { Say '```'; foreach ($l in $lines) { Say $l }; Say '```'; Say '' }

$scratch = Join-Path ([IO.Path]::GetTempPath()) ("tw-fonts-" + [Guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Force $scratch | Out-Null
$logFile = Join-Path $scratch 'gui.log'
$guiArgs = "`"$Document`" --backend null --home `"$scratch`" --background --log --log-file `"$logFile`" --exit-after 120"

Say "# textweaver-gui Fonts dialog report"
Say ""
Say "- Generated: $((Get-Date).ToString('dddd, MMMM d, yyyy HH:mm'))"
Say "- Executable: $Exe"
Say "- Launch: --background (the main window and the Fonts window are minimized before they are shown, so neither is activated); backend null (silent)"
Say ""

$foregroundBefore = [TwFont]::GetForegroundWindow()
$foregroundBeforeInfo = [TwFont]::ForegroundInfo()
$guiPid = [TwFont]::LaunchInactive($Exe, $guiArgs, $repo)
[TwFont]::Listen($guiPid)
$failures = @()
$script:tookForeground = $false
function Check-Foreground { if ([TwFont]::IsForeground($guiPid)) { $script:tookForeground = $true } }
try {
    $frame = [TwFont]::WaitTop($guiPid, 'textweaver', 20000)
    if ($frame -eq [IntPtr]::Zero) { throw "The window did not appear." }
    for ($i = 0; $i -lt 20 -and -not [TwFont]::ListenOnLiveLabel($frame); $i++) { Start-Sleep -Milliseconds 50 }
    Start-Sleep -Milliseconds 1500
    Check-Foreground
    $doc = [TwFont]::ChildByClass($frame, 'RICHEDIT', 0)
    Say "## Before"
    Say ""
    Say "- Document control: $([TwFont]::Font($doc))"
    Say ""

    if (-not [TwFont]::MenuCommand($frame, 'Fonts')) { throw "No View, Fonts menu item." }
    $dlg = [TwFont]::WaitTop($guiPid, 'Fonts', 15000)
    if ($dlg -eq [IntPtr]::Zero) { throw "The Fonts dialog did not open." }
    if ([TwFont]::WaitButton($dlg, 'Cancel', 10000) -eq [IntPtr]::Zero) { throw "The Fonts dialog has no Cancel button." }
    Start-Sleep -Milliseconds 300
    Check-Foreground
    Say "## The dialog's controls, in tab order (UIA, then MSAA)"
    Say ""
    $lines = @("- Window: $([TwFont]::Describe($dlg)); minimized (never activated): $([TwFont]::IsIconic($dlg))")
    foreach ($h in [TwFont]::Children($dlg)) {
        $lines += "- " + [TwFont]::Describe($h)
        $lines += "    " + [TwFont]::Msaa($h)
    }
    Fence $lines

    $list = [TwFont]::ChildByClass($dlg, 'ListBox', 0)
    $items = [System.Windows.Automation.AutomationElement]::FromHandle($list).FindAll(
        [System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
    Say "## Family list ($($items.Count) families; the first eight)"
    Say ""
    $first = @(); $k = 0
    foreach ($it in $items) { if ($k -lt 8) { $first += "- " + $it.Current.Name }; $k++ }
    Fence $first

    $preview = [TwFont]::ChildByClass($dlg, 'RICHEDIT', 0)
    Say "## Choosing $Family, $Size points, bold"
    Say ""
    $steps = @("Preview at first: $([TwFont]::Font($preview))")
    if (-not [TwFont]::SelectItem($list, $Family)) { $failures += "no $Family in the list" }
    Start-Sleep -Milliseconds 400
    $steps += "Selected ${Family}: preview $([TwFont]::Font($preview))"
    $spinEdit = [TwFont]::ChildByClass($dlg, 'Edit', 0)
    [TwFont]::SetValue($spinEdit, "$Size")
    $steps += "Size set to ${Size}: the size box reads $([TwFont]::Q([TwFont]::GetValue($spinEdit)))"
    $bold = [TwFont]::ButtonNamed($dlg, 'Bold')
    $state = [TwFont]::Toggle($bold)
    Start-Sleep -Milliseconds 400
    $steps += "Bold toggled ($state): preview $([TwFont]::Font($preview))"
    [TwFont]::Invoke([TwFont]::ButtonNamed($dlg, 'OK'))
    Start-Sleep -Milliseconds 1200
    Check-Foreground
    $steps += "OK pressed; dialog still open: $([TwFont]::FindTop($guiPid, 'Fonts') -ne [IntPtr]::Zero)"
    Fence $steps

    Say "## After"
    Say ""
    $after = [TwFont]::Font($doc)
    Say "- Document control: $after"
    if ($after -notlike "font $Family*") { $failures += "the document font is not $Family" }
    $settings = Join-Path $scratch 'config\settings.toml'
    if (-not (Test-Path $settings)) { $settings = Get-ChildItem -Recurse $scratch -Filter settings.toml | Select-Object -First 1 -ExpandProperty FullName }
    Say "- Saved settings ($settings):"
    Say ""
    if ($settings -and (Test-Path $settings)) { Fence (Get-Content $settings) } else { $failures += "no settings.toml"; Fence @('(none)') }

    # Cancel leaves everything as it is.
    [void][TwFont]::MenuCommand($frame, 'Fonts')
    $dlg2 = [TwFont]::WaitTop($guiPid, 'Fonts', 15000)
    $cancel = if ($dlg2 -ne [IntPtr]::Zero) { [TwFont]::WaitButton($dlg2, 'Cancel', 10000) } else { [IntPtr]::Zero }
    if ($cancel -ne [IntPtr]::Zero) {
        Check-Foreground
        [TwFont]::Invoke($cancel)
        Start-Sleep -Milliseconds 800
        Say "- Opened again and cancelled; document control: $([TwFont]::Font($doc))"
        Say ""
    } else {
        $failures += "the dialog did not open a second time with a Cancel button (found window $dlg2)"
    }
} catch {
    $failures += "error: $($_.Exception.Message)"
} finally {
    # The GUI must never hold the foreground. (The foreground window may
    # still change: the person at the machine keeps working.)
    $fgSame = ([TwFont]::GetForegroundWindow() -eq $foregroundBefore)
    Say "- The GUI held the foreground at any check: $script:tookForeground; foreground window the same as before: $fgSame"
    Say "- Foreground before: $foregroundBeforeInfo; after: $([TwFont]::ForegroundInfo())"
    Say ""
    if ($script:tookForeground) { $failures += "the GUI took the foreground" }
    $f = [TwFont]::FindTop($guiPid, 'textweaver')
    if ($f -ne [IntPtr]::Zero) { [TwFont]::Close($f) }
    $p = Get-Process -Id $guiPid -ErrorAction SilentlyContinue
    if ($p -and -not $p.WaitForExit(10000)) { Stop-Process -Id $guiPid -Force; Say "(the GUI did not exit when closed and was stopped)" }
}

Say "## Live-region announcements"
Say ""
$notes = [TwFont]::Taken()
if ($notes.Count -eq 0) { $notes = @('(none)') }
Fence $notes
if (-not ($notes | Where-Object { $_ -like "Font: $Family, $Size points, bold.*" })) { $failures += "no announcement of the new font" }
Say "## GUI log (font lines)"
Say ""
if (Test-Path $logFile) { Fence (Get-Content $logFile | Where-Object { $_ -match 'font|announce' }) } else { Fence @('(no log)') }
Remove-Item -Recurse -Force $scratch -ErrorAction SilentlyContinue
try { [System.Windows.Automation.Automation]::RemoveAllEventHandlers() } catch { }

Say "## Result"
Say ""
if ($failures.Count -eq 0) { Say "PASS" } else { Say ("FAIL: " + ($failures -join '; ')) }
$text = $report -join "`n"
if ($Out) { Set-Content -Path $Out -Value $text -Encoding UTF8 }
$text
if ($failures.Count -gt 0) { exit 1 }
