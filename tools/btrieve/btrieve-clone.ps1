# Has a real Btrieve engine (WBTRV32.DLL and the 6.15 workstation engine W32MKDE.EXE, as
# Nightmare Redux ships them) build files, so Daydream's .DAT writer can be checked against
# what Btrieve itself writes.
#
#   powershell -ExecutionPolicy Bypass -File btrieve-clone.ps1 OUTDIR ENGINEDIR FILE.DAT...
#
# For each file, OUTDIR gets <stem>.EMPTY.DAT, a clone made from the file's own Stat (its
# record length, page size, flags, keys and collating sequences, as BUTIL -CLONE makes
# one), and <stem>.FULL.DAT, another clone with every record of the file inserted in
# physical order (Step First / Step Next). Variable-length records go in whole. ENGINEDIR
# holds WBTRV32.DLL and W32MKDE.EXE. The engine is 32-bit, so this re-runs itself under the
# 32-bit Windows PowerShell when started from a 64-bit one.
param(
    [Parameter(Mandatory = $true)][string]$Out,
    [Parameter(Mandatory = $true)][string]$Engine,
    [Parameter(Mandatory = $true, ValueFromRemainingArguments = $true)][string[]]$Files
)
$ErrorActionPreference = 'Stop'

if ([IntPtr]::Size -ne 4) {
    $ps32 = Join-Path $env:SystemRoot 'SysWOW64\WindowsPowerShell\v1.0\powershell.exe'
    & $ps32 -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $PSCommandPath $Out $Engine @Files
    exit $LASTEXITCODE
}

$Engine = (Resolve-Path $Engine).Path
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$Out = (Resolve-Path $Out).Path
$env:PATH = "$Engine;$env:PATH"
Set-Location $Engine

Add-Type -TypeDefinition @'
using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

public static class BtrieveClone {
    [DllImport("WBTRV32.DLL", CallingConvention = CallingConvention.StdCall)]
    static extern short BTRCALL(ushort op, byte[] pos, byte[] data, ref int len, byte[] key, short keyLen, short keyNum);

    const ushort OPEN = 0, CLOSE = 1, INSERT = 2, CREATE = 14, STAT = 15, STOP = 25,
        STEP_NEXT = 24, STEP_FIRST = 33;
    const short END_OF_FILE = 9;

    static void Check(short status, string what) {
        if (status != 0) throw new Exception(what + ": Btrieve status " + status);
    }

    static byte[] Name(string file) {
        var key = new byte[255];
        var name = Encoding.ASCII.GetBytes(file + "\0");
        Array.Copy(name, key, name.Length);
        return key;
    }

    static void Close(byte[] pos) {
        int len = 0;
        BTRCALL(CLOSE, pos, new byte[1], ref len, new byte[255], 255, 0);
    }

    // The Stat buffer as a Create buffer: the file spec (16 bytes), the key specs (16 each)
    // and the collating sequences behind them, with Stat's counts cleared.
    static byte[] Spec(byte[] stat, int len) {
        var spec = new byte[len];
        Array.Copy(stat, spec, len);
        int keys = BitConverter.ToInt16(spec, 4);
        Array.Clear(spec, 6, 4); // record count
        int at = 16;
        int segments = 0;
        for (int k = 0; k < keys; k++) {
            while (true) {
                int flags = BitConverter.ToUInt16(spec, at + 4);
                Array.Clear(spec, at + 6, 4); // unique values
                at += 16;
                segments++;
                if ((flags & 0x10) == 0) break;
            }
        }
        return spec;
    }

    public static string Clone(string file, string outDir) {
        var pos = new byte[128];
        var data = new byte[65535];
        int len = 0;
        Check(BTRCALL(OPEN, pos, data, ref len, Name(file), 255, 0), "open " + file);
        string stem = Path.GetFileNameWithoutExtension(file);
        string empty = Path.Combine(outDir, stem + ".EMPTY.DAT");
        string full = Path.Combine(outDir, stem + ".FULL.DAT");
        int inserted = 0;
        try {
            len = data.Length;
            Check(BTRCALL(STAT, pos, data, ref len, new byte[255], 255, 0), "stat");
            var spec = Spec(data, len);
            foreach (var target in new[] { empty, full }) {
                int specLen = spec.Length;
                var buf = (byte[])spec.Clone();
                Check(BTRCALL(CREATE, new byte[128], buf, ref specLen, Name(target), 255, 0), "create " + target);
            }
            var fpos = new byte[128];
            len = 0;
            Check(BTRCALL(OPEN, fpos, new byte[1], ref len, Name(full), 255, 0), "open " + full);
            try {
                ushort op = STEP_FIRST;
                var key = new byte[255];
                while (true) {
                    len = data.Length;
                    short s = BTRCALL(op, pos, data, ref len, key, 255, 0);
                    if (s == END_OF_FILE) break;
                    Check(s, "step");
                    var rec = new byte[len];
                    Array.Copy(data, rec, len);
                    int ilen = len;
                    Check(BTRCALL(INSERT, fpos, rec, ref ilen, new byte[255], 255, 0), "insert " + inserted);
                    inserted++;
                    op = STEP_NEXT;
                }
            } finally {
                Close(fpos);
            }
        } finally {
            Close(pos);
        }
        return "cloned, " + inserted + " records";
    }

    public static void Stop() {
        int len = 0;
        BTRCALL(STOP, new byte[128], new byte[1], ref len, new byte[255], 255, 0);
    }
}
'@

try {
    foreach ($f in $Files) {
        $f = (Resolve-Path $f).Path
        $result = [BtrieveClone]::Clone($f, $Out)
        Write-Output "$f`: $result"
    }
} finally {
    [BtrieveClone]::Stop()
}
