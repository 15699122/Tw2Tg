from pathlib import Path
import os,json,sqlite3,hashlib,sys
root=Path(os.environ['TEMP'])/'tw2tg-followup-20261018/matrix-canonical'
db=root/'profile/config/archive.sqlite3'
with sqlite3.connect(db.as_uri()+'?mode=ro',uri=True) as connection:
 result={t:connection.execute('SELECT * FROM '+t+' ORDER BY rowid').fetchall() for t in ['jobs','tweets','record_exchange_ledger']}
result['media']={str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((root/'归档 文件').rglob('*.bin'))}
payload=json.dumps(result,sort_keys=True,ensure_ascii=False)
out={'jobs':len(result['jobs']),'tweets':len(result['tweets']),'ledger':len(result['record_exchange_ledger']),'media':len(result['media']),'canonical_facts_sha256':hashlib.sha256(payload.encode()).hexdigest()}
Path(__file__).with_name('matrix-'+sys.argv[1]+'.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(out))
