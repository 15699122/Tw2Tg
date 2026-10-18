import hashlib,json,re,struct,subprocess,zipfile
from pathlib import Path
evidence=Path(__file__).resolve().parent
pkg=Path('dist-portable/XArchive-0.2.1-dev-dc731cc-windows-x64-full').resolve()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
manifest=json.loads((pkg/'package-manifest.json').read_text())
assert manifest['package_type']=='full' and manifest['platform']=='windows-x64'
for f in manifest['installation']['files']: assert (pkg/f).is_file(),f
required=['xarchive-desktop.exe','native-host/xarchive-native-host.exe','sidecar/xarchive-downloader/xarchive-downloader.exe','sidecar/xarchive-downloader/_internal/python312.dll','sidecar/gallery-dl/gallery-dl.exe','sidecar/aria2/aria2c.exe','extension/src/browser-pairing.js']
for f in required: assert (pkg/f).stat().st_size>0,f
for f in required[:3]:
 b=(pkg/f).read_bytes(); pe=struct.unpack_from('<I',b,0x3c)[0]
 assert b[pe:pe+4]==b'PE\0\0' and struct.unpack_from('<H',b,pe+4)[0]==0x8664
worker=evidence/'worker-dist/xarchive-downloader'
for p in worker.rglob('*'):
 if p.is_file(): assert sha(p)==sha(pkg/'sidecar/xarchive-downloader'/p.relative_to(worker)),p
pins=json.loads(Path('desktop/scripts/windows-external-assets.json').read_text())
assert sha(pkg/'sidecar/gallery-dl/gallery-dl.exe')==pins['gallery']['sha256']
assert sha(pkg/'sidecar/aria2/aria2c.exe')==sha(Path('validation-artifacts/windows-aa8dabd/aria-verified-extract/aria2-1.37.0-win-64bit-build1/aria2c.exe'))
for p in pkg.rglob('*'):
 if p.is_file(): assert not re.search(r'(?i)(?:^|[/\\])\.env|\.(?:db|sqlite3|log)$',str(p.relative_to(pkg))),p
versions={}
for rel,args in [('sidecar/gallery-dl/gallery-dl.exe',['--version']),('sidecar/aria2/aria2c.exe',['--version']),('sidecar/xarchive-downloader/xarchive-downloader.exe',['--help'])]:
 r=subprocess.run([str(pkg/rel),*args],capture_output=True,text=True,encoding='utf-8',timeout=20)
 assert r.returncode==0,(rel,r.stderr)
 versions[rel]=r.stdout
identity={'input_sha':'9e8fc3055d052b5281546eff6a813a6da37c79d4','desktop_source_sha':'dc731cce8a1b696af1dea00c3ad491b3e2ecc045','native_host_and_worker_build_sha':'775af5fa59c926db4740d5cfdd0d508e753f75df','component_provenance':'Desktop canonical fresh release build; Native Host cargo release and worker PyInstaller built this batch at 775af5f; their sources unchanged through dc731cc. Gallery/aria2 match reviewed external pins. No binary reuse opt-in.','type':'full portable development candidate','release_acceptance':False}
(pkg/'build-identity.json').write_text(json.dumps(identity,indent=2)+'\n')
files={p.relative_to(pkg).as_posix():sha(p) for p in sorted(pkg.rglob('*')) if p.is_file()}
(evidence/'package-files-sha256.json').write_text(json.dumps(files,indent=2)+'\n')
archive=pkg.parent/(pkg.name+'-validated.zip')
assert not archive.exists()
with zipfile.ZipFile(archive,'w',zipfile.ZIP_DEFLATED,compresslevel=6) as z:
 for p in sorted(pkg.rglob('*')):
  z.write(p,pkg.name+'/'+p.relative_to(pkg).as_posix())
with zipfile.ZipFile(archive) as z:
 assert z.testzip() is None
 for rel,digest in files.items(): assert hashlib.sha256(z.read(pkg.name+'/'+rel)).hexdigest()==digest
result={'status':'PASS','files':len(files),'zip':str(archive),'zip_bytes':archive.stat().st_size,'zip_sha256':sha(archive),'desktop_sha256':files['xarchive-desktop.exe'],'native_host_sha256':files[required[1]],'worker_sha256':files[required[2]],'versions':versions,'identity':identity}
(evidence/'package-verification.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items() if k not in ['versions','identity']},indent=2))
