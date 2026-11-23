# Reads Btrieve files through a real Btrieve engine (WBTRV32.DLL and the 6.15 workstation
# engine W32MKDE.EXE, as Nightmare Redux ships them) and writes their records in each key's
# order, so Daydream's tests can check its reader and writer against Btrieve itself.
#
#   powershell -ExecutionPolicy Bypass -File btrieve-dump.ps1 OUTDIR ENGINEDIR FILE.DAT...
#
# All the files go through one engine start. For each file, OUTDIR\<file name> gets
# key<N>.bin for every key: each record as a 4-byte little-endian length and its bytes, in
# the order Btrieve's Get First / Get Next return them; and stat.txt with the record count
# and key count from Btrieve's Stat. ENGINEDIR holds WBTRV32.DLL and W32MKDE.EXE. The
# engine caches pages until it exits, so give it files it has not read before. It is 32-bit,
# so this re-runs itself under the 32-bit Windows PowerShell when started from a 64-bit one.
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
# WBTRV32.DLL starts the engine it finds on the PATH
$env:PATH = "$Engine;$env:PATH"
Set-Location $Engine

Add-Type -TypeDefinition @'
using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

public static class Btrieve {
    [DllImport("WBTRV32.DLL", CallingConvention = CallingConvention.StdCall)]
    static extern short BTRCALL(ushort op, byte[] pos, byte[] data, ref int len, byte[] key, short keyLen, short keyNum);

    const ushort OPEN = 0, CLOSE = 1, GET_NEXT = 6, GET_FIRST = 12, STAT = 15, STOP = 25;
    const short END_OF_FILE = 9;

    static void Check(short status, string what) {
        if (status != 0) throw new Exception(what + ": Btrieve status " + status);
    }

    public static string Dump(string file, string outDir) {
        var pos = new byte[128];
        var data = new byte[65535];
        var key = new byte[255];
        var name = Encoding.ASCII.GetBytes(file + "\0");
        Array.Copy(name, key, name.Length);
        int len = 0;
        Check(BTRCALL(OPEN, pos, data, ref len, key, 255, 0), "open " + file);
        try {
            // Stat: record count at +6 (4 bytes), key count at +4 (2 bytes) of the file spec
            len = data.Length;
            Check(BTRCALL(STAT, pos, data, ref len, new byte[255], 255, 0), "stat");
            int keys = BitConverter.ToInt16(data, 4);
            uint records = BitConverter.ToUInt32(data, 6);
            File.WriteAllText(Path.Combine(outDir, "stat.txt"), "records " + records + "\nkeys " + keys + "\n");
            for (short k = 0; k < keys; k++) {
                using (var w = new BinaryWriter(File.Create(Path.Combine(outDir, "key" + k + ".bin")))) {
                    ushort op = GET_FIRST;
                    while (true) {
                        len = data.Length;
                        short s = BTRCALL(op, pos, data, ref len, key, 255, k);
                        if (s == END_OF_FILE) break;
                        Check(s, "key " + k + " read");
                        w.Write(len);
                        w.Write(data, 0, len);
                        op = GET_NEXT;
                    }
                }
            }
            return "records " + records + ", keys " + keys;
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
        $dir = Join-Path $Out (Split-Path $f -Leaf)
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        $result = [Btrieve]::Dump($f, $dir)
        Write-Output "$f`: $result"
    }
} finally {
    [Btrieve]::Stop()
}
