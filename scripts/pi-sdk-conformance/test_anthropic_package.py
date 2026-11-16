import unittest
from run_anthropic_package import check_execution, check_telemetry_feature


class ExecutionTests(unittest.TestCase):
    def test_complete(self):
        check_execution("test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;")

    def test_filtered_ignored_or_missing_cannot_pass(self):
        for text in ("", "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out;",
                     "test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out;"):
            with self.assertRaises(ValueError):check_execution(text)

    def test_companion_tracing_must_match(self):
        metadata={"resolve":{"nodes":[{"id":"companion","features":["tracing"]}]}}
        self.assertEqual(check_telemetry_feature(metadata,"companion",True),["tracing"])
        with self.assertRaises(ValueError):check_telemetry_feature(metadata,"companion",False)
        metadata["resolve"]["nodes"][0]["features"]=[]
        with self.assertRaises(ValueError):check_telemetry_feature(metadata,"companion",True)

if __name__=="__main__":unittest.main()
