#!/usr/bin/env bash
# Regenerates crates/daydream-nmr/src/columns.rs from a Nightmare Redux checkout.
#
#   tools/nmr/generate.sh path/to/Nightmare-Redux
#
# Reads, from NMR's source: CreateAccessTables (modMMUDFunctions.bas), the Access branch of
# every Export* routine (frmDatabaseExport.frm) and the vO record types and field maps
# (modFieldmaps_vO.bas, the build for MajorMUD 1.11o and 1.11p).
set -euo pipefail
nmr=${1:?usage: generate.sh NMR_DIR}
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

tr -d '\r' < "$nmr/modMMUDFunctions.bas" \
  | awk '/^Public Function CreateAccessTables/{on=1} on{print} on&&/^End Function/{exit}' > "$work/create.bas"
tr -d '\r' < "$nmr/frmDatabaseExport.frm" > "$work/export.frm"
tr -d '\r' < "$nmr/modFieldmaps_vO.bas" > "$work/vO.bas"

python3 -I "$here/fieldmap.py" "$work/vO.bas" > "$work/vO.json"
python3 -I "$here/exports.py" "$work/export.frm" > "$work/exports.json"
python3 -I "$here/gen.py" "$work/create.bas" "$work/vO.json" "$work/exports.json" \
  "$root/crates/daydream-types/src/records/layout.rs" > "$work/joined.json"
python3 -I "$here/gen_rust.py" "$work/joined.json" > "$root/crates/daydream-nmr/src/columns.rs"
echo "wrote crates/daydream-nmr/src/columns.rs"
