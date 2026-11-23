# Writes an NMR-format .mdb from a Daydream plan, through Windows' own Jet 4.0 engine.
#
#   powershell -ExecutionPolicy Bypass -File write-mdb.ps1 realm.plan realm.mdb
#
# Jet 4.0 is 32-bit, so this re-runs itself under the 32-bit Windows PowerShell when started
# from a 64-bit one. The tables, columns and keys are created through ADOX exactly as Nightmare
# Redux's CreateAccessTables creates them; the rows go in through OLE DB in one transaction.
# The plan format is described in crates/daydream-nmr/src/jet.rs.
param(
    [Parameter(Mandatory = $true)][string]$Plan,
    [Parameter(Mandatory = $true)][string]$Out
)
$ErrorActionPreference = 'Stop'

if ([IntPtr]::Size -ne 4) {
    $ps32 = Join-Path $env:SystemRoot 'SysWOW64\WindowsPowerShell\v1.0\powershell.exe'
    & $ps32 -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $PSCommandPath $Plan $Out
    exit $LASTEXITCODE
}

$Plan = (Resolve-Path $Plan).Path
$Out = [IO.Path]::GetFullPath($Out)
if (Test-Path $Out) { Remove-Item $Out }

Add-Type -ReferencedAssemblies System.Data -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Data;
using System.Data.OleDb;
using System.Globalization;
using System.IO;
using System.Text;

public class PlanColumn { public string Name; public string Type; public int Size; }
public class PlanIndex { public string Name; public bool Primary; public List<string> Columns = new List<string>(); }
public class PlanTable {
    public string Name;
    public List<PlanColumn> Columns = new List<PlanColumn>();
    public List<PlanIndex> Indexes = new List<PlanIndex>();
    public List<string[]> Rows = new List<string[]>();
}

public static class DaydreamPlan {
    public static List<PlanTable> Read(string path) {
        var tables = new List<PlanTable>();
        PlanTable cur = null;
        foreach (var line in File.ReadLines(path, new UTF8Encoding(false))) {
            if (line.Length == 0) continue;
            var f = line.Split('\t');
            switch (f[0]) {
                case "T": cur = new PlanTable { Name = f[1] }; tables.Add(cur); break;
                case "C": cur.Columns.Add(new PlanColumn { Name = f[1], Type = f[2], Size = int.Parse(f[3], CultureInfo.InvariantCulture) }); break;
                case "K":
                    var k = new PlanIndex { Name = f[1], Primary = f[2] == "primary" };
                    for (int i = 3; i < f.Length; i++) k.Columns.Add(f[i]);
                    cur.Indexes.Add(k);
                    break;
                case "R":
                    var row = new string[f.Length - 1];
                    Array.Copy(f, 1, row, 0, row.Length);
                    if (row.Length != cur.Columns.Count) throw new Exception(cur.Name + ": row has " + row.Length + " values for " + cur.Columns.Count + " columns");
                    cur.Rows.Add(row);
                    break;
                default: throw new Exception("bad plan line: " + f[0]);
            }
        }
        return tables;
    }

    static string Unescape(string s) {
        var sb = new StringBuilder(s.Length);
        for (int i = 1; i < s.Length; i++) {
            char c = s[i];
            if (c != '\\') { sb.Append(c); continue; }
            char n = s[++i];
            switch (n) {
                case 't': sb.Append('\t'); break;
                case 'n': sb.Append('\n'); break;
                case 'r': sb.Append('\r'); break;
                case '0': sb.Append('\0'); break;
                default: sb.Append(n); break;
            }
        }
        return sb.ToString();
    }

    static object Value(string v) {
        switch (v[0]) {
            case 'N': return DBNull.Value;
            case 'L': return int.Parse(v.Substring(1), CultureInfo.InvariantCulture);
            case 'D': return double.Parse(v.Substring(1), NumberStyles.Float, CultureInfo.InvariantCulture);
            case 'S': return Unescape(v);
            default: throw new Exception("bad value: " + v);
        }
    }

    public static void Insert(string mdb, List<PlanTable> tables) {
        using (var conn = new OleDbConnection("Provider=Microsoft.Jet.OLEDB.4.0;Data Source=" + mdb)) {
            conn.Open();
            using (var tx = conn.BeginTransaction()) {
                foreach (var t in tables) {
                    var names = new List<string>();
                    var marks = new List<string>();
                    foreach (var c in t.Columns) { names.Add("[" + c.Name + "]"); marks.Add("?"); }
                    var cmd = new OleDbCommand("INSERT INTO [" + t.Name + "] (" + string.Join(", ", names) + ") VALUES (" + string.Join(", ", marks) + ")", conn, tx);
                    foreach (var c in t.Columns) {
                        OleDbType ty;
                        switch (c.Type) {
                            case "Long": ty = OleDbType.Integer; break;
                            case "Double": ty = OleDbType.Double; break;
                            case "Text": ty = OleDbType.VarWChar; break;
                            default: ty = OleDbType.LongVarWChar; break;
                        }
                        var p = cmd.Parameters.Add("@" + cmd.Parameters.Count, ty);
                        // Prepare needs a size for every variable-length parameter
                        p.Size = c.Type == "Text" ? c.Size : c.Type == "Memo" ? 65535 : 0;
                    }
                    cmd.Prepare();
                    foreach (var row in t.Rows) {
                        for (int i = 0; i < row.Length; i++) cmd.Parameters[i].Value = Value(row[i]);
                        cmd.ExecuteNonQuery();
                    }
                }
                tx.Commit();
            }
        }
    }
}
'@

$tables = [DaydreamPlan]::Read($Plan)

# ADOX, as CreateAccessTables: adInteger 3, adDouble 5, adVarWChar 202, adLongVarWChar 203
$types = @{ Long = 3; Double = 5; Text = 202; Memo = 203 }
$conn = "Provider=Microsoft.Jet.OLEDB.4.0;Data Source=$Out"
$cat = New-Object -ComObject ADOX.Catalog
[void]$cat.Create($conn)
foreach ($t in $tables) {
    $table = New-Object -ComObject ADOX.Table
    $table.Name = $t.Name
    foreach ($c in $t.Columns) {
        if ($c.Type -eq 'Text' -or $c.Type -eq 'Memo') {
            $table.Columns.Append($c.Name, $types[$c.Type], $c.Size)
        } else {
            $table.Columns.Append($c.Name, $types[$c.Type])
        }
    }
    [void]$cat.Tables.Append($table)
}
foreach ($t in $tables) {
    $table = $cat.Tables.Item($t.Name)
    foreach ($k in $t.Indexes) {
        if ($k.Primary) {
            $key = New-Object -ComObject ADOX.Key
            $key.Name = $k.Name
            $key.Type = 1          # adKeyPrimary
            foreach ($col in $k.Columns) { $key.Columns.Append($col) }
            $table.Keys.Append($key)
        } else {
            $index = New-Object -ComObject ADOX.Index
            $index.Name = $k.Name
            $index.Unique = $false
            foreach ($col in $k.Columns) { $index.Columns.Append($col) }
            $table.Indexes.Append($index)
        }
    }
}
$cat.ActiveConnection.Close()
[void][Runtime.InteropServices.Marshal]::ReleaseComObject($cat)
[GC]::Collect()
[GC]::WaitForPendingFinalizers()

[DaydreamPlan]::Insert($Out, $tables)
$rows = 0
foreach ($t in $tables) { $rows += $t.Rows.Count }
Write-Output "wrote $Out ($($tables.Count) tables, $rows rows)"
