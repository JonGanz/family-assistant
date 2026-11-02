"""Tests for tools/fa.py: negative tests for `check` (P-15) and every CLI subcommand (P-06), each in a temp tree.
Run: python3 tools/test_fa.py"""
import os, subprocess, sys, tempfile, unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fa  # noqa: E402

FA = os.path.join(os.path.dirname(os.path.abspath(__file__)), "fa.py")
TODAY = "2031-05-06"

GOOD_Q = "---\nid: Q-001\ntitle: t\nstatus: open\nbatch: 1\narea: infra\ncreated: 2026-01-01\nresolved:\ndecisions: []\n---\n# Q-001\n"


class CheckTest(unittest.TestCase):
    def tree(self, files):
        d = tempfile.mkdtemp()
        for p, s in files.items():
            os.makedirs(os.path.join(d, os.path.dirname(p)), exist_ok=True)
            with open(os.path.join(d, p), "w") as fh:
                fh.write(s)
        cwd = os.getcwd()
        os.chdir(d)
        try:
            return fa.check()
        finally:
            os.chdir(cwd)

    def base(self):
        idx = "---\ntitle: i\nupdated: 2026-01-01\n---\n"
        return {"design/_index.md": idx + "[Q-001](Q-001-a.md)\n", "decisions/_index.md": idx,
                "design/Q-001-a.md": GOOD_Q}

    def test_good_tree_passes(self):
        self.assertEqual(self.tree(self.base()), [])

    def test_missing_key_fails(self):
        t = self.base()
        t["design/Q-001-a.md"] = GOOD_Q.replace("batch: 1\n", "")
        self.assertTrue(any("batch" in e for e in self.tree(t)))

    def test_bad_status_fails(self):
        t = self.base()
        t["design/Q-001-a.md"] = GOOD_Q.replace("status: open", "status: maybe")
        self.assertTrue(any("status" in e for e in self.tree(t)))

    def test_unindexed_question_fails(self):
        t = self.base()
        t["design/_index.md"] = "---\ntitle: i\nupdated: 2026-01-01\n---\n"
        self.assertTrue(any("not listed in index" in e for e in self.tree(t)))

    def test_new_task_without_decisions_fails(self):
        t = self.base()
        t["plan/M9/T-01-x.md"] = ("---\nid: T-01\ntitle: x\nmilestone: M9\nwave: 1\ndepends_on: []\n"
                                  "agent: a\nstatus: todo\nowns: []\n---\n")
        self.assertTrue(any("decisions" in e for e in self.tree(t)))

    def test_bad_yaml_fails(self):
        d = tempfile.mkdtemp()
        p = os.path.join(d, "bad.yml")
        with open(p, "w") as fh:
            fh.write("a: [1, 2\n")
        self.assertTrue(fa.check_yaml([p]))


QIDX = ("---\ntitle: Design Questions Index\nupdated: 2020-01-01\n---\n\n# Design Questions\n\n## Template\n```\n## Options (optional)\n```\n\n"
        "## Open\n_None. The next question number is Q-001._\n\n## Resolved\n| ID | Title | Decisions |\n|---|---|---|\n")
DIDX = "---\ntitle: Decisions Index\nupdated: 2020-01-01\n---\n\n| ID | Title | Status | Source Qs | Affects |\n|---|---|---|---|---|\n"
LIDX = "---\ntitle: Log Index\nupdated: 2020-01-01\n---\n\n# Daily Logs\n\n"
HUMAN = ("---\ntitle: Human steps\nupdated: 2020-01-01\n---\n\n| Step | Milestone | What | Status | Verified in |\n|---|---|---|---|---|\n"
         "| H-01 | M0 | Push the repo | done (2026-10-24) | log |\n| H-02 | M0 | Wire the doorbell | open | shakedown |\n")
PLAN = "---\ntitle: P\nmilestone: M9\nstatus: {s}\nupdated: 2026-01-01\narch: []\ndecisions: []\n---\n"
TASK = ("---\nid: T-0{n}\ntitle: \"Task {n}\"\nmilestone: M9\nwave: 1\ndepends_on: []\nagent: a\nstatus: {s}\nowns: []\ndecisions: []\n---\n")
ARGS_D = ["--slug", "use-x", "--title", "Use X: for Y", "--questions", "Q-001", "--affects", "arch/A.md,arch/B.md",
          "--context", "ctx", "--decision", "dec", "--consequences", "con", "--alternatives", "alt"]


class CliTest(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        for p, s in {"design/_index.md": QIDX, "decisions/_index.md": DIDX, "log/_index.md": LIDX,
                     "plan/human-steps.md": HUMAN, "plan/M9.md": PLAN.format(s="running"),
                     "plan/M9/T-01-a.md": TASK.format(n=1, s="done"), "plan/M9/T-02-b.md": TASK.format(n=2, s="todo"),
                     "plan/M8.md": PLAN.format(s="code-complete"), "plan/M8/T-01-a.md": TASK.format(n=1, s="todo")}.items():
            self.write(p, s)
        os.makedirs(os.path.join(self.d, "design/resolved"))
        subprocess.run(["git", "init", "-q"], cwd=self.d, check=True)

    def write(self, p, s):
        os.makedirs(os.path.dirname(os.path.join(self.d, p)), exist_ok=True)
        with open(os.path.join(self.d, p), "w") as fh:
            fh.write(s)

    def read(self, p):
        with open(os.path.join(self.d, p)) as fh:
            return fh.read()

    def fa(self, *args, ok=True):
        env = dict(os.environ, FA_ROOT=self.d, FA_TODAY=TODAY)
        r = subprocess.run([sys.executable, FA, *args], cwd=self.d, env=env, capture_output=True, text=True)
        if ok:
            self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        return r

    def newq(self, slug="first", extra=()):
        return self.fa("new-q", "--slug", slug, "--title", "Which one?", "--area", "tech", "--batch", "1",
                       "--question", "Pick.", *extra)

    def test_help_lists_every_subcommand(self):
        out = self.fa("--help").stdout
        for c in ["check", "check-yaml", "next-id", "new-q", "new-d", "resolve", "supersede", "annotate", "log", "status",
                  "rebuild-design-index"]:
            self.assertIn(c, out)

    def test_new_q_then_check_passes_and_stamps_index(self):
        r = self.newq(extra=["--option", "A", "--option", "B (configurable, default A)"])
        self.assertIn("design/Q-001-first.md", r.stdout)
        q = self.read("design/Q-001-first.md")
        self.assertIn(f"created: {TODAY}", q)
        self.assertIn("- A\n- B (configurable, default A)", q)
        idx = self.read("design/_index.md")
        self.assertIn("[Q-001](Q-001-first.md)", idx)
        self.assertIn(f"updated: {TODAY}", idx)
        self.assertEqual(self.fa("check").stdout.strip(), "docs check OK")

    def test_new_q_numbers_follow_the_highest_across_open_and_resolved(self):
        self.write("design/resolved/Q-007-old.md", GOOD_Q.replace("Q-001", "Q-007").replace("status: open", "status: resolved"))
        self.assertEqual(self.fa("next-id", "--kind", "q").stdout.strip(), "Q-008")
        self.newq()
        self.assertTrue(os.path.exists(os.path.join(self.d, "design/Q-008-first.md")))
        self.assertIn("Q-009", self.fa("next-id", "--kind", "q").stdout)
        r = self.fa("new-q", "--n", "8", "--slug", "x", "--title", "t", "--area", "a", "--batch", "1", "--question", "q", ok=False)
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("already exists", r.stderr)

    def test_new_q_rejects_positional_arguments(self):
        r = self.fa("new-q", "5", "slug", "Title", "tech", "1", "Question?", ok=False)
        self.assertEqual(r.returncode, 2)
        self.assertIn("error:", r.stderr)
        r = self.fa("new-q", "5", "--slug", "s", "--title", "t", "--area", "a", "--batch", "1", "--question", "q", ok=False)
        self.assertEqual(r.returncode, 2)
        self.assertIn("unrecognized arguments: 5", r.stderr)
        self.assertFalse(os.listdir(os.path.join(self.d, "design/resolved")))
        self.assertFalse([f for f in os.listdir(os.path.join(self.d, "design")) if f.startswith("Q-")])

    def test_at_file_text_and_literal_at(self):
        self.write("q.txt", "A long\nmulti-line question.\n")
        self.fa("new-q", "--slug", "long", "--title", "@@handle", "--area", "tech", "--batch", "2", "--question", "@q.txt")
        q = self.read("design/Q-001-long.md")
        self.assertIn("## Question\nA long\nmulti-line question.\n", q)
        self.assertIn("# Q-001: @@handle".replace("@@", "@"), q)
        self.assertIn('title: "@handle"', q)  # quoted: a bare leading @ is not valid YAML
        self.assertIn("## Batch 2, open", self.read("design/_index.md"))
        self.assertEqual(self.fa("check").stdout.strip(), "docs check OK")

    def test_new_d_with_colon_title_and_index_row(self):
        r = self.fa("new-d", "--id", "D-001", *ARGS_D)
        self.assertIn("decisions/D-001-use-x.md", r.stdout)
        d = self.read("decisions/D-001-use-x.md")
        self.assertIn('title: "Use X: for Y"', d)
        self.assertIn("affects: [arch/A.md, arch/B.md]", d)
        self.assertIn("source_questions: [Q-001]", d)
        idx = self.read("decisions/_index.md")
        self.assertIn("| [D-001](D-001-use-x.md) | Use X: for Y | accepted | Q-001 | arch/A.md, arch/B.md |", idx)
        self.assertIn(f"updated: {TODAY}", idx)
        self.assertEqual(self.fa("next-id", "--kind", "d").stdout.strip(), "D-002")
        self.assertNotEqual(self.fa("new-d", "--id", "D-001", *ARGS_D, ok=False).returncode, 0)

    def test_new_d_default_id_and_instruction(self):
        self.fa("new-d", "--slug", "s", "--title", "T", "--affects", "arch/A.md", "--context", "c", "--decision", "d",
                "--consequences", "c", "--alternatives", "a")
        self.assertIn("| accepted | (instruction) |", self.read("decisions/_index.md"))
        self.assertIn("source_questions: []", self.read("decisions/D-001-s.md"))

    def test_resolve_moves_with_git_and_rebuilds_index(self):
        self.newq()
        subprocess.run(["git", "add", "-A"], cwd=self.d, check=True)
        subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@example.com", "commit", "-qm", "x"], cwd=self.d, check=True)
        self.fa("new-d", "--id", "D-001", *ARGS_D)
        self.fa("resolve", "--q", "Q-001", "--decisions", "D-001,D-002", "--date", "2031-01-02")
        self.assertFalse(os.path.exists(os.path.join(self.d, "design/Q-001-first.md")))
        q = self.read("design/resolved/Q-001-first.md")
        self.assertIn("status: resolved", q)
        self.assertIn("resolved: 2031-01-02", q)
        self.assertIn("decisions: [D-001, D-002]", q)
        st = subprocess.run(["git", "status", "--short"], cwd=self.d, capture_output=True, text=True).stdout
        self.assertRegex(st, r"R.*design/Q-001-first.md -> design/resolved/Q-001-first.md")
        idx = self.read("design/_index.md")
        self.assertIn("[Q-001](resolved/Q-001-first.md) | Which one? | D-001, D-002", idx)
        self.assertIn("next question number is Q-002", idx)
        self.fa("resolve", "--q", "Q-009", "--decisions", "D-001", ok=False)
        self.assertEqual(self.fa("check").stdout.strip(), "docs check OK")

    def test_resolve_works_for_an_untracked_question(self):
        self.newq()
        self.fa("resolve", "--q", "Q-001", "--decisions", "D-001")
        self.assertTrue(os.path.exists(os.path.join(self.d, "design/resolved/Q-001-first.md")))

    def test_supersede_and_annotate(self):
        self.fa("new-d", "--id", "D-001", *ARGS_D)
        self.fa("new-d", "--id", "D-002", "--slug", "use-z", "--title", "Use Z", "--affects", "arch/A.md", "--context", "c",
                "--decision", "d", "--consequences", "c", "--alternatives", "a")
        self.fa("supersede", "--old", "D-001", "--new", "D-002", "--note", "Z replaces X.")
        old = self.read("decisions/D-001-use-x.md")
        self.assertIn("status: superseded", old)
        self.assertIn("superseded_by: D-002", old)
        self.assertIn("> **Superseded by [D-002](D-002-use-z.md).** Z replaces X.", old)
        self.assertIn("supersedes: D-001", self.read("decisions/D-002-use-z.md"))
        self.assertIn("superseded → D-002", self.read("decisions/_index.md"))
        self.fa("annotate", "--d", "D-002", "--note", "Amended by the doorbell change.")
        self.assertIn("> Amended by the doorbell change.", self.read("decisions/D-002-use-z.md"))
        self.assertEqual(self.fa("check").stdout.strip(), "docs check OK")

    def test_log_creates_then_appends(self):
        r = self.fa("log", "--topics", "alpha,beta", "--text", "- did a thing", "--heading", "First")
        self.assertIn(f"log/{TODAY}.md", r.stdout)
        log = self.read(f"log/{TODAY}.md")
        self.assertTrue(log.startswith(f"---\ndate: {TODAY}\ntopics: [alpha, beta]\n---\n"))
        self.assertIn("## First\n- did a thing\n", log)
        idx = self.read("log/_index.md")
        self.assertIn(f"- [{TODAY}]({TODAY}.md): alpha; beta", idx)
        self.assertIn(f"updated: {TODAY}", idx)
        self.write("t.txt", "second body")
        self.fa("log", "--topics", "beta,gamma", "--text", "@t.txt", "--heading", "Second")
        log = self.read(f"log/{TODAY}.md")
        self.assertIn("topics: [alpha, beta, gamma]", log)
        self.assertIn("## First\n- did a thing\n\n## Second\nsecond body\n", log)
        self.assertEqual(self.read("log/_index.md").count(f"[{TODAY}]"), 1)
        self.assertEqual(self.fa("check").stdout.strip(), "docs check OK")

    def test_status_lists_open_question_and_human_step(self):
        self.newq()
        s = self.fa("status")
        self.assertIn("STATUS.md written", s.stdout)
        st = self.read("STATUS.md")
        self.assertIn("[Q-001](design/Q-001-first.md): Which one? (batch 1)", st)
        self.assertIn("H-02 (M0): Wire the doorbell", st)
        self.assertNotIn("H-01", st)
        self.assertIn("M9 (running)", st)
        self.assertIn("T-02: Task 2 (todo)", st)
        self.assertNotIn("Task 1", st)  # done
        self.assertNotIn("M8", st)  # closed plan
        self.assertIn(f"on {TODAY}", st)
        self.write("design/Q-001-first.md", self.read("design/Q-001-first.md") + "Yes, X.\n")
        self.fa("status")
        self.assertIn("answer waiting", self.read("STATUS.md"))

    def test_answered_and_rebuild_design_index(self):
        self.newq()
        self.assertEqual(self.fa("answered").stdout.strip(), "")
        self.write("design/Q-001-first.md", self.read("design/Q-001-first.md") + "Yes, X.\n")
        self.assertIn("Yes, X.", self.fa("answered").stdout)
        self.write("design/_index.md", self.read("design/_index.md").replace("updated: " + TODAY, "updated: 2020-01-01"))
        self.fa("rebuild-design-index")
        self.assertIn(f"updated: {TODAY}", self.read("design/_index.md"))


if __name__ == "__main__":
    os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
    unittest.main()
