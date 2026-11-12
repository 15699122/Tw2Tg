import assert from "node:assert/strict";
import { test } from "node:test";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { resolve } from "node:path";
import { launcher, isPidAlive, killTree, ownedDescendants, sameProcess } from "../scripts/wdio-tauri-service.mjs";

test("ownership excludes foreign ports, pre-parent creations and reused identities", () => {
  const root={pid:10,parent:1,created:"100",exe:"C:/driver.exe"};
  const child={pid:11,parent:10,created:"101",exe:"C:/native.exe"};
  const foreign={pid:12,parent:1,created:"110",exe:"C:/unrelated.exe"};
  const stale={pid:13,parent:10,created:"90",exe:"C:/old.exe"};
  assert.deepEqual([...ownedDescendants([root,child,foreign,stale],[10]).keys()],[10,11]);
  assert.equal(sameProcess(child,{...child,created:"102"}),false);
  assert.equal(sameProcess(child,{...child,exe:"C:/unrelated.exe"}),false);
});

test("ownership snapshot failure still stops direct upstream children without blind cleanup", { skip:process.platform!=="win32" }, async()=>{
  const upstream=Object.getPrototypeOf(launcher.prototype);
  const original=upstream.onComplete;
  let stopped=false;let blindCleanup=false;
  upstream.onComplete=async()=>{stopped=true};
  const instance=Object.assign(Object.create(launcher.prototype),{
    driverPorts:()=>[],snapshotOwnedDrivers:async()=>{throw Error("snapshot unavailable")},
    reapSurvivorDrivers:async()=>{blindCleanup=true},
  });
  try{await assert.rejects(instance.onComplete(1,{},[]),/snapshot unavailable/);assert.equal(stopped,true);assert.equal(blindCleanup,false)}
  finally{upstream.onComplete=original}
});

test("real driver cleanup preserves a foreign listener and terminates its own tree", {
  skip: process.platform!=="win32" || process.env.WDIO_NATIVE_CLEANUP!=="1",
  timeout:60000,
}, async () => {
  const foreign=spawn(process.execPath,["-e", "require('net').createServer(s=>s.end('alive')).listen(0,'127.0.0.1',function(){console.log(this.address().port)})"],{windowsHide:true,stdio:["ignore","pipe","pipe"]});
  const port=Number((await once(foreign.stdout,"data"))[0].toString().trim());
  const driver=spawn(process.env.TAURI_DRIVER_PATH,["--port","4449","--native-port","4450","--native-driver",resolve(process.env.EDGEDRIVER_PATH)],{windowsHide:true,stdio:"ignore"});
  await once(driver,"spawn");
  const instance=Object.assign(Object.create(launcher.prototype),{options:{tauriDriverPort:45884},driverPool:{
    getStatus:()=>({identifiers:["owned"]}),getDriver:()=>({process:{proc:driver},port:4449,nativePort:4450})}});
  try {
    let ready=false;
    for(let i=0;i<50;i++){
      try{if((await fetch("http://127.0.0.1:4449/status",{signal:AbortSignal.timeout(200)})).ok){ready=true;break}}catch{}
      await new Promise(r=>setTimeout(r,100));
    }
    assert.ok(ready,"owned driver must become ready");
    const owners=await instance.collectPortOwnerPids([4449,4450,port]);
    assert.ok(owners.includes(foreign.pid),"foreign fixture really occupies a tracked port");
    const owned=await instance.snapshotOwnedDrivers();
    assert.ok(owned.has(driver.pid)); assert.ok(owned.size>=2,"native child captured");
    assert.equal(owned.has(foreign.pid),false);
    await instance.reapSurvivorDrivers(owned,[4449,4450,port]);
    assert.equal(isPidAlive(foreign.pid),true,"unrelated listener must survive");
    assert.deepEqual(await instance.collectPortOwnerPids([4449,4450]),[]);
    assert.deepEqual(await instance.collectPortOwnerPids([port]),[foreign.pid]);
    console.log(JSON.stringify({status:"PASS",foreignPid:foreign.pid,foreignPort:port,owned:[...owned.keys()],foreignAlive:true}));
  } finally {
    if(isPidAlive(driver.pid))await killTree(driver.pid);
    foreign.kill();
  }
});
