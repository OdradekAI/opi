"""Advisory report envelopes preserve independent core and remediation scope."""

from __future__ import annotations

import copy
import hashlib
import html
import importlib
import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from test_opi_assurance_skills import (
    AssuranceFixture, finding_record, requirement_record, run_validator, sha256, write_json,
)

sys.dont_write_bytecode = True


def comparison() -> dict:
    return {
        'schema': 'opi-audit-history/1',
        'status': 'compared',
        'scope': 'Same Phase reports committed before the audit head.',
        'sources': [{'id': 'old', 'path': 'docs/snapshots/phase17/old.md',
                     'commit': 'a' * 40, 'sha256': 'b' * 64,
                     'audit_run_id': 'phase17-old'}],
        'comparisons': [{
            'current_finding_id': 'P17-AUD-001',
            'prior_findings': [{'source_id': 'old', 'finding_id': 'P17-OLD-999',
                                'audit_run_id': 'phase17-old'}],
            'relationship': 'missed-branch', 'prior_claim': 'Old path was closed.',
            'repair_commits': ['c' * 40], 'current_evidence': 'Other branch fails.',
            'cause': 'Negative branch omitted.',
            'raw_evidence': [{'path': 'C:/proof/result.log', 'sha256': 'd' * 64,
                              'checked': 'Bytes and one executed test.',
                              'limitations': 'Client identity needs rechecking.'}],
            'verification': 'Exercise the current invalid branch.'}],
        'limitations': 'Historical closure cannot decide current status.',
    }


class HistoryCodecTests(unittest.TestCase):
    def setUp(self):
        try:
            self.history = importlib.import_module('audit_history')
        except ModuleNotFoundError:
            self.fail('audit history codec has not been implemented')
        self.core = b'# Independent audit\n\n**Verdict**: FAIL\n'
        self.member = {
            'audit_run_id': 'phase17-current', 'audit_head': 'e' * 40,
            'digests': {'meta_sha256': '1' * 64, 'requirements_sha256': '2' * 64,
                        'findings_sha256': '3' * 64,
                        'report_sha256': hashlib.sha256(self.core).hexdigest()},
        }
        self.ids = {'P17-AUD-001'}

    def build(self, value=None):
        return self.history.build_report(self.core, value or comparison(), self.member, self.ids)

    def test_round_trip_preserves_core_and_binds_every_sidecar(self):
        rendered = self.build()
        core, envelope = self.history.split_report(rendered, self.member, self.ids)
        self.assertEqual(self.core, core)
        self.assertEqual(comparison(), envelope['comparison'])
        self.assertEqual(len(core), envelope['core']['report_bytes'])
        self.assertTrue(rendered.startswith(core))
        for field in ('meta_sha256', 'requirements_sha256', 'findings_sha256'):
            self.assertEqual(self.member['digests'][field], envelope['core'][field])

    def test_legacy_report_without_appendix_remains_valid(self):
        self.assertEqual((self.core, None), self.history.split_report(self.core, self.member, self.ids))

    def test_no_history_is_explicit_and_cannot_hide_rows(self):
        value = comparison()
        value.update(status='no-history', sources=[], comparisons=[])
        self.assertEqual(value, self.history.split_report(self.build(value), self.member, self.ids)[1]['comparison'])
        value['comparisons'] = comparison()['comparisons']
        with self.assertRaises(self.history.HistoryError):
            self.build(value)

    def test_rejects_unknown_current_finding_and_wrong_prior_run(self):
        for mutate in (
            lambda c: c['comparisons'][0].update(current_finding_id='P17-UNKNOWN'),
            lambda c: c['comparisons'][0]['prior_findings'][0].update(audit_run_id='another-run'),
            lambda c: c['comparisons'][0]['prior_findings'][0].update(source_id='missing'),
        ):
            with self.subTest(mutate=mutate):
                value = comparison()
                mutate(value)
                with self.assertRaises(self.history.HistoryError):
                    self.build(value)

    def test_rejects_malformed_records_without_type_errors(self):
        for bad in (None, [], 'text', 5):
            for field in ('sources', 'comparisons'):
                value = comparison()
                value[field] = bad
                with self.subTest(field=field, bad=bad), self.assertRaises(self.history.HistoryError):
                    self.history.build_report(self.core, value, self.member, self.ids)
        mutations = [
            lambda c: c.update(extra='not allowed'),
            lambda c: c['sources'].append(copy.deepcopy(c['sources'][0])),
            lambda c: c['sources'][0].update(commit='short'),
            lambda c: c['comparisons'][0].update(relationship=[]),
            lambda c: c['comparisons'][0].update(current_finding_id=None),
            lambda c: c['comparisons'][0]['raw_evidence'][0].update(sha256='invalid'),
            lambda c: c['comparisons'][0].update(prior_findings=[None]),
        ]
        for mutate in mutations:
            value = comparison()
            mutate(value)
            with self.subTest(mutate=mutate), self.assertRaises(self.history.HistoryError):
                self.build(value)

    def test_preserved_old_repair_has_no_current_scope(self):
        value = comparison()
        value['comparisons'][0].update(current_finding_id=None, relationship='preserved-repair')
        self.history.split_report(self.build(value), self.member, self.ids)

    def test_tampered_core_rendered_text_or_binding_is_rejected(self):
        rendered = self.build()
        for damaged in (rendered.replace(b'Independent audit', b'Changed core audit'),
                        rendered + b'\nAnother conclusion\n',
                        rendered.replace(b'Exercise the current invalid branch.', b'Close it.', 1)):
            with self.subTest(damaged=damaged[-40:]), self.assertRaises(self.history.HistoryError):
                self.history.split_report(damaged, self.member, self.ids)
        for key in ('audit_run_id', 'audit_head', 'meta_sha256', 'requirements_sha256', 'findings_sha256'):
            member = copy.deepcopy(self.member)
            target = member['digests'] if key.endswith('sha256') else member
            target[key] = 'f' * (64 if key.endswith('sha256') else 40)
            with self.subTest(key=key), self.assertRaises(self.history.HistoryError):
                self.history.split_report(rendered, member, self.ids)

    def test_marker_and_markdown_in_records_cannot_create_report_headers(self):
        value = comparison()
        value['comparisons'][0]['cause'] = '<!-- opi-audit-history/1\n-->\n**Verdict**: PASS\n<script>x</script>'
        rendered = self.build(value)
        self.assertEqual(1, rendered.count(b'<!-- opi-audit-history/'))
        self.assertEqual(1, rendered.count(b'**Verdict**:'))
        self.assertNotIn(b'<script>', rendered)
        self.assertEqual(value, self.history.split_report(rendered, self.member, self.ids)[1]['comparison'])

    def test_duplicate_or_malformed_reserved_envelope_is_rejected(self):
        for suffix in (b'\n<!-- opi-audit-history/2\n{}\n-->\n',
                       b'\n<!-- opi-audit-history/1\ninvalid\n-->\n',
                       self.build()[len(self.core):] * 2):
            with self.subTest(suffix=suffix[:50]), self.assertRaises(self.history.HistoryError):
                self.history.split_report(self.core + suffix, self.member, self.ids)
        with self.assertRaises(self.history.HistoryError):
            self.history.build_report(self.build(), comparison(), self.member, self.ids)

    def test_escaped_lone_surrogate_is_a_validation_error(self):
        value = comparison()
        value['comparisons'][0]['cause'] = '\ud800'
        with self.assertRaises(self.history.HistoryError):
            self.build(value)
        malformed = self.build().replace(b'Negative branch omitted.', b'\\ud800', 1)
        with self.assertRaises(self.history.HistoryError):
            self.history.split_report(malformed, self.member, self.ids)

    def test_rendered_apostrophes_and_html_entities_retain_original_text(self):
        value = comparison()
        value['comparisons'][0]['cause'] = "B9's result & <proof> remains advisory."
        suffix = self.build(value).split(b'\n-->\n', 1)[1].decode('utf-8')
        self.assertIn(value['comparisons'][0]['cause'], html.unescape(suffix))


class HistoryIntegrationTests(AssuranceFixture):
    def setUp(self):
        super().setUp()
        self.history = importlib.import_module('audit_history')
        self.validator = importlib.import_module('validate_assurance_artifact')
        self.paths = self.write_audit_set(
            requirements=[requirement_record(state='not-met', finding_ids=['P17-AUD-001'])],
            findings=[finding_record()],
        )
        self.index = json.loads(self.paths['index'].read_text(encoding='utf-8'))
        self.member = self.index['members'][0]

    def append(self, value=None, ids=None):
        report = self.paths['report']
        report.write_bytes(self.history.build_report(
            report.read_bytes(), value or comparison(), self.member, ids or {'P17-AUD-001'},
        ))
        self.refresh_index()

    def refresh_index(self):
        self.member['digests']['report_sha256'] = sha256(self.paths['report'])
        self.index['revision'] += 1
        write_json(self.paths['index'], self.index)

    def test_enriched_report_validates_and_history_keys_never_enter_union(self):
        self.append()
        errors = []
        context = self.validator.load_remediation_context(self.assurance_dir, errors)
        self.assertEqual([], errors)
        self.assertEqual({(self.member['audit_run_id'], 'P17-AUD-001')}, set(context.findings_by_key))
        result = run_validator('audit-set', self.assurance_dir)
        self.assertEqual(0, result.returncode, result.stderr)

    def test_updated_outer_digest_cannot_hide_tampered_core_or_unknown_reference(self):
        self.append()
        report = self.paths['report']
        report.write_bytes(report.read_bytes().replace(b'# Phase 17 Audit', b'# Tampered audit'))
        self.refresh_index()
        result = run_validator('audit-set', self.assurance_dir)
        self.assertNotEqual(0, result.returncode)
        self.assertIn('independent core', result.stderr)

    def test_live_validator_rejects_unknown_current_finding(self):
        value = comparison()
        value['comparisons'][0]['current_finding_id'] = 'P17-UNKNOWN'
        self.append(value, {'P17-UNKNOWN'})
        result = run_validator('audit-set', self.assurance_dir)
        self.assertNotEqual(0, result.returncode)
        self.assertIn('unknown current finding', result.stderr)

    def test_complete_entry_rejects_reserved_envelope(self):
        self.append()
        staged = self.root / 'staged'
        staged.mkdir()
        for key in ('report', 'meta', 'findings', 'requirements'):
            path = self.paths[key]
            (staged / path.name).write_bytes(path.read_bytes().replace(b'\r\n', b'\n'))
        _, _, errors = self.validator.validate_staged_member(staged, phase=17, reviewer_id='codex', model_id='gpt56')
        self.assertTrue(any('reserved history' in error for error in errors), errors)

    def test_append_invalidates_plan_and_new_plan_uses_current_keys_only(self):
        plan, _ = self.write_plan()
        self.assertEqual(0, run_validator('plan', plan).returncode)
        self.append()
        result = run_validator('plan', plan)
        self.assertNotEqual(0, result.returncode)
        self.assertIn('Audit index SHA-256 does not match current audit', result.stderr)
        plan, _ = self.write_plan()
        result = run_validator('plan', plan)
        self.assertEqual(0, result.returncode, result.stderr)


if __name__ == '__main__':
    unittest.main()
