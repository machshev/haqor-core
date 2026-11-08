"""Regression checks for the full-corpus alignment review inventory."""
import importlib.util
from pathlib import Path
import sqlite3
import unittest

spec = importlib.util.spec_from_file_location(
    "audit", Path(__file__).with_name("audit-peshitta-links.py"))
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


class AlignmentReviewTests(unittest.TestCase):
    def setUp(self):
        self.db = sqlite3.connect(":memory:")
        self.addCleanup(self.db.close)
        self.db.executescript("""
            CREATE TABLE nt_word(ref INTEGER, ord INTEGER, word_id INTEGER);
            CREATE TABLE syriac_word(word_id INTEGER, vocalised TEXT);
            INSERT INTO nt_word VALUES (2621697, 0, 1), (2621697, 1, 2);
            INSERT INTO syriac_word VALUES (1, 'ܪܘܚܐ'), (2, 'ܩܘܕܫܐ');
        """)
        self.rows = {(40, 1, 1): "[Holy|1] [Spirit|0]."}

    def test_individual_links_can_follow_english_order(self):
        result = audit.audit(self.rows, self.db, {})
        self.assertEqual(result["source_words"], 2)
        self.assertEqual(result["shared_spans"], 0)
        self.assertEqual(result["pending_chapters"], 1)

    def test_missing_and_out_of_range_links_fail(self):
        for text in ["[Spirit|0]", "[Holy|0] [Spirit|0]", "[Holy|2] [Spirit|0]"]:
            with self.subTest(text=text), self.assertRaises(ValueError):
                audit.audit({(40, 1, 1): text}, self.db, {})

    def test_discontinuous_english_can_share_one_source_owner(self):
        rows = {(40, 1, 1): "[and by|0] [another|1] [way|0]."}
        result = audit.audit(rows, self.db, {})
        self.assertEqual(result["source_words"], 2)
        self.assertEqual(result["shared_spans"], 0)

    def test_cross_verse_unlinked_empty_and_malformed_spans_fail(self):
        for text in ["[Holy Spirit|1:0,1]", "Holy [Spirit|0,1]", "[|0,1]",
                     "[Holy Spirit|0,0,1]", "[Holy Spirit|0,1",
                     "{[Holy Spirit|0,1]", "[Holy Spirit|0,1]}"]:
            with self.subTest(text=text), self.assertRaises(ValueError):
                audit.spans(text)

    def test_complete_verse_inventory_required(self):
        with self.assertRaisesRegex(ValueError, "verse inventory"):
            audit.audit({}, self.db, {})

    def test_exception_needs_exact_text_positions_reason_and_review_hash(self):
        self.rows = {(40, 1, 1): "[Holy Spirit|0,1]."}
        checkpoint = {"sha256": audit.chapter_hash(self.rows, 40, 1),
                      "independent_review": True,
                      "exceptions": [{"verse": 1, "english": "Holy Spirit",
                                      "positions": [0, 1], "reason": "test fixture"}]}
        ledger = {"chapters": {"40:1": checkpoint}}
        self.assertEqual(audit.audit(self.rows, self.db, ledger)["reviewed_chapters"], 1)
        checkpoint["exceptions"][0]["reason"] = ""
        with self.assertRaisesRegex(ValueError, "linguistic reason"):
            audit.audit(self.rows, self.db, ledger)
        checkpoint["exceptions"] = []
        with self.assertRaisesRegex(ValueError, "exception inventory"):
            audit.audit(self.rows, self.db, ledger)
        checkpoint["sha256"] = "stale"
        with self.assertRaisesRegex(ValueError, "hash is stale"):
            audit.audit(self.rows, self.db, ledger)


if __name__ == "__main__":
    unittest.main()
