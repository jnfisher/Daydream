# Imports a .mdb into a realm through Nightmare Redux's own Database Importer, with nobody
# at the keyboard, and times each step.
#
#   powershell -ExecutionPolicy Bypass -File nmr-import.ps1 NMRDIR WGDIR FILE.mdb OUTDIR
#
# NMRDIR holds nightmare_redux.exe and what it ships beside it (its OCXs, ability.mdb and the
# Btrieve engine); its settings.ini is rewritten. WGDIR holds the realm as Worldgroup files
# (wccitem2.dat and so on), which NMR edits in place. NMR has no command line, so this drives
# its windows with plain Win32 messages, which need no focus or desktop input: it picks
# Tools > Databases > Import Records from the menu, selects FILE.mdb (alone in its folder,
# which is the importer's start folder), ticks every table, chooses Update and full logging,
# presses Import, answers NMR's message boxes, then closes NMR and the engine so every write
# is on disk. OUTDIR gets NMR's import log, timings.txt, the text of every message box NMR
# showed, and a screenshot of anything unexpected.
param(
    [Parameter(Mandatory = $true)][string]$Nmr,
    [Parameter(Mandatory = $true)][string]$Wg,
    [Parameter(Mandatory = $true)][string]$Mdb,
    [Parameter(Mandatory = $true)][string]$Out,
    [int]$TimeoutSeconds = 600
)
$ErrorActionPreference = 'Stop'

$Nmr = (Resolve-Path $Nmr).Path
$Wg = (Resolve-Path $Wg).Path
$Mdb = (Resolve-Path $Mdb).Path
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$Out = (Resolve-Path $Out).Path
$importDir = Split-Path $Mdb -Parent
# Btrieve ends a file name at the first space
if ($Wg -match ' ') { throw "the realm's path has a space, which Btrieve cannot open: $Wg" }

Add-Type -AssemblyName System.Drawing, System.Windows.Forms
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public static class W {
    delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc f, IntPtr l);
    [DllImport("user32.dll")] static extern bool EnumChildWindows(IntPtr p, EnumProc f, IntPtr l);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr h);
    [DllImport("user32.dll", CharSet = CharSet.Auto)] static extern int GetClassName(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll", CharSet = CharSet.Auto)] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll", CharSet = CharSet.Auto)] static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr h, int id);
    [DllImport("user32.dll")] static extern IntPtr GetMenu(IntPtr h);
    [DllImport("user32.dll")] static extern IntPtr GetSubMenu(IntPtr m, int i);
    [DllImport("user32.dll")] static extern int GetMenuItemCount(IntPtr m);
    [DllImport("user32.dll")] static extern uint GetMenuItemID(IntPtr m, int i);
    [DllImport("user32.dll", CharSet = CharSet.Auto)] static extern int GetMenuString(IntPtr m, uint i, StringBuilder s, int n, uint f);

    public const uint WM_COMMAND = 0x111, WM_CLOSE = 0x10, BM_CLICK = 0xF5, BM_SETCHECK = 0xF1,
        LB_SETCURSEL = 0x186, LB_GETCOUNT = 0x18B, LB_GETTEXT = 0x189, LB_GETTEXTLEN = 0x18A;

    public static string Class(IntPtr h) { var s = new StringBuilder(256); GetClassName(h, s, 256); return s.ToString(); }
    public static string Text(IntPtr h) { var s = new StringBuilder(4096); GetWindowText(h, s, 4096); return s.ToString(); }

    public static IntPtr[] TopLevel(uint pid) {
        var found = new List<IntPtr>();
        EnumWindows((h, l) => {
            uint p; GetWindowThreadProcessId(h, out p);
            if (p == pid && IsWindowVisible(h)) found.Add(h);
            return true;
        }, IntPtr.Zero);
        return found.ToArray();
    }

    public static IntPtr[] Children(IntPtr parent) {
        var found = new List<IntPtr>();
        EnumChildWindows(parent, (h, l) => { found.Add(h); return true; }, IntPtr.Zero);
        return found.ToArray();
    }

    // A message box's text: the static control with id 0xFFFF, or else every static's text
    public static string DialogText(IntPtr dlg) {
        var sb = new StringBuilder();
        foreach (var c in Children(dlg))
            if (Class(c) == "Static" && Text(c).Length > 0) sb.Append(Text(c)).Append(' ');
        return sb.ToString().Trim();
    }

    // The command id of a menu item, found by its path of captions (without & and accelerator)
    public static int MenuId(IntPtr window, string[] path) {
        IntPtr menu = GetMenu(window);
        for (int depth = 0; depth < path.Length; depth++) {
            int n = GetMenuItemCount(menu), hit = -1;
            for (int i = 0; i < n; i++) {
                var s = new StringBuilder(256);
                GetMenuString(menu, (uint)i, s, 256, 0x400);
                string t = s.ToString().Replace("&", "");
                int tab = t.IndexOf('\t'); if (tab >= 0) t = t.Substring(0, tab);
                if (t.Trim() == path[depth]) { hit = i; break; }
            }
            if (hit < 0) return -1;
            if (depth == path.Length - 1) return (int)GetMenuItemID(menu, hit);
            menu = GetSubMenu(menu, hit);
        }
        return -1;
    }

    public static string[] ListItems(IntPtr list) {
        int n = (int)SendMessage(list, LB_GETCOUNT, IntPtr.Zero, IntPtr.Zero);
        var items = new string[n];
        for (int i = 0; i < n; i++) {
            int len = (int)SendMessage(list, LB_GETTEXTLEN, (IntPtr)i, IntPtr.Zero);
            var buf = Marshal.AllocHGlobal((len + 1) * 2);
            SendMessage(list, LB_GETTEXT, (IntPtr)i, buf);
            items[i] = Marshal.PtrToStringAuto(buf);
            Marshal.FreeHGlobal(buf);
        }
        return items;
    }

    public static bool Checked(IntPtr box) { return SendMessage(box, 0xF0, IntPtr.Zero, IntPtr.Zero) != IntPtr.Zero; }

    public static void Select(IntPtr list, int i) { SendMessage(list, LB_SETCURSEL, (IntPtr)i, IntPtr.Zero); }
}
'@

$clock = [Diagnostics.Stopwatch]::StartNew()
$timings = New-Object System.Collections.Generic.List[string]
$lap = [Diagnostics.Stopwatch]::StartNew()
function Mark([string]$what) {
    $line = '{0,-44} {1,7:N1} s  (total {2,6:N1} s)' -f $what, $lap.Elapsed.TotalSeconds, $clock.Elapsed.TotalSeconds
    Write-Host $line
    $timings.Add($line)
    $lap.Restart()
}
$shots = 0
function Shot([string]$why) {
    $script:shots++
    $b = [Windows.Forms.SystemInformation]::VirtualScreen
    $bmp = New-Object Drawing.Bitmap $b.Width, $b.Height
    $g = [Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($b.Left, $b.Top, 0, 0, $bmp.Size)
    $file = Join-Path $Out ('{0:D2}-{1}.png' -f $script:shots, ($why -replace '[^A-Za-z0-9]+', '-'))
    $bmp.Save($file, [Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
    Write-Host "screenshot: $file"
}
$dialogs = New-Object System.Collections.Generic.List[string]
# Message boxes NMR shows on the way, and the button each gets: 6 Yes, 7 No, 1 OK
$answers = @(
    @{ Match = 'Are you sure you want to import from'; Button = 6; Expected = $true },
    @{ Match = 'Import complete, view log file'; Button = 7; Expected = $true; Done = $true },
    @{ Match = 'Import canceled, view log file'; Button = 7; Done = $true }
)
$state = @{ Done = $false; Unexpected = 0 }
# Answers every message box NMR has up; returns how many there were
function Answer-Dialogs($proc) {
    $n = 0
    foreach ($h in [W]::TopLevel([uint32]$proc.Id)) {
        if ([W]::Class($h) -ne '#32770') { continue }
        $n++
        $text = [W]::DialogText($h)
        $title = [W]::Text($h)
        $rule = $answers | Where-Object { $text -like "*$($_.Match)*" } | Select-Object -First 1
        if ($rule) {
            $button = $rule.Button
        } else {
            $state.Unexpected++
            Shot "unexpected dialog"
            # anything else: OK if it has one, else Yes
            $button = if ([W]::GetDlgItem($h, 1) -ne [IntPtr]::Zero) { 1 } else { 6 }
        }
        $line = "[{0,6:N1} s] {1}: {2} -> {3}" -f $clock.Elapsed.TotalSeconds, $title, $text, @{1 = 'OK'; 6 = 'Yes'; 7 = 'No' }[$button]
        if (-not $rule) { $line = "UNEXPECTED $line" }
        Write-Host $line
        $dialogs.Add($line)
        [W]::PostMessage($h, [W]::WM_COMMAND, [IntPtr]$button, [IntPtr]::Zero) | Out-Null
        if ($rule -and $rule.Done) { $state.Done = $true }
        Start-Sleep -Milliseconds 200
    }
    $n
}
function Wait-For([string]$what, $proc, [scriptblock]$test, [int]$seconds = 120) {
    $until = $clock.Elapsed.TotalSeconds + $seconds
    while ($clock.Elapsed.TotalSeconds -lt $until) {
        if ($proc.HasExited) { throw "NMR exited (code $($proc.ExitCode)) while waiting for $what" }
        Answer-Dialogs $proc | Out-Null
        $r = & $test
        if ($r) { return $r }
        Start-Sleep -Milliseconds 250
    }
    Shot "timeout $what"
    foreach ($h in [W]::TopLevel([uint32]$proc.Id)) {
        Write-Host ("  window {0} '{1}' enabled {2}" -f [W]::Class($h), [W]::Text($h), [W]::IsWindowEnabled($h))
    }
    throw "timed out after $seconds s waiting for $what"
}

# settings.ini: the realm, its version (9 is 1.11p) and call letters, and the importer's
# options; NMR reads a missing key as 0
$wgPath = $Wg.TrimEnd('\') + '\'
$ini = @"
[Settings]
FirstRun=1
eDatFileVersion=9
DatCallLetters=cc
WGPath=$wgPath
AutoCompile=0
ShowAbilityEditWarning=0
TaskBarPos=1
[Options]
ImportPath=$importDir
ImportUpdate=1
ImportErrorsSkips=0
ImportErrorsOnly=0
"@
foreach ($t in 'Messages', 'Rooms', 'Shops', 'Spells', 'Items', 'Textblocks', 'Races', 'Classes', 'Monsters', 'Actions') {
    $ini += "`r`nImport${t}All=1"
}
[IO.File]::WriteAllText((Join-Path $Nmr 'settings.ini'), ($ini -replace "`r?`n", "`r`n") + "`r`n", [Text.Encoding]::ASCII)
Remove-Item -ErrorAction SilentlyContinue (Join-Path $importDir 'NMR-Log_Import.txt')

# WBTRV32.DLL starts the engine it finds on the PATH
$env:PATH = "$Nmr;$env:PATH"
$proc = Start-Process -FilePath (Join-Path $Nmr 'nightmare_redux.exe') -WorkingDirectory $Nmr -PassThru
$result = 1
try {
    Mark 'start NMR'

    # Ready when the main window is up and enabled, and no form or message box (the splash) is
    $main = Wait-For 'the main window' $proc {
        $top = [W]::TopLevel([uint32]$proc.Id)
        $m = $top | Where-Object { [W]::Class($_) -eq 'ThunderRT6MDIForm' } | Select-Object -First 1
        $other = $top | Where-Object { [W]::Class($_) -in '#32770', 'ThunderRT6FormDC' }
        if ($m -and -not $other -and [W]::IsWindowEnabled($m)) { $m }
    }
    Mark "NMR open ($([W]::Text($main)))"

    $main = [IntPtr]$main
    $id = [W]::MenuId($main, [string[]]@('Tools', 'Databases', 'Import Records'))
    if ($id -lt 0) { Shot 'no menu'; throw 'no Tools > Databases > Import Records menu item' }
    [W]::PostMessage($main, [W]::WM_COMMAND, [IntPtr]$id, [IntPtr]::Zero) | Out-Null
    $importer = Wait-For 'the Database Importer' $proc {
        [W]::Children($main) | Where-Object { [W]::Text($_) -eq 'Database Importer' } | Select-Object -First 1
    } 30
    # The form shows before Form_Load has made all its controls; wait for the file list
    $list = Wait-For 'the importer''s file list' $proc {
        [W]::Children([IntPtr]$importer) | Where-Object { [W]::Class($_) -like '*FileListBox' } | Select-Object -First 1
    } 30
    $list = [IntPtr]$list
    Start-Sleep -Milliseconds 500
    $controls = [W]::Children([IntPtr]$importer)
    $button = { param($caption) $controls | Where-Object { [W]::Class($_) -like '*CommandButton' -and [W]::Text($_) -eq $caption } | Select-Object -First 1 }
    $name = Split-Path $Mdb -Leaf
    Wait-For "$name in the file list" $proc { ([W]::ListItems($list)) -contains $name } 10 | Out-Null
    $items = @([W]::ListItems($list))
    Mark 'importer open'

    [W]::Select($list, [Array]::IndexOf($items, $name))
    [W]::PostMessage((& $button 'All'), [W]::BM_CLICK, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
    Start-Sleep -Milliseconds 300
    Answer-Dialogs $proc | Out-Null
    $checked = $controls | Where-Object { [W]::Class($_) -like '*CheckBox' } | Where-Object { [W]::Checked($_) } | ForEach-Object { [W]::Text($_) }
    Write-Host "ticked: $($checked -join ', ')"
    [W]::PostMessage((& $button '&Import'), [W]::BM_CLICK, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
    Wait-For 'the import to start' $proc { $dialogs | Where-Object { $_ -like '*Are you sure you want to import*' } } 15 | Out-Null
    Mark 'import started'
    Wait-For 'the import to finish' $proc { $state.Done } $TimeoutSeconds | Out-Null
    Mark 'import finished'

    # Close NMR so it closes the files, then the engine, which caches pages until it exits
    [W]::PostMessage($main, [W]::WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
    while (-not $proc.HasExited -and $clock.Elapsed.TotalSeconds -lt $TimeoutSeconds + 300) {
        Answer-Dialogs $proc | Out-Null
        Start-Sleep -Milliseconds 250
    }
    Mark 'NMR closed'
    $result = if ($state.Unexpected -gt 0) { 2 } else { 0 }
} catch {
    Write-Host "FAILED: $_"
    Write-Host $_.InvocationInfo.PositionMessage
    Write-Host $_.ScriptStackTrace
    Shot 'failure'
} finally {
    if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }
    Get-Process W32MKDE -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Seconds 1
    Mark 'engine stopped'
    $log = Join-Path $importDir 'NMR-Log_Import.txt'
    if (Test-Path $log) { Copy-Item $log $Out }
    $timings | Set-Content (Join-Path $Out 'timings.txt')
    $dialogs | Set-Content (Join-Path $Out 'dialogs.txt')
}
if ($state.Unexpected -gt 0) { Write-Output "$($state.Unexpected) unexpected message box(es); see dialogs.txt" }
exit $result
