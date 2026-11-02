"""Design-workflow tooling, as a CLI and as an importable library.

CLI (keyword flags only; a long text flag also takes `@path` to read the text from a file, `@@x` is a literal `@x`):
    python3 tools/fa.py --help
Library (import from repo root:  import sys; sys.path.insert(0, "tools"); from fa import *), paths relative to the cwd:
- answered()                          -> {path: answer_text} for open questions with answers
- new_question(n, slug, title, area, batch, question, options=[])
- new_decision(id, slug, title, qs, affects, context, decision, consequences, alternatives, supersedes="")
- resolve(qid, decisions, date)       -> set front-matter + git mv to design/resolved/
- supersede(old_id, new_id, note)     -> mark old decision superseded + banner + index status
- annotate(dec_id, note)              -> add an "Amended/Refined by" banner to a decision
- rebuild_design_index()              -> regenerate open-batch tables + Resolved table from front-matter
- edit(path, [(old, new), ...])       -> exact replacements, asserting each old string exists
Every writer that touches an index stamps its `updated:` date (FA_TODAY=YYYY-MM-DD overrides today, for tests).
"""
import glob, re, subprocess, datetime, os, shutil


def today():
    return os.environ.get("FA_TODAY") or datetime.date.today().isoformat()


TODAY = today()


def _fm(text, key):
    m = re.search(rf"^{key}: (.*)$", text, re.M)
    return m.group(1).strip() if m else ""


def _yq(s):
    """A front-matter scalar: quoted when a bare value would be read as something else by YAML."""
    s = str(s)
    if s and (re.search(r": |\s#|^[\s\-?:,\[\]{}&*!|>'\"%@`#]|\s$", s) or s.lower() in ("null", "true", "false", "yes", "no", "~")):
        return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'
    return s


def stamp(index_path, date=None):
    """Set `updated:` in the front-matter of an index file (inserted if missing)."""
    s = open(index_path, encoding="utf-8").read()
    d = date or today()
    m = re.match(r"---\n(.*?\n)---\n", s, re.S)
    if not m:
        s = f"---\nupdated: {d}\n---\n" + s
    elif re.search(r"^updated:.*$", m.group(1), re.M):
        fm = re.sub(r"^updated:.*$", f"updated: {d}", m.group(1), count=1, flags=re.M)
        s = f"---\n{fm}---\n" + s[m.end():]
    else:
        s = f"---\n{m.group(1)}updated: {d}\n---\n" + s[m.end():]
    open(index_path, "w", encoding="utf-8").write(s)


def _append_line(path, line):
    s = open(path, encoding="utf-8").read()
    open(path, "w", encoding="utf-8").write(s + ("" if s.endswith("\n") or not s else "\n") + line + "\n")


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
    open(path, "w").write(f"---\nid: {qid}\ntitle: {_yq(title)}\nstatus: open\nbatch: {batch}\narea: {area}\ncreated: {today()}\n"
                          f"resolved:\ndecisions: []\n{extra_fm}---\n\n# {qid}: {title}\n\n## Question\n{question}\n{o}\n## Answer\n<!-- answer below this line -->\n\n")
    return path


def new_decision(did, slug, title, qs, affects, context, decision, consequences, alternatives, supersedes="", extra_fm=""):
    path = f"decisions/{did}-{slug}.md"
    open(path, "w").write(f"---\nid: {did}\ntitle: {_yq(title)}\nstatus: accepted\ndate: {today()}\nsource_questions: [{', '.join(qs)}]\n"
                          f"supersedes: {supersedes}\nsuperseded_by:\naffects: [{affects}]\n{extra_fm}---\n\n# {did}: {title}\n\n"
                          f"## Context\n{context}\n\n## Decision\n{decision}\n\n## Consequences\n{consequences}\n\n## Alternatives considered\n{alternatives}\n")
    _append_line("decisions/_index.md", f"| [{did}]({did}-{slug}.md) | {title.replace('|', '/')} | accepted | {', '.join(qs) or '(instruction)'} | {affects} |")
    stamp("decisions/_index.md")
    return path


def resolve(qid, decisions, date=None):
    date = date or today()
    [f] = glob.glob(f"design/{qid}-*.md")
    s = open(f).read()
    s = re.sub(r"^status: .*$", "status: resolved", s, count=1, flags=re.M)
    s = re.sub(r"^resolved:[ \t]*$", f"resolved: {date}", s, count=1, flags=re.M)
    s = re.sub(r"^decisions: .*$", f"decisions: [{', '.join(decisions)}]", s, count=1, flags=re.M)
    open(f, "w").write(s)
    os.makedirs("design/resolved", exist_ok=True)
    if subprocess.run(["git", "mv", f, "design/resolved/"], capture_output=True).returncode != 0:
        shutil.move(f, "design/resolved/")  # not a tracked file (or not a git checkout): a plain move


def _banner(path, note):
    s = open(path).read(); i = s.index("\n# D-")
    open(path, "w").write(s[:i] + f"\n> {note}\n" + s[i:])


def annotate(did, note):
    [p] = glob.glob(f"decisions/{did}-*.md"); _banner(p, note)


def supersede(old, new, note):
    [p] = glob.glob(f"decisions/{old}-*.md")
    [n] = glob.glob(f"decisions/{new}-*.md")
    idx = open("decisions/_index.md").read()
    idx, hits = re.subn(rf"(\| \[{old}\][^\n]*?\| )accepted( \|)", rf"\1superseded → {new}\2", idx)
    assert hits == 1, f"{old} has no 'accepted' row in decisions/_index.md"
    s = open(p).read().replace("status: accepted", "status: superseded", 1).replace("superseded_by:\n", f"superseded_by: {new}\n", 1)
    open(p, "w").write(s)
    _banner(p, f"**Superseded by [{new}]({n.split('/')[-1]}).** {note}")
    ns = open(n).read()
    if re.search(r"^supersedes:[ \t]*$", ns, re.M):
        open(n, "w").write(re.sub(r"^supersedes:[ \t]*$", f"supersedes: {old}", ns, count=1, flags=re.M))
    open("decisions/_index.md", "w").write(idx)
    stamp("decisions/_index.md")


def rebuild_design_index():
    s = open("design/_index.md").read()
    head = re.split(r"\n## (?:Batch \d+|Open)", s)[0].rstrip() + "\n\n"
    open_q = {}
    for f in sorted(glob.glob("design/Q-*.md")):
        t = open(f).read()
        open_q.setdefault(_fm(t, "batch"), []).append(f"| [{_fm(t, 'id')}]({f.split('/')[-1]}) | {_fm(t, 'title').strip(chr(34))} | {_fm(t, 'area')} |")
    body = ""
    for b in sorted(open_q, key=int):
        body += f"## Batch {b}, open\n| ID | Title | Area |\n|---|---|---|\n" + "\n".join(open_q[b]) + "\n\n"
    if not open_q:
        nums = [int(re.search(r"Q-(\d+)", f).group(1)) for f in glob.glob("design/**/Q-*.md", recursive=True)]
        body = f"## Open\n_None. The next question number is Q-{max(nums, default=0) + 1:03d}._\n\n"
    res = []
    for f in sorted(glob.glob("design/resolved/Q-*.md")):
        t = open(f).read()
        res.append(f"| [{_fm(t, 'id')}](resolved/{f.split('/')[-1]}) | {_fm(t, 'title').strip(chr(34))} | {_fm(t, 'decisions').strip('[]')} |")
    open("design/_index.md", "w").write(head + body + "## Resolved\n| ID | Title | Decisions |\n|---|---|---|\n" + "\n".join(res) + "\n")
    stamp("design/_index.md")


def edit(path, pairs):
    s = open(path).read()
    for a, b in pairs:
        assert a in s, (path, a[:80])
        s = s.replace(a, b)
    open(path, "w").write(s)


def next_number(kind):
    """Highest existing number + 1: questions over design/ and design/resolved/, decisions over decisions/."""
    pat, files = (("Q", glob.glob("design/Q-*.md") + glob.glob("design/resolved/Q-*.md")) if kind == "q"
                  else ("D", glob.glob("decisions/D-*.md")))
    nums = [int(m.group(1)) for f in files if (m := re.match(rf"{pat}-(\d+)", os.path.basename(f)))]
    return max(nums, default=0) + 1


def log_entry(topics, text, heading=None, summary=None, date=None):
    """Append a `## ` section to log/<date>.md (created with front-matter if missing); topics merge into the front-matter
    list; log/_index.md gets its line if the day is new. Returns the log path."""
    d = date or today()
    path = f"log/{d}.md"
    os.makedirs("log", exist_ok=True)
    heading = heading or (", ".join(topics) if topics else "Notes")
    section = f"## {heading}\n{text.strip()}\n"
    if not os.path.exists(path):
        open(path, "w").write(f"---\ndate: {d}\ntopics: [{', '.join(topics)}]\n---\n\n# {d}\n\n{section}")
        line = f"- [{d}]({d}.md): {summary or '; '.join(topics) or heading}"
        if os.path.exists("log/_index.md"):
            _append_line("log/_index.md", line)
            stamp("log/_index.md")
        return path
    s = open(path).read()
    m = re.search(r"^topics: \[(.*)\]$", s, re.M)
    if m and topics:
        have = [t.strip() for t in m.group(1).split(",") if t.strip()]
        add = [t for t in topics if t not in have]
        if add:
            s = s[:m.start()] + f"topics: [{', '.join(have + add)}]" + s[m.end():]
    open(path, "w").write(s.rstrip("\n") + "\n\n" + section)
    return path


def _plan_closed(status):
    return bool(re.match(r"\s*(code-complete|closed|done|complete)", status or "", re.I))


def status_md():
    """Build STATUS.md: open questions, open human steps, unfinished tasks of unclosed plans, newest log."""
    out = [f"# Status\n\n_Generated by `python3 tools/fa.py status` on {today()}. Do not edit by hand._\n"]
    ans = answered()
    qs = []
    for f in sorted(glob.glob("design/Q-*.md")):
        t = open(f).read()
        qs.append(f"- [{_fm(t, 'id')}]({f}): {_fm(t, 'title').strip(chr(34))} (batch {_fm(t, 'batch')})" + (" **answer waiting to be processed**" if f in ans else ""))
    out.append("## Open questions\n" + ("\n".join(qs) if qs else "None.") + "\n")
    steps = []
    if os.path.exists("plan/human-steps.md"):
        for ln in open("plan/human-steps.md", encoding="utf-8").read().splitlines():
            c = [x.strip() for x in ln.strip().strip("|").split("|")]
            if ln.startswith("|") and len(c) >= 4 and re.match(r"[A-Z]+-\d+$", c[0]) and c[3].lower().startswith("open"):
                steps.append(f"- {c[0]} ({c[1]}): {c[2]}")
    out.append("## Open human steps\n" + ("\n".join(steps) if steps else "None.") + "\n")
    tasks = []
    for p in sorted(glob.glob("plan/M*.md")):
        fm = parse_fm(p) or {}
        if _plan_closed(fm.get("status")):
            continue
        ms = os.path.basename(p)[:-3]
        rows = []
        for t in sorted(glob.glob(f"plan/{ms}/T-*.md")):
            tf = parse_fm(t) or {}
            if tf.get("status") != "done":
                rows.append(f"  - {tf.get('id', os.path.basename(t))}: {tf.get('title', '')} ({tf.get('status', '?')})")
        if rows:
            tasks += [f"- {ms} ({fm.get('status', '')})"] + rows
    out.append("## Tasks not done (plans not closed)\n" + ("\n".join(tasks) if tasks else "None.") + "\n")
    logs = sorted(os.path.basename(f)[:-3] for f in glob.glob("log/*.md") if re.fullmatch(r"\d{4}-\d{2}-\d{2}\.md", os.path.basename(f)))
    out.append("## Newest log\n" + (f"[{logs[-1]}](log/{logs[-1]}.md)" if logs else "None.") + "\n")
    return "\n".join(out)


import argparse, sys

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




# ---------------------------------------------------------------- CLI
def _text(s):
    """`@path` reads the text from a file; `@@x` is a literal `@x`."""
    if s.startswith("@@"):
        return s[1:]
    if s.startswith("@"):
        with open(s[1:], encoding="utf-8") as fh:
            return fh.read().rstrip("\n")
    return s


def _list(s):
    return [x.strip() for x in _text(s).split(",") if x.strip()]


def _slug(s):
    if not re.fullmatch(r"[a-z0-9]+(-[a-z0-9]+)*", s):
        raise argparse.ArgumentTypeError(f"slug '{s}' must be lowercase words joined by '-'")
    return s


def _qid(s):
    if not re.fullmatch(r"Q-\d{3,}", s):
        raise argparse.ArgumentTypeError(f"'{s}' is not a question id like Q-001")
    return s


def _did(s):
    if not re.fullmatch(r"D-\d{3,}", s):
        raise argparse.ArgumentTypeError(f"'{s}' is not a decision id like D-001")
    return s


def _date(s):
    try:
        datetime.date.fromisoformat(s)
    except ValueError:
        raise argparse.ArgumentTypeError(f"'{s}' is not YYYY-MM-DD")
    return s


def build_parser():
    ap = argparse.ArgumentParser(prog="fa.py", description="Design-workflow tooling. Keyword flags only; a long text flag "
                                 "also takes @path to read the text from a file. Use --flag=-text for text that starts with '-'.")
    ap.add_argument("--root", help="repo root (default: $FA_ROOT, else the directory above tools/)")
    sub = ap.add_subparsers(dest="cmd", required=True, metavar="<subcommand>")

    def cmd(name, help_):
        return sub.add_parser(name, help=help_, description=help_)

    cmd("check", "lint the design tree (front-matter, enums, id/filename parity, index parity)")
    p = cmd("check-yaml", "parse YAML files")
    p.add_argument("files", nargs="+")
    p = cmd("next-id", "print the next free question or decision number")
    p.add_argument("--kind", choices=["q", "d"], required=True)
    cmd("answered", "print open questions that have an answer waiting")
    p = cmd("new-q", "create design/Q-NNN-slug.md and rebuild design/_index.md")
    p.add_argument("--n", type=int, help="question number (default: next free)")
    p.add_argument("--slug", type=_slug, required=True)
    p.add_argument("--title", type=_text, required=True)
    p.add_argument("--area", required=True)
    p.add_argument("--batch", type=int, required=True)
    p.add_argument("--question", type=_text, required=True)
    p.add_argument("--option", type=_text, action="append", default=[], help="repeatable")
    p = cmd("new-d", "create decisions/D-NNN-slug.md and append it to decisions/_index.md")
    p.add_argument("--id", type=_did, help="decision id (default: next free)")
    p.add_argument("--slug", type=_slug, required=True)
    p.add_argument("--title", type=_text, required=True)
    p.add_argument("--questions", type=_list, default=[], help="comma list (empty: a direct user instruction)")
    p.add_argument("--affects", type=_list, required=True, help="comma list of arch files")
    p.add_argument("--context", type=_text, required=True)
    p.add_argument("--decision", type=_text, required=True)
    p.add_argument("--consequences", type=_text, required=True)
    p.add_argument("--alternatives", type=_text, required=True)
    p.add_argument("--supersedes", type=_did, help="only records the link; `supersede` marks the old one")
    p = cmd("resolve", "mark a question resolved, git mv it to design/resolved/, rebuild the index")
    p.add_argument("--q", type=_qid, required=True)
    p.add_argument("--decisions", type=_list, required=True)
    p.add_argument("--date", type=_date)
    p = cmd("supersede", "mark a decision superseded by another")
    p.add_argument("--old", type=_did, required=True)
    p.add_argument("--new", type=_did, required=True)
    p.add_argument("--note", type=_text, required=True)
    p = cmd("annotate", "add an Amended/Refined banner to a decision")
    p.add_argument("--d", type=_did, required=True)
    p.add_argument("--note", type=_text, required=True)
    p = cmd("log", "append a section to log/<today>.md (created with front-matter and an index line if new)")
    p.add_argument("--topics", type=_list, required=True)
    p.add_argument("--text", type=_text, required=True)
    p.add_argument("--heading", type=_text, help="section heading (default: the topics)")
    p.add_argument("--summary", type=_text, help="log/_index.md line for a new day (default: the topics)")
    cmd("status", "write STATUS.md: open questions, open human steps, unfinished tasks, newest log")
    cmd("rebuild-design-index", "regenerate design/_index.md from the question files")
    return ap


def main(argv=None):
    a = build_parser().parse_args(argv)
    root = a.root or os.environ.get("FA_ROOT") or os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
    os.chdir(root)

    def fail(msg):
        print(f"error: {msg}", file=sys.stderr)
        return 1

    c = a.cmd
    if c == "check-yaml":
        bad = check_yaml(a.files)
        print("\n".join(bad) if bad else f"yaml OK ({len(a.files)} file(s))")
        return 1 if bad else 0
    if c == "check":
        e = check()
        for line in e:
            print(line)
        print(f"{len(e)} failure(s)" if e else "docs check OK")
        return 1 if e else 0
    if c == "next-id":
        n = next_number(a.kind)
        print(f"{'Q' if a.kind == 'q' else 'D'}-{n:03d}")
    elif c == "answered":
        for f, t in answered().items():
            print(f"=== {f}\n{t}")
    elif c == "new-q":
        n = a.n or next_number("q")
        if glob.glob(f"design/Q-{n:03d}-*.md") or glob.glob(f"design/resolved/Q-{n:03d}-*.md"):
            return fail(f"Q-{n:03d} already exists (numbers are never reused)")
        print(new_question(n, a.slug, a.title, a.area, a.batch, a.question, a.option))
        rebuild_design_index()
    elif c == "new-d":
        did = a.id or f"D-{next_number('d'):03d}"
        if glob.glob(f"decisions/{did}-*.md"):
            return fail(f"{did} already exists")
        print(new_decision(did, a.slug, a.title, a.questions, ", ".join(a.affects), a.context, a.decision,
                           a.consequences, a.alternatives, a.supersedes or ""))
    elif c == "resolve":
        if not glob.glob(f"design/{a.q}-*.md"):
            return fail(f"{a.q} is not an open question in design/")
        resolve(a.q, a.decisions, a.date)
        rebuild_design_index()
    elif c == "supersede":
        supersede(a.old, a.new, a.note)
    elif c == "annotate":
        annotate(a.d, a.note)
    elif c == "log":
        print(log_entry(a.topics, a.text, a.heading, a.summary))
    elif c == "status":
        open("STATUS.md", "w", encoding="utf-8").write(status_md())
        print("STATUS.md written")
    elif c == "rebuild-design-index":
        rebuild_design_index()
    return 0


if __name__ == "__main__":
    sys.exit(main())
