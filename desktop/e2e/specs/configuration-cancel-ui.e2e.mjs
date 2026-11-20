import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { access,readFile } from "node:fs/promises";
import path from "node:path";
import { waitForApplicationDocument,waitForStartupContract } from "../support/native-startup.mjs";
const candidate=JSON.stringify({schema_version:1,kind:"tw2tg.configuration",settings:{log_max_files:11}});
const read=async p=>readFile(p).catch(e=>{if(e.code==="ENOENT")return null;throw e});
const exported=()=>browser.executeAsync(done=>window.__TAURI__.core.invoke("export_configuration_exchange").then(v=>done({value:v}),e=>done({error:String(e)})));
async function stop(p){if(p && p.exitCode===null && p.signalCode===null){const exited=new Promise(r=>p.once("exit",r));p.kill();await exited;}}
describe("Windows configuration GUI cancellation",()=>{
 it("cancels waiting apply without writing and reopens admission",async function(){
  if(process.platform!=="win32" || process.env.WDIO_F2_GUI!=="1" || !process.env.WDIO_WORKSPACE_HOLDER || !process.env.XARCHIVE_PORTABLE_ROOT) this.skip();
  await waitForApplicationDocument(); await waitForStartupContract();
  const root=process.env.XARCHIVE_PORTABLE_ROOT, locks=path.join(root,"config","locks"), config=path.join(root,"config","config.yaml");
  await (await browser.$("button=下载配置")).click();
  await (await browser.$("#configuration-exchange button.settings-panel-toggle")).click();
  await (await browser.$("#configuration-exchange-document")).setValue(candidate);
  await (await (await browser.$("#configuration-exchange")).$("button=预览导入")).click();
  await browser.waitUntil(async()=>(await (await browser.$("#configuration-exchange")).getText()).includes("预览修订："),{timeout:10000});
  const before=await read(config), runtimeBefore=await exported(); assert.equal(runtimeBefore.error,undefined);
  const holder=spawn(process.env.WDIO_WORKSPACE_HOLDER,["--exact","active_workspace_holder","--nocapture"],{env:{...process.env,TW2TG_ACTIVE_WORKSPACE_HOLDER:locks},stdio:"ignore",windowsHide:true});
  try{
   await browser.waitUntil(async()=>{if(holder.exitCode!==null)throw Error(`holder exited ${holder.exitCode}`); return access(path.join(locks,"active-holder.ready")).then(()=>true,()=>false)}, {timeout:10000});
   await (await (await browser.$("#configuration-exchange")).$("button=应用导入")).click();
   const cancel=await (await browser.$("#configuration-exchange")).$("button=取消等待"); await cancel.waitForDisplayed({timeout:10000}); await cancel.click();
   await browser.waitUntil(async()=>(await (await browser.$("#configuration-exchange")).getText()).includes("已取消等待；本机配置未改动。"),{timeout:10000});
   const text=await (await browser.$("#configuration-exchange")).getText(); assert.ok(!text.includes("锁失败"));
   assert.deepEqual(await read(config),before); assert.deepEqual(await exported(),runtimeBefore); console.log("GUI_CANCEL_NO_CHANGE");
   const second=spawn(process.env.WDIO_WORKSPACE_HOLDER,["--exact","active_workspace_holder","--nocapture"],{env:{...process.env,TW2TG_ACTIVE_WORKSPACE_HOLDER:locks,TW2TG_ACTIVE_HOLDER_SECOND:"1"},stdio:"ignore",windowsHide:true});
   try{await browser.waitUntil(async()=>{if(second.exitCode!==null)throw Error(`second holder exited ${second.exitCode}`);return access(path.join(locks,"active-holder-second.ready")).then(()=>true,()=>false)}, {timeout:10000}); console.log("ADMISSION_REOPENED");}finally{await stop(second)}
  }finally{await stop(holder)}
  await browser.waitUntil(async()=>await (await (await browser.$("#configuration-exchange")).$("button=应用导入")).isEnabled(),{timeout:10000});
  await (await (await browser.$("#configuration-exchange")).$("button=应用导入")).click();
  await browser.waitUntil(async()=>(await (await browser.$("#configuration-exchange")).getText()).includes("配置已写入"),{timeout:10000});
  assert.match((await read(config)).toString("utf8"),/max_files:\s*11/); console.log("LATER_GUI_APPLY");
 });
});
