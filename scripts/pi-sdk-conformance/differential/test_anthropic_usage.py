import copy
import unittest
from compare_anthropic_usage import compare, ENDINGS, ROUTES


def traces():
    fixture=[{"id":"zero","expected":[0,0,0,0]}]
    result={"text":"AB","usage":[0]*4,"cost":0,"stop":"stop","raw_stop":"end_turn","response_id":"usage-response"}
    def row(route,ending,rust):
        final={**result,"stop":"stop" if ending=="success" else "error"}
        observed=route in ("simple","prepared-observed")
        attrs={"cost.total":0,"stop":final["stop"],**{f"usage.{k}":0 for k in ("input","output","cache_read","cache_write")}}
        metrics=[{"name":"pi.ai.tokens","value":0,"attributes":{"kind":k}} for k in ("input","output","cache_read","cache_write")]
        metrics += [{"name":n,"value":0,"attributes":{}} for n in ("pi.ai.cost","pi.ai.request.duration","pi.ai.first_chunk.duration")]
        initial={**result,"text":"A","usage":[11,7,5,3],"cost":52e-6,"stop":"pending","raw_stop":None}
        events=[{"kind":"text_delta","delta":"A","message":initial},
                {"kind":"text_delta","delta":"B","message":{**initial,"text":"AB"}},
                {"kind":"text_end","message":{**initial,"text":"AB"}},
                {"kind":"done" if ending=="success" else "error","message":final}]
        return {"id":"zero","route":route,"ending":ending,"result":final,"events":events,"delivery":"ok" if ending=="success" else "error",
                "held":{**result,"text":"A","usage":[11,7,5,3],"cost":52e-6,"stop":"pending"} if rust else final.copy(),
                "requests":1,"metrics":metrics if observed else [],
                "spans":[{"name":"pi.ai.request","ended":True,"attributes":attrs,"events":[{"name":"pi.ai.first_chunk"},{"name":"pi.ai.block","attributes":{"type":"text","index":0,"chars":2}}]}] if observed else []}
    return fixture,{"cases":[row(r,e,False) for e in ENDINGS for r in ("simple","direct")]},{"cases":[row(r,e,True) for e in ENDINGS for r in ROUTES]}


class ComparatorTests(unittest.TestCase):
    def test_complete_matrix(self):
        self.assertEqual(compare(*traces())["rust_routes"],12)

    def test_missing_route(self):
        f,p,r=traces();r["cases"].pop()
        with self.assertRaisesRegex(ValueError,"coverage"):compare(f,p,r)

    def test_synchronized_forged_usage(self):
        f,p,r=traces()
        for trace in (p,r):trace["cases"][0]["result"]["usage"]=[11,7,5,3]
        with self.assertRaisesRegex(ValueError,"usage"):compare(f,p,r)

    def test_stale_failure_cost(self):
        f,p,r=traces();r["cases"][4]["result"]["cost"]=52e-6
        with self.assertRaisesRegex(ValueError,"cost"):compare(f,p,r)

    def test_duplicate_metric(self):
        f,p,r=traces();r["cases"][0]["metrics"].append(copy.deepcopy(r["cases"][0]["metrics"][0]))
        with self.assertRaisesRegex(ValueError,"token metric"):compare(f,p,r)

    def test_raw_route_observer_leak(self):
        f,p,r=traces();r["cases"][2]["spans"]=copy.deepcopy(r["cases"][0]["spans"])
        with self.assertRaisesRegex(ValueError,"observer ownership"):compare(f,p,r)

    def test_mutated_owned_partial(self):
        f,p,r=traces();r["cases"][0]["held"]["text"]="AB"
        with self.assertRaisesRegex(ValueError,"owned partial"):compare(f,p,r)

    def test_span_left_open(self):
        f,p,r=traces();r["cases"][0]["spans"][0]["ended"]=False
        with self.assertRaisesRegex(ValueError,"settlement"):compare(f,p,r)

    def test_missing_business_events(self):
        f,p,r=traces();r["cases"][0]["events"]=[]
        with self.assertRaisesRegex(ValueError,"business event order"):compare(f,p,r)

    def test_wrong_block_attributes(self):
        f,p,r=traces();r["cases"][0]["spans"][0]["events"][1]["attributes"]={"type":"tool_call","index":91,"chars":999}
        with self.assertRaisesRegex(ValueError,"block attributes"):compare(f,p,r)

    def test_stale_event_cost(self):
        f,p,r=traces();r["cases"][0]["events"][0]["message"]["cost"]=0
        with self.assertRaisesRegex(ValueError,"event accounting"):compare(f,p,r)


if __name__=="__main__":unittest.main()
