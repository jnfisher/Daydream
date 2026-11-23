# Has a real Btrieve engine (WBTRV32.DLL and the 6.15 workstation engine W32MKDE.EXE, as
# Nightmare Redux ships them) delete and insert records in files, so Daydream's tests can
# check that Btrieve keeps working in the files Daydream's .DAT writer builds.
#
#   powershell -ExecutionPolicy Bypass -File btrieve-churn.ps1 ENGINEDIR FILE.DAT...
#
# For each file: every record is read in physical order (Step First / Step Next) with its
# position; every other record is then found again by its position (Get Direct) and
# deleted; and the deleted records are inserted again, last first. The file ends up with
# the same records, its pages split, merged and reused by Btrieve itself. A file Btrieve
# fails on gets <file>.error.txt with the status, and the others go on. ENGINEDIR holds
# WBTRV32.DLL and W32MKDE.EXE. The engine is 32-bit, so this re-runs itself under the
# 32-bit Windows PowerShell when started from a 64-bit one.
param(
    [Parameter(Mandatory = $true)][string]$Engine,
    [Parameter(Mandatory = $true, ValueFromRemainingArguments = $true)][string[]]$Files
)
$ErrorActionPreference = 'Stop'

if ([IntPtr]::Size -ne 4) {
    $ps32 = Join-Path $env:SystemRoot 'SysWOW64\WindowsPowerShell\v1.0\powershell.exe'
    & $ps32 -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $PSCommandPath $Engine @Files
    exit $LASTEXITCODE
}

$Engine = (Resolve-Path $Engine).Path
$env:PATH = "$Engine;$env:PATH"
Set-Location $Engine

Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public static class BtrieveChurn {
    [DllImport("WBTRV32.DLL", CallingConvention = CallingConvention.StdCall)]
    static extern short BTRCALL(ushort op, byte[] pos, byte[] data, ref int len, byte[] key, short keyLen, short keyNum);

    const ushort OPEN = 0, CLOSE = 1, INSERT = 2, DELETE = 4, STOP = 25, GET_POSITION = 22,
        GET_DIRECT = 23, STEP_NEXT = 24, STEP_FIRST = 33;
    const short END_OF_FILE = 9;

    static void Check(short status, string what) {
        if (status != 0) throw new Exception(what + ": Btrieve status " + status);
    }

    public static string Churn(string file) {
        var pos = new byte[128];
        var data = new byte[65535];
        var key = new byte[255];
        var name = Encoding.ASCII.GetBytes(file + "\0");
        Array.Copy(name, key, name.Length);
        int len = 0;
        Check(BTRCALL(OPEN, pos, data, ref len, key, 255, 0), "open " + file);
        try {
            var records = new List<byte[]>();
            var positions = new List<byte[]>();
            ushort op = STEP_FIRST;
            while (true) {
                len = data.Length;
                short s = BTRCALL(op, pos, data, ref len, key, 255, 0);
                if (s == END_OF_FILE) break;
                Check(s, "step");
                var rec = new byte[len];
                Array.Copy(data, rec, len);
                records.Add(rec);
                var at = new byte[4];
                int plen = 4;
                Check(BTRCALL(GET_POSITION, pos, at, ref plen, key, 255, 0), "get position");
                positions.Add(at);
                op = STEP_NEXT;
            }
            var deleted = new List<byte[]>();
            for (int i = 1; i < records.Count; i += 2) {
                Array.Clear(data, 0, data.Length);
                Array.Copy(positions[i], data, 4);
                len = data.Length;
                Check(BTRCALL(GET_DIRECT, pos, data, ref len, key, 255, 0), "get direct " + i);
                len = 0;
                Check(BTRCALL(DELETE, pos, data, ref len, key, 255, 0), "delete " + i);
                deleted.Add(records[i]);
            }
            for (int i = deleted.Count - 1; i >= 0; i--) {
                var rec = (byte[])deleted[i].Clone();
                len = rec.Length;
                Check(BTRCALL(INSERT, pos, rec, ref len, key, 255, 0), "insert " + i);
            }
            return records.Count + " records, " + deleted.Count + " deleted and inserted again";
        } finally {
            len = 0;
            BTRCALL(CLOSE, pos, data, ref len, key, 255, 0);
        }
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
        try {
            $result = [BtrieveChurn]::Churn($f)
        } catch {
            # one bad file must not hide the others: the test reports <file>.error.txt
            $result = $_.Exception.InnerException.Message
            if (-not $result) { $result = $_.Exception.Message }
            Set-Content -Path "$f.error.txt" -Value $result
        }
        Write-Output "$f`: $result"
    }
} finally {
    [BtrieveChurn]::Stop()
}
