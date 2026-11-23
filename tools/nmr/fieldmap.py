"""Join NMR vO's record types (VB members) to their PALN32 field maps (record bytes).

Prints, per record type, every VB element: name[index], VB type, record offset, record
length, FLD type. Usage: python3 fieldmap.py vO.bas [Type ...]
"""
import re
import sys
import json

VB_SIZES = {"Long": 4, "Integer": 2, "Byte": 1, "Double": 8, "Single": 4, "Currency": 8}


def strip_comment(line):
    out, q = [], False
    for ch in line:
        if ch == '"':
            q = not q
        if ch == "'" and not q:
            break
        out.append(ch)
    return "".join(out).strip()


def parse(src):
    lines = [strip_comment(l) for l in src.splitlines()]
    types, maps = {}, {}
    i = 0
    while i < len(lines):
        l = lines[i]
        m = re.match(r"(?:Public |Private )?Type (\w+)RecType$", l, re.I)
        if m:
            name = m.group(1)
            members = []
            i += 1
            while not lines[i].startswith("End Type"):
                mm = re.match(r"(\w+)(?:\((?:(\d+)\s+To\s+)?(\d+)\))?\s+As\s+(String\s*\*\s*(\d+)|\w+)", lines[i])
                if mm:
                    lo = int(mm.group(2) or 0)
                    count = int(mm.group(3)) - lo + 1 if mm.group(3) else None
                    if mm.group(5):
                        members.append((mm.group(1), count, "String", int(mm.group(5))))
                    else:
                        members.append((mm.group(1), count, mm.group(4), VB_SIZES[mm.group(4)]))
                i += 1
            types[name.lower()] = members
        m = re.match(r"Sub Add(\w+)FieldMap\(", l)
        if m:
            name = m.group(1)
            entries = []
            i += 1
            while not lines[i].startswith("End Sub"):
                mm = re.match(r"AddField Map, ctr, (FLD_\w+), (\d+)", lines[i])
                if mm:
                    entries.append((mm.group(1), int(mm.group(2))))
                elif lines[i] and not lines[i].startswith("'"):
                    raise SystemExit(f"Add{name}FieldMap: unexpected line {lines[i]!r}")
                i += 1
            maps[name.lower()] = entries
        i += 1
    return types, maps


def join(members, entries, tname):
    out, off, k = [], 0, 0
    for name, count, vbtype, size in members:
        for idx in range(count or 1):
            if k >= len(entries):
                raise SystemExit(f"{tname}: field map ran out at {name}")
            fld, length = entries[k]
            k += 1
            out.append({
                "member": name, "index": idx if count else None, "vb": vbtype,
                "vb_size": size, "fld": fld, "offset": off, "len": length,
            })
            off += length
    if k != len(entries):
        raise SystemExit(f"{tname}: {len(entries) - k} field map entries left over")
    return out, off


if __name__ == "__main__":
    src = open(sys.argv[1], encoding="latin-1").read()
    types, maps = parse(src)
    want = [t.lower() for t in sys.argv[2:]] or sorted(set(types) & set(maps))
    result = {}
    for t in want:
        elems, total = join(types[t], maps[t], t)
        result[t] = {"len": total, "elements": elems}
        print(f"{t}: {len(elems)} elements, {total} bytes", file=sys.stderr)
    json.dump(result, sys.stdout, indent=1)
