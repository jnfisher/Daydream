"""Generate Daydream's NMR .mdb column table from NMR's own source.

Joins, per column of NMR's `CreateAccessTables`: the record element NMR's Access export
writes into it (frmDatabaseExport), that element's byte span in NMR's vO field maps (the
Worldgroup 3 layouts), and the daydream-types WG_* member that span falls in.

Usage: python3 gen.py create.bas vO.json exports.json layout.rs > generated.rs
"""
import json
import re
import sys

NMR_TYPE = {
    "Rooms": "room", "Classes": "class", "Races": "race", "Spells": "spell",
    "Monsters": "monster", "Items": "item", "Shops": "shop", "Messages": "message",
    "Textblocks": "textblock", "Actions": "action",
}
WG_LAYOUT = {
    "room": "WG_ROOM", "class": "WG_CLASS", "race": "WG_RACE", "spell": "WG_SPELL",
    "monster": "WG_MONSTER", "item": "WG_ITEM", "shop": "WG_SHOP", "message": "WG_MUDMSG",
    "textblock": "WG_TEXTBLK", "action": "WG_ACTION",
}
DOS_LAYOUT = {k: v[3:] for k, v in WG_LAYOUT.items()}
DOS_LAYOUT["message"] = "MUDMSG"


def strip_comment(line):
    out, q = [], False
    for ch in line:
        if ch == '"':
            q = not q
        if ch == "'" and not q:
            break
        out.append(ch)
    return "".join(out).strip()


def parse_schema(src):
    tables, cur, loops, indexes = [], None, [], []
    lines = [strip_comment(l) for l in src.splitlines()]
    i = 0
    key = None
    while i < len(lines):
        l = lines[i]
        m = re.match(r'\.Name = "(\w+)"$', l)
        if m and key is None:
            cur = {"name": m.group(1), "columns": []}
            tables.append(cur)
        m2 = re.match(r"With (pk\w+|idx\w+)$", l)
        if m2:
            key = {"name": m2.group(1), "columns": [], "primary": False, "table": None}
            indexes.append(key)
        if l == "End With":
            key = None
        if key is not None:
            m = re.match(r'\.Columns\.Append "(.+)"$', l)
            if m:
                key["columns"].append(m.group(1))
            if l == ".Type = adKeyPrimary":
                key["primary"] = True
            m = re.match(r'\.RelatedTable = "(\w+)"$', l)
            if m:
                key["table"] = m.group(1)
        m = re.match(r"For x = (\d+) To (\d+)$", l)
        if m:
            loops.append((i, int(m.group(1)), int(m.group(2))))
        elif l == "Next":
            start, x, hi = loops.pop()
            if x < hi:
                loops.append((start, x + 1, hi))
                i = start + 1
                continue
        elif key is None and cur is not None:
            m = re.match(r'(?:If .* Then )?\.Columns\.Append (.+?), (ad\w+)(?:, (\d+))?$', l)
            if m:
                expr = m.group(1)
                x = loops[-1][1] if loops else None
                name = "".join(
                    p.strip().strip('"') if p.strip().startswith('"') else str(x)
                    for p in re.sub(r"^CStr\(|^\(|\)$", "", expr).split("&")
                )
                cur["columns"].append({
                    "name": name, "type": m.group(2),
                    "size": int(m.group(3)) if m.group(3) else None,
                    "optional": l.startswith("If "),
                })
        m = re.match(r"tabNew(\w+)\.Indexes\.Append (idx\w+)$", l)
        if m:
            next(k for k in indexes if k["name"] == m.group(2))["table"] = m.group(1)
        i += 1
    return tables, indexes


def parse_layouts(src):
    layouts = {}
    for m in re.finditer(r"pub const (\w+): Layout = Layout \{(.*?)\n\};", src, re.S):
        body = m.group(2)
        length = int(re.search(r"len: (\d+)", body).group(1))
        fields = [
            (f.group(1), int(f.group(2), 16), int(f.group(3)), int(f.group(4)))
            for f in re.finditer(r'f\("(\w+)", 0x([0-9A-Fa-f]+), (\d+), (\d+)\)', body)
        ]
        layouts[m.group(1)] = {"len": length, "fields": fields}
    return layouts


def pieces(layout, off, length):
    """The members covering bytes off..off+length: (member, offset in it, byte count)."""
    out, end = [], off + length
    for name, start, size, elem in layout["fields"]:
        lo, hi = max(off, start), min(end, start + size)
        if lo < hi:
            out.append([name, lo - start, hi - lo])
    if sum(p[2] for p in out) != length:
        raise SystemExit(f"span {off}+{length} not covered")
    return out


def main():
    tables, indexes = parse_schema(open(sys.argv[1], encoding="latin-1").read())
    nmr = json.load(open(sys.argv[2]))
    exports = json.load(open(sys.argv[3]))
    layouts = parse_layouts(open(sys.argv[4]).read())
    out = []
    for t in tables:
        name = t["name"]
        if name == "Info":
            out.append({"table": name, "columns": [
                {"name": c["name"], "type": c["type"], "size": c["size"], "source": None}
                for c in t["columns"]]})
            continue
        rtype = NMR_TYPE[name]
        elems = nmr[rtype]["elements"]
        wg = layouts[WG_LAYOUT[rtype]]
        dos = layouts[DOS_LAYOUT[rtype]]
        assert nmr[rtype]["len"] <= wg["len"], (name, nmr[rtype]["len"], wg["len"])
        srcs = {}
        for e in exports[name]:
            srcs.setdefault(e["column"], e)  # first assignment: the default branch
        cols = []
        for c in t["columns"]:
            s = srcs.get(c["name"])
            if s is None:
                raise SystemExit(f"{name}.{c['name']}: no export source")
            src = None
            if "member" in s:
                el = next(
                    e for e in elems
                    if e["member"].lower() == s["member"].lower() and e["index"] == s["index"]
                )
                dos_names = {f[0] for f in dos["fields"]}
                ps = pieces(wg, el["offset"], el["len"])
                for p in ps:
                    if p[0] not in dos_names and not p[0].startswith("pad_"):
                        raise SystemExit(f"{name}.{c['name']}: {p[0]} missing from DOS layout")
                src = {
                    "nmr": f"{el['member']}" + (f"({el['index']})" if el["index"] is not None else ""),
                    "vb": el["vb"], "len": el["len"], "pieces": ps, "wrap": s.get("wrap"),
                }
            else:
                src = {"expr": s["expr"]}
            cols.append({"name": c["name"], "type": c["type"], "size": c["size"],
                         "optional": c["optional"], "source": src})
        out.append({"table": name, "columns": cols})
    json.dump({"tables": out, "indexes": indexes}, sys.stdout, indent=1)


main()
