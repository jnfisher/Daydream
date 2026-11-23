"""Read NMR's Access-format export routines: which record element fills each column.

Takes the normal branch of every `If chkZeroUserInteraction.Value = 1` block (that option
is handled by hand). Usage: python3 exports.py export.frm > exports.json
"""
import json
import re
import sys

TABLES = {
    "Rooms": "Roomrec", "Spells": "Spellrec", "Actions": "Actionrec", "Classes": "Classrec",
    "Races": "Racerec", "Shops": "Shoprec", "Monsters": "Monsterrec", "Messages": "Messagerec",
    "Items": "Itemrec", "Textblocks": "TextblockRec",
}


def strip_comment(line):
    out, q = [], False
    for ch in line:
        if ch == '"':
            q = not q
        if ch == "'" and not q:
            break
        out.append(ch)
    return "".join(out).strip()


def vb_eval(expr, env):
    if "&" in expr:
        return "".join(str(vb_eval(part.strip(), env)) for part in expr.split("&"))
    py = re.sub(r"\bCStr\b", "", expr)
    py = re.sub(r"\bx\b", "X", py)
    return eval(py, {"X": env.get("x")}, {})


def section(src, table):
    lines = src.splitlines()
    start = next(i for i, l in enumerate(lines) if l.startswith(f"Private Sub Export{table}("))
    acc = next(i for i in range(start, len(lines)) if lines[i].startswith("Access:"))
    end = next(i for i in range(acc, len(lines)) if lines[i].startswith("End Sub"))
    return [strip_comment(l) for l in lines[acc:end]]


def columns(lines, table, rec):
    tab = f"tab{table}"
    out = []
    i = 0
    loops = []  # (start index, var value, hi)
    skip_depth = None  # nesting depth inside a skipped zero-interaction branch
    depth = 0
    while i < len(lines):
        l = lines[i]
        if re.match(r"If .* Then$", l):
            depth += 1
            if "chkZeroUserInteraction" in l and skip_depth is None:
                skip_depth = depth
            i += 1
            continue
        if l == "Else" or l.startswith("ElseIf"):
            if skip_depth == depth:
                skip_depth = -depth  # now in the normal branch
            i += 1
            continue
        if l == "End If":
            if skip_depth is not None and abs(skip_depth) == depth:
                skip_depth = None
            depth -= 1
            i += 1
            continue
        if skip_depth is not None and skip_depth > 0:
            i += 1
            continue
        m = re.match(r"For x = (\d+) To (\d+)$", l)
        if m:
            loops.append((i, int(m.group(1)), int(m.group(2))))
            i += 1
            continue
        if l == "Next":
            start, x, hi = loops.pop()
            if x < hi:
                loops.append((start, x + 1, hi))
                i = start + 1
            else:
                i += 1
            continue
        m = re.match(rf'{tab}\.Fields\((.+?)\) = (.+)$', l)
        if m:
            env = {"x": loops[-1][1]} if loops else {}
            col = vb_eval(m.group(1), env)
            rhs = m.group(2)
            wrap = None
            w = re.match(r"(\w+)\((.*)\)$", rhs)
            if w and not w.group(1).startswith(rec):
                wrap, rhs = w.group(1), w.group(2)
            r = re.match(rf"{rec}\.(\w+)(?:\((.+)\))?$", rhs)
            if r:
                idx = vb_eval(r.group(2), env) if r.group(2) else None
                out.append({"column": col, "member": r.group(1), "index": idx, "wrap": wrap})
            else:
                out.append({"column": col, "expr": m.group(2)})
        i += 1
    return out


if __name__ == "__main__":
    src = open(sys.argv[1], encoding="latin-1").read()
    result = {t: columns(section(src, t), t, r) for t, r in TABLES.items()}
    for t, cols in result.items():
        print(t, len(cols), file=sys.stderr)
    json.dump(result, sys.stdout, indent=1)
