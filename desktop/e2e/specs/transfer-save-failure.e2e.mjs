import assert from 'node:assert/strict';
import { mkdir, readFile, rmdir } from 'node:fs/promises';
import path from 'node:path';
import { waitForApplicationDocument, waitForStartupContract } from '../support/native-startup.mjs';

describe('transfer configuration write failure', () => {
  it('observes runtime and persisted configuration after an isolated write failure', async function () {
    if (process.platform !== 'win32' || process.env.WDIO_TRANSFER_FAILURE_GUI !== '1' || !process.env.XARCHIVE_PORTABLE_ROOT) this.skip();
    await waitForApplicationDocument(); await waitForStartupContract();
    const root=process.env.XARCHIVE_PORTABLE_ROOT;
    assert.equal(await readFile(path.join(root,'w3-transfer-fixture.marker'),'utf8'),'isolated-transfer-failure');
    const call=(command,args={})=>browser.executeAsync((command,args,done)=>window.__TAURI__.core.invoke(command,args).then(value=>done({value}),error=>done({error:String(error)})),command,args);
    const before=await call('get_app_status'); assert.equal(before.error,undefined); assert.equal(before.value.use_aria2,false);
    const config=path.join(root,'config','config.yaml');
    const readConfig=()=>readFile(config).catch(e=>{if(e.code==='ENOENT')return null;throw e;});
    const bytes=await readConfig();
    const blocker=path.join(root,'config','config.yaml.tmp');
    await mkdir(path.dirname(blocker),{recursive:true}); await mkdir(blocker);
    try {
      const attempt=await call('set_use_aria2',{useAria2:true});
      assert.ok(attempt.error,'write failure must be returned');
      const after=await call('get_app_status'); assert.equal(after.error,undefined);
      assert.deepEqual(await readConfig(),bytes);
      console.log('TRANSFER_WRITE_FAILURE',JSON.stringify({before:before.value.use_aria2,after:after.value.use_aria2,errorReturned:true,persistedUnchanged:true}));
      assert.equal(after.value.use_aria2,false,'failed save must not report the unsaved setting');
    } finally { await rmdir(blocker); }
  });
});
