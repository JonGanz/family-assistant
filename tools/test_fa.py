"""Negative tests for `fa.py check` (P-15): a broken tree must fail. Run: python3 tools/test_fa.py"""
import os, sys, tempfile, unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fa  # noqa: E402

GOOD_Q = "---\nid: Q-001\ntitle: t\nstatus: open\nbatch: 1\narea: infra\ncreated: 2026-01-01\nresolved:\ndecisions: []\n---\n# Q-001\n"


class CheckTest(unittest.TestCase):
    def tree(self, files):
        d = tempfile.mkdtemp()
        for p, s in files.items():
            os.makedirs(os.path.join(d, os.path.dirname(p)), exist_ok=True)
            open(os.path.join(d, p), "w").write(s)
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
        open(p, "w").write("a: [1, 2\n")
        self.assertTrue(fa.check_yaml([p]))


if __name__ == "__main__":
    os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
    unittest.main()
