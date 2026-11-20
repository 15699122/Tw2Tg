import assert from "node:assert/strict";
import {readFile} from "node:fs/promises";
import path from "node:path";
import {waitForApplicationDocument,waitForStartupContract} from "../support/native-startup.mjs";
describe("configuration preview document binding",()=>{
 it("requires another preview after editing the document",async function(){
  if(process.platform!=="win32" || process.env.WDIO_F2_GUI!=="1" || !process.env.XARCHIVE_PORTABLE_ROOT) this.skip();
  await waitForApplicationDocument();await waitForStartupContract();
  const doc=n=>JSON.stringify({schema_version:1,kind:"tw2tg.configuration",settings:{log_max_files:n}});
  await (await browser.$("button=下载配置")).click(); await (await browser.$("#configuration-exchange button.settings-panel-toggle")).click();
  const section=await browser.$("#configuration-exchange"), input=await browser.$("#configuration-exchange-document");
  await input.setValue(doc(11)); await (await section.$("button=预览导入")).click();
  await browser.waitUntil(async()=>(await section.getText()).includes("预览修订："),{timeout:10000});
  await input.click(); await browser.keys(["Control","a"]); await browser.keys("Backspace"); await input.addValue(doc(12)); assert.equal(await input.getValue(),doc(12));
  const apply=await section.$("button=应用导入");
  assert.equal(await apply.isEnabled(),false,"edited document must require a new preview");
  assert.equal((await section.getText()).includes("预览修订："),false);
  await (await section.$("button=预览导入")).click();
  await browser.waitUntil(async()=>(await section.getText()).includes("预览修订："),{timeout:10000});
  assert.equal(await apply.isEnabled(),true); await apply.click();
  await browser.waitUntil(async()=>(await section.getText()).includes("配置已写入"),{timeout:10000});
  const config=await readFile(path.join(process.env.XARCHIVE_PORTABLE_ROOT,"config","config.yaml"),"utf8"); assert.match(config,/max_files:\s*12/); console.log("REPREVIEWED_DOCUMENT_APPLIED");
 });
});
