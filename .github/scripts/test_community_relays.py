import unittest

import community_relays


def entry(url):
    return {
        "url": url,
        "operator": "Ada",
        "github": "ada",
        "region": "us-east",
        "added": "2026-10-20",
    }


class AdvanceTest(unittest.TestCase):
    def test_three_failures_remove_and_a_success_resets(self):
        entries = [entry("https://up.example"), entry("https://down.example")]
        kept, failures, removed = community_relays.advance(entries, {}, {"https://up.example": True, "https://down.example": False})
        self.assertEqual([e["url"] for e in kept], ["https://up.example", "https://down.example"])
        self.assertEqual(failures, {"https://down.example": 1})
        self.assertEqual(removed, [])

        kept, failures, removed = community_relays.advance(kept, failures, {"https://up.example": True, "https://down.example": False})
        self.assertEqual(failures, {"https://down.example": 2})
        self.assertEqual(removed, [])

        kept, failures, removed = community_relays.advance(kept, failures, {"https://up.example": True, "https://down.example": False})
        self.assertEqual([e["url"] for e in kept], ["https://up.example"])
        self.assertEqual(failures, {})
        self.assertEqual([e["url"] for e in removed], ["https://down.example"])

        kept, failures, removed = community_relays.advance(kept, {"https://up.example": 2}, {"https://up.example": True})
        self.assertEqual(failures, {})
        self.assertEqual(removed, [])

    def test_an_empty_list_removes_nothing(self):
        self.assertEqual(community_relays.advance([], {"https://old.example": 9}, {}), ([], {}, []))

    def test_new_entries_must_be_https_and_healthy(self):
        base = {"relays": []}
        head = {"relays": [entry("https://ok.example"), entry("http://nope.example")]}
        seen = []

        def fake_probe(url):
            seen.append(url)
            return url == "https://ok.example"

        errors = community_relays.check_pr(base, head, fake_probe)
        self.assertTrue(any("https://" in err for err in errors))
        self.assertEqual(seen, ["https://ok.example"])

        refused = {"relays": [entry("https://use1-1.relay.n0.iroh.link")]}
        errors = community_relays.check_pr(base, refused, lambda url: True)
        self.assertTrue(any("refused" in err for err in errors))

        same = community_relays.check_pr({"relays": [entry("https://ok.example")]}, {"relays": [entry("https://ok.example")]}, lambda url: False)
        self.assertEqual(same, [])


if __name__ == "__main__":
    unittest.main()
