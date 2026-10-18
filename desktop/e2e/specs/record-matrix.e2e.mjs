import assert from "node:assert/strict";
import { readFile, readdir, writeFile, rename, mkdir, stat } from "node:fs/promises";
import { join } from "node:path";
import YAML from "yaml";
import { waitForApplicationDocument, waitForStartupContract } from "../support/native-startup.mjs";

const root=process.env.TW2TG_GUI_MATRIX_ROOT ?? "";
const downloads=process.env.TW2TG_MATRIX_DOWNLOADS ?? "";
const archive=join(root,"归档 文件");
const exports=[];
async function press(text){await browser.$(`button=${text}`).click()}
async function setInput(selector,value){const input=await browser.$(selector);await input.click();await browser.keys(["Control","a"]);await browser.keys("Backspace");if(value)await input.addValue(value)}
async function preview(){await press("预览导入");await browser.waitUntil(async()=> (await browser.$("main").getText()).includes("预览完成") && await browser.$("table").isExisting() && await browser.$("button=预览导入").isEnabled(),{timeout:10000})}
async function saveScreenshot(name){await browser.saveScreenshot(join(process.env.WDIO_LOG_DIR,name+".png"))}

(process.platform==="win32" && root && downloads ? describe : describe.skip)("Windows F3 actual native GUI matrix",()=>{
 before(async()=>{await waitForApplicationDocument();await waitForStartupContract();await press("记录导入")});
 afterEach(async function(){if(this.currentTest.state==="failed"){await saveScreenshot("failed-"+this.currentTest.title.slice(0,24).replaceAll(/[^a-zA-Z0-9]/g,"_"));await writeFile(join(process.env.WDIO_LOG_DIR,"failure-body.txt"),await browser.$("main").getText())}});
 it("exports Job/Tweet/Batch × JSON/YAML × download/archive through native save buttons",async()=>{
  await setInput("#record-import-root",archive);
  for(const kind of ["download","archive"]){
   await browser.$("#record-export-kind").selectByAttribute("value",kind);
   for(const [scope,value,count] of [["job","matrix-job",1],["tweet","995000",1],["batch","matrix-batch",41]]){
    for(const name of ["job","tweet","batch"])await setInput("#record-export-"+name,name===scope?value:"");
    for(const format of ["json","yaml"]){
     const before=new Set(await readdir(downloads));
     await press("导出 "+format.toUpperCase());
     let file;
     await browser.waitUntil(async()=>{file=(await readdir(downloads)).find(f=>!before.has(f)&&f.startsWith(kind+"-records")&&f.endsWith("."+format));return Boolean(file)},{timeout:10000,timeoutMsg:`native save ${scope}/${kind}/${format}`});
     const raw=await readFile(join(downloads,file),"utf8");
     const doc=format==="json"?JSON.parse(raw):YAML.parse(raw);
     assert.equal(doc.records.length,count,"batch export must exceed the 20-row UI page");
     assert.equal(new Set(doc.records.map(r=>r.metadata.tweet_id)).size,count);
     if(kind==="archive")assert.ok(doc.records.every(r=>r.final_path.startsWith("Tweets/")));
     const copy=join(process.env.WDIO_LOG_DIR,`${scope}-${kind}.${format}`);await writeFile(copy,raw);
     exports.push({scope,kind,format,count,saved:file,bytes:(await stat(join(downloads,file))).size});
    }
   }
  }
  for(const scope of ["job","tweet","batch"])for(const kind of ["download","archive"]){
   const json=JSON.parse(await readFile(join(process.env.WDIO_LOG_DIR,`${scope}-${kind}.json`),"utf8"));
   assert.deepEqual(json,YAML.parse(await readFile(join(process.env.WDIO_LOG_DIR,`${scope}-${kind}.yaml`),"utf8")));
  }
  await writeFile(join(process.env.WDIO_LOG_DIR,"exports.json"),JSON.stringify(exports,null,2));
  await saveScreenshot("export-matrix");
 });
 it("disables ambiguous scopes and invalidates preview after either input changes",async()=>{
  await setInput("#record-export-tweet","995000");
  assert.equal(await browser.$("button=导出 JSON").isEnabled(),false);
  await setInput("#record-export-batch","");
  await setInput("#record-import-records",await readFile(join(root,"archive-document.json"),"utf8"));
  await preview();assert.equal(await browser.$("button=接受导入").isEnabled(),true);
  await browser.$("#record-import-records").addValue(" ");assert.equal(await browser.$("button=接受导入").isEnabled(),false);
  await preview();await browser.$("#record-import-root").addValue(" ");
  assert.equal(await browser.$("button=接受导入").isEnabled(),false);
  await setInput("#record-import-root",archive);
 });
 it("rejects changed canonical metadata and stale root/media without creating Jobs",async()=>{
  const raw=await readFile(join(root,"archive-document.json"),"utf8");
  const conflict=JSON.parse(raw);conflict.records[0].metadata.text="conflicting payload";
  await setInput("#record-import-records",JSON.stringify(conflict));await preview();
  assert.equal(await browser.$("button=接受导入").isEnabled(),false);await saveScreenshot("conflict-preserved");
  await setInput("#record-import-records",raw);await preview();
  await rename(archive,archive+"-old");await mkdir(archive);
  try{await press("接受导入");await browser.waitUntil(async()=> (await browser.$("main").getText()).includes("stale"),{timeout:10000});await saveScreenshot("stale-root-rejected")}
  finally{const {rmdir}=await import("node:fs/promises");await rmdir(archive);await rename(archive+"-old",archive)}
  await preview();await writeFile(join(archive,"Tweets/995000/a.bin"),"tampered");
  try{await press("接受导入");await browser.waitUntil(async()=> (await browser.$("main").getText()).includes("digest mismatch"),{timeout:10000});await saveScreenshot("stale-media-rejected")}
  finally{await writeFile(join(archive,"Tweets/995000/a.bin"),"original")}
  const jobs=await browser.executeAsync(done=>window.__TAURI__.core.invoke("list_jobs_page",{request:{offset:0,limit:20}}).then(done));
  assert.equal(jobs.total,1,"only intentionally seeded Job remains");
 });
});
