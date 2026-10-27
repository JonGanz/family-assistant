"""Helpers for the design workflow. Import from repo root:  import sys; sys.path.insert(0, "tools"); from fa import *
- answered()                          -> {path: answer_text} for open questions with answers
- new_question(n, slug, title, area, batch, question, options=[])
- new_decision(id, slug, title, qs, affects, context, decision, consequences, alternatives, supersedes="")
- resolve(qid, decisions, date)       -> set front-matter + git mv to design/resolved/
- supersede(old_id, new_id, note)     -> mark old decision superseded + banner + index status
- annotate(dec_id, note)              -> add an "Amended/Refined by" banner to a decision
- rebuild_design_index()              -> regenerate open-batch tables + Resolved table from front-matter
- edit(path, [(old, new), ...])       -> exact replacements, asserting each old string exists
"""
import glob, re, subprocess, datetime

TODAY = datetime.date.today().isoformat()


def _fm(text, key):
    m = re.search(rf"^{key}: (.*)$", text, re.M)
    return m.group(1).strip() if m else ""


def answered():
    out = {}
    for f in sorted(glob.glob("design/Q-*.md")):
        a = open(f).read().split("<!-- answer below this line -->", 1)[-1].strip()
        if a:
            out[f] = a
    return out


def new_question(n, slug, title, area, batch, question, options=(), extra_fm=""):
    qid = f"Q-{n:03d}"
    o = ("\n## Options\n" + "\n".join(f"- {x}" for x in options) + "\n") if options else ""
    path = f"design/{qid}-{slug}.md"
    open(path, "w").write(f"---\nid: {qid}\ntitle: {title}\nstatus: open\nbatch: {batch}\narea: {area}\ncreated: {TODAY}\n"
                          f"resolved:\ndecisions: []\n{extra_fm}---\n\n# {qid}: {title}\n\n## Question\n{question}\n{o}\n## Answer\n<!-- answer below this line -->\n\n")
    return path


def new_decision(did, slug, title, qs, affects, context, decision, consequences, alternatives, supersedes="", extra_fm=""):
    path = f"decisions/{did}-{slug}.md"
    open(path, "w").write(f"---\nid: {did}\ntitle: {title}\nstatus: accepted\ndate: {TODAY}\nsource_questions: [{', '.join(qs)}]\n"
                          f"supersedes: {supersedes}\nsuperseded_by:\naffects: [{affects}]\n{extra_fm}---\n\n# {did}: {title}\n\n"
                          f"## Context\n{context}\n\n## Decision\n{decision}\n\n## Consequences\n{consequences}\n\n## Alternatives considered\n{alternatives}\n")
    open("decisions/_index.md", "a").write(f"| [{did}]({did}-{slug}.md) | {title} | accepted | {', '.join(qs)} | {affects} |\n")
    return path


def resolve(qid, decisions, date=TODAY):
    [f] = glob.glob(f"design/{qid}-*.md")
    s = open(f).read().replace("status: open", "status: resolved").replace("resolved:\n", f"resolved: {date}\n")
    s = s.replace("decisions: []", f"decisions: [{', '.join(decisions)}]")
    open(f, "w").write(s)
    subprocess.run(["git", "mv", f, "design/resolved/"], check=True)


def _banner(path, note):
    s = open(path).read(); i = s.index("\n# D-")
    open(path, "w").write(s[:i] + f"\n> {note}\n" + s[i:])


def annotate(did, note):
    [p] = glob.glob(f"decisions/{did}-*.md"); _banner(p, note)


def supersede(old, new, note):
    [p] = glob.glob(f"decisions/{old}-*.md")
    s = open(p).read().replace("status: accepted", "status: superseded", 1).replace("superseded_by:\n", f"superseded_by: {new}\n", 1)
    open(p, "w").write(s)
    [n] = glob.glob(f"decisions/{new}-*.md")
    _banner(p, f"**Superseded by [{new}]({n.split('/')[-1]}).** {note}")
    idx = open("decisions/_index.md").read()
    idx = re.sub(rf"(\| \[{old}\][^\n]*?\| )accepted( \|)", rf"\1superseded → {new}\2", idx)
    open("decisions/_index.md", "w").write(idx)


def rebuild_design_index():
    s = open("design/_index.md").read()
    head = re.split(r"\n## (?:Batch \d+|Open)", s)[0].rstrip() + "\n\n"
    open_q = {}
    for f in sorted(glob.glob("design/Q-*.md")):
        t = open(f).read()
        open_q.setdefault(_fm(t, "batch"), []).append(f"| [{_fm(t, 'id')}]({f.split('/')[-1]}) | {_fm(t, 'title')} | {_fm(t, 'area')} |")
    body = ""
    for b in sorted(open_q, key=int):
        body += f"## Batch {b}, open\n| ID | Title | Area |\n|---|---|---|\n" + "\n".join(open_q[b]) + "\n\n"
    if not open_q:
        nums = [int(re.search(r"Q-(\d+)", f).group(1)) for f in glob.glob("design/**/Q-*.md", recursive=True)]
        body = f"## Open\n_None. The next question number is Q-{max(nums) + 1:03d}._\n\n"
    res = []
    for f in sorted(glob.glob("design/resolved/Q-*.md")):
        t = open(f).read()
        res.append(f"| [{_fm(t, 'id')}](resolved/{f.split('/')[-1]}) | {_fm(t, 'title')} | {_fm(t, 'decisions').strip('[]')} |")
    open("design/_index.md", "w").write(head + body + "## Resolved\n| ID | Title | Decisions |\n|---|---|---|\n" + "\n".join(res) + "\n")


def edit(path, pairs):
    s = open(path).read()
    for a, b in pairs:
        assert a in s, (path, a[:80])
        s = s.replace(a, b)
    open(path, "w").write(s)


# ---------------------------------------------------------------- check (CLI)
import os


def check_yaml(files):
    """Parses each file with PyYAML, or with js-yaml from the web app's node_modules. Returns errors."""
    try:
        import yaml
    except ImportError:
        yaml = None
    if yaml:
        bad = []
        for f in files:
            try:
                yaml.safe_load(open(f, encoding="utf-8"))
            except Exception as e:  # noqa: BLE001
                bad.append(f"{f}: {e}")
        return bad
    libs = sorted(glob.glob("apps/web/node_modules/.pnpm/js-yaml@*/node_modules/js-yaml"))
    if not libs:
        return ["no YAML parser: install PyYAML or run `corepack pnpm@9 install` in apps/web"]
    js = ("const y=require(process.argv[1]),fs=require('fs');let bad=0;"
          "for(const f of process.argv.slice(2)){try{y.load(fs.readFileSync(f,'utf8'))}catch(e){console.log(f+': '+e.message);bad=1}}"
          "process.exit(bad)")
    r = subprocess.run(["node", "-e", js, os.path.abspath(libs[-1]), *files], capture_output=True, text=True)
    return [l for l in r.stdout.splitlines() if l] or ([r.stderr.strip()] if r.returncode else [])


def parse_fm(path):
    """Return dict of front-matter keys (raw string values) or None if there is none."""
    lines = open(path, encoding="utf-8").read().split("\n")
    if not lines or lines[0].strip() != "---":
        return None
    d = {}
    for ln in lines[1:]:
        if ln.strip() == "---":
            return d
        m = re.match(r"^([A-Za-z_][\w-]*):\s*(.*)$", ln)
        if m:
            d[m.group(1)] = m.group(2).strip().strip("\"'")
    return None


_REQ = {
    "Q": "id title status batch area created resolved decisions",
    "D": "id title status date source_questions supersedes superseded_by affects",
    "arch": "title scope status updated drawio depends_on",
    "log": "date topics",
    "R": "id title date method verified informs",
    "plan": "title milestone status updated arch decisions",
    "T": "id title milestone wave depends_on agent status owns",
}


def check():
    errs = []
    ids = {"Q": {}, "D": {}}

    def err(p, m):
        errs.append(f"{p}: {m}")

    def need(p, kind, enum=None):
        fm = parse_fm(p)
        if fm is None:
            err(p, "missing front-matter")
            return None
        for k in _REQ[kind].split():
            if k not in fm:
                err(p, f"missing key '{k}'")
        if enum and fm.get("status") not in enum:
            err(p, f"status '{fm.get('status')}' not in {sorted(enum)}")
        return fm

    base = lambda p: os.path.basename(p)
    for p in sorted(glob.glob("design/Q-*.md") + glob.glob("design/resolved/Q-*.md")):
        fm = need(p, "Q", {"open", "answered", "resolved"})
        m = re.match(r"(Q-\d+)", base(p))
        if fm:
            if "/resolved/" in p and fm.get("status") != "resolved":
                err(p, "file under resolved/ must have status resolved")
            if m and fm.get("id") != m.group(1):
                err(p, f"id '{fm.get('id')}' does not match filename")
        if m:
            ids["Q"].setdefault(m.group(1), []).append(p)
    for p in sorted(glob.glob("decisions/D-*.md")):
        fm = need(p, "D", {"accepted", "superseded"})
        m = re.match(r"(D-\d+)", base(p))
        if fm:
            if m and fm.get("id") != m.group(1):
                err(p, f"id '{fm.get('id')}' does not match filename")
            if fm.get("status") == "superseded" and not fm.get("superseded_by"):
                err(p, "superseded decision has empty superseded_by")
        if m:
            ids["D"].setdefault(m.group(1), []).append(p)
    for p in sorted(glob.glob("arch/*.md")):
        if base(p) == "_index.md":
            continue
        fm = need(p, "arch", {"draft", "accepted"})
        if fm and fm.get("drawio") and not os.path.exists(os.path.join(os.path.dirname(p), fm["drawio"])) \
                and not os.path.exists(fm["drawio"]):
            err(p, f"drawio file '{fm['drawio']}' not found")
    for p in sorted(glob.glob("log/*.md")):
        if re.fullmatch(r"\d{4}-\d{2}-\d{2}\.md", base(p)):
            need(p, "log")
    for p in sorted(glob.glob("research/R-*.md")):
        need(p, "R")
    for p in sorted(glob.glob("plan/M*.md")):
        need(p, "plan")
    for p in sorted(glob.glob("plan/M*/T-*.md")):
        fm = need(p, "T", {"todo", "running", "done", "blocked"})
        # P-10: briefs from M3.5 on must cite their decisions.
        if fm and str(fm.get("milestone")) not in ("M0", "M1", "M2", "M3") and "decisions" not in fm:
            err(p, "missing key 'decisions'")

    for k, d in ids.items():
        for i, ps in d.items():
            if len(ps) > 1:
                err(ps[0], f"duplicate {i}: {', '.join(ps)}")

    for kind, idx in (("Q", "design/_index.md"), ("D", "decisions/_index.md")):
        txt = open(idx, encoding="utf-8").read() if os.path.exists(idx) else ""
        linked = set(re.findall(rf"\[({kind}-\d+)\]", txt))
        for i in sorted(ids[kind]):
            if i not in linked:
                err(idx, f"{i} not listed in index")
        for i in sorted(linked - set(ids[kind])):
            err(idx, f"{i} listed in index but no file on disk")
    for idx in sorted(glob.glob("*/_index.md")):
        fm = parse_fm(idx)
        if fm is None or "updated" not in fm:
            err(idx, "missing 'updated' key")
    return errs


if __name__ == "__main__":
    import argparse, sys
    ap = argparse.ArgumentParser(prog="fa.py")
    ap.add_argument("cmd", choices=["check", "check-yaml"])
    ap.add_argument("files", nargs="*")
    a = ap.parse_args()
    os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
    if a.cmd == "check-yaml":
        bad = check_yaml(a.files)
        print("\n".join(bad) if bad else f"yaml OK ({len(a.files)} file(s))")
        sys.exit(1 if bad else 0)
    e = check()
    for line in e:
        print(line)
    print(f"{len(e)} failure(s)" if e else "docs check OK")
    sys.exit(1 if e else 0)
