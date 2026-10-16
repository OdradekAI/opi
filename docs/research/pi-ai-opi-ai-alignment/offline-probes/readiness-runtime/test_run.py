"""Negative CLI checks: reject changed inputs before launching research children."""
import subprocess, sys, tempfile, unittest
from pathlib import Path

ASSETS=Path(__file__).resolve().parent
REPO=ASSETS.parents[4]
NODE='D:/nvm/v24.13.0/node.exe'

class InputChecks(unittest.TestCase):
    def invoke(self, *changes):
        with tempfile.TemporaryDirectory(prefix='opi-readiness-negative-') as scratch:
            values={'--pi':str(REPO/'.repo/pi-1.3.0'),'--node':NODE,'--scratch':scratch,'--target':'E:/opi/cargo-targets/windows-direct'}
            values.update(dict(changes))
            command=[sys.executable,str(ASSETS/'run.py')]
            for key,value in values.items(): command += [key,value]
            result=subprocess.run(command,capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0)
            self.assertFalse((Path(scratch)/'commands.json').exists())
            return result.stderr
    def test_wrong_node(self):
        self.assertIn('Node pin mismatch',self.invoke(('--node',str(ASSETS/'pins.json'))))
    def test_missing_shared_target(self):
        self.assertIn('existing shared Cargo target',self.invoke(('--target',str(ASSETS/'not-a-target'))))
    def test_source_tree_cannot_be_scratch(self):
        self.assertIn('Scratch must be isolated',self.invoke(('--scratch',str(REPO/'.repo/pi-1.3.0'))))
    def test_workspace_cannot_be_scratch(self):
        self.assertIn('Scratch must be isolated',self.invoke(('--scratch',str(REPO))))

if __name__=='__main__':unittest.main()
