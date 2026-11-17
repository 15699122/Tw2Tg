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


    def test_openssl_notice_checked_only_when_runtime_is_bundled(self):
        license_path = script.parents[1] / "third-party" / "openssl" / "LICENSE"
        with TemporaryDirectory() as temp:
            worker = Path(temp)
            internal = worker / "_internal"
            internal.mkdir()
            self.assertFalse(module.include_openssl_license(worker, license_path))
            (internal / "libcrypto-3-x64.dll").write_bytes(b"fixture")
            output = worker / "LICENSE.OpenSSL.txt"
            output.write_bytes(b"previous")
            wrong = worker / "wrong-license"
            wrong.write_bytes(b"incorrect")
            with self.assertRaisesRegex(ValueError, "Missing or altered"):
                module.include_openssl_license(worker, wrong)
            self.assertEqual(output.read_bytes(), b"previous")
            self.assertTrue(module.include_openssl_license(worker, license_path))
            self.assertEqual(output.read_bytes(), license_path.read_bytes())


if __name__ == "__main__":
    unittest.main()
