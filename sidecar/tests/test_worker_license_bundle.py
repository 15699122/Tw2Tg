from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest


script = Path(__file__).resolve().parents[1] / "pyinstaller" / "include_python_license.py"
spec = spec_from_file_location("include_python_license", script)
module = module_from_spec(spec)
spec.loader.exec_module(module)


class PythonLicenseBundleTest(unittest.TestCase):
    def test_matches_runtime_before_copy_and_preserves_previous_output_on_failure(self):
        with TemporaryDirectory() as temp:
            root = Path(temp)
            runtime = root / "runtime"
            worker = root / "worker"
            runtime.mkdir()
            (worker / "_internal").mkdir(parents=True)
            (runtime / "python312.dll").write_bytes(b"runtime")
            (runtime / "LICENSE.txt").write_bytes(b"upstream licence\n")
            bundled = worker / "_internal" / "python312.dll"
            bundled.write_bytes(b"different")
            output = worker / "LICENSE.Python.txt"
            output.write_bytes(b"previous")
            with self.assertRaisesRegex(ValueError, "does not match"):
                module.include_python_license(runtime, worker, "python312.dll")
            self.assertEqual(output.read_bytes(), b"previous")
            bundled.write_bytes(b"runtime")
            (runtime / "LICENSE.txt").write_bytes(b"")
            with self.assertRaisesRegex(ValueError, "Missing or empty"):
                module.include_python_license(runtime, worker, "python312.dll")
            self.assertEqual(output.read_bytes(), b"previous")
            (runtime / "LICENSE.txt").write_bytes(b"upstream licence\n")
            result = module.include_python_license(runtime, worker, "python312.dll")
            self.assertEqual(output.read_bytes(), (runtime / "LICENSE.txt").read_bytes())
            self.assertEqual(result, module.digest(output))


if __name__ == "__main__":
    unittest.main()
