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
