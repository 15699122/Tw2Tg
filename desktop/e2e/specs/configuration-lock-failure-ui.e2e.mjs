import assert from "node:assert/strict";
import { lstat,mkdir,readFile,rmdir,unlink } from "node:fs/promises";
import path from "node:path";
import { waitForApplicationDocument,waitForStartupContract } from "../support/native-startup.mjs";
const candidate=JSON.stringify({schema_version:1,kind:"tw2tg.configuration",settings:{log_max_files:13}});
const read=async p=>readFile(p).catch(e=>{if(e.code==="ENOENT")return null;throw e});
const exported=()=>browser.executeAsync(done=>window.__TAURI__.core.invoke("export_configuration_exchange").then(v=>done({value:v}),e=>done({error:String(e)})));
describe("Windows configuration GUI lock failure",()=>{
 it("reports a lock I/O failure without writing and permits a later apply",async function(){
  if(process.platform!=="win32" || process.env.WDIO_F2_GUI!=="1" || !process.env.XARCHIVE_PORTABLE_ROOT) this.skip();
  await waitForApplicationDocument(); await waitForStartupContract();
  const root=process.env.XARCHIVE_PORTABLE_ROOT, lock=path.join(root,"config","locks","workspace-admission.lock"), config=path.join(root,"config","config.yaml");
  assert.equal(await readFile(path.join(root,"w3-fixture.marker"),"utf8"),"isolated-w3-gui");
  await (await browser.$("button=下载配置")).click();
  await (await browser.$("#configuration-exchange button.settings-panel-toggle")).click();
  await (await browser.$("#configuration-exchange-document")).setValue(candidate);
  const section=await browser.$("#configuration-exchange");
  await (await section.$("button=预览导入")).click();
  await browser.waitUntil(async()=>(await section.getText()).includes("预览修订："),{timeout:10000});
  const before=await read(config), runtimeBefore=await exported(); assert.equal(runtimeBefore.error,undefined);
  const existing=await lstat(lock).catch(e=>{if(e.code==="ENOENT")return null;throw e});
  if(existing){assert.ok(existing.isFile() && existing.size===0,"fixture lock is not an empty regular file");await unlink(lock)}
  await mkdir(lock);
  try{
   await (await section.$("button=应用导入")).click();
   await browser.waitUntil(async()=>(await section.getText()).includes("应用失败："),{timeout:10000});
   const text=await section.getText(); assert.ok(text.includes("workspace maintenance unavailable")); assert.ok(!text.includes("已取消等待") && !text.includes("等待超时"));
   assert.deepEqual(await read(config),before); assert.deepEqual(await exported(),runtimeBefore); console.log("GUI_LOCK_FAILURE_NO_CHANGE");
  }finally{await rmdir(lock)}
  await browser.waitUntil(async()=>await (await section.$("button=应用导入")).isEnabled(),{timeout:10000});
  await (await section.$("button=应用导入")).click();
  await browser.waitUntil(async()=>(await section.getText()).includes("配置已写入"),{timeout:10000});
  assert.match((await read(config)).toString("utf8"),/max_files:\s*13/); console.log("LATER_GUI_APPLY");
 });
});
