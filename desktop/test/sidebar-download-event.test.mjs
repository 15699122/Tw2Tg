import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {transform} from 'esbuild';
import React from 'react';

test('sidebar downloads and SQLite events invoke the supplied page refresh', async () => {
  const source=readFileSync(new URL('../src/main.jsx',import.meta.url),'utf8');
  const call=source.slice(source.indexOf('<Sidebar'),source.indexOf('<main className="main-panel">'));

  const body=source.slice(source.indexOf('function Sidebar('),source.indexOf('function NavItem('));
  const compiled=await transform(body,{loader:'jsx',jsxFactory:'React.createElement'});
  const stub=()=>null;
  const Sidebar=new Function('React','Icon','Separator','NavItem','ConnectionStatus','ExtensionConnectionStatus',compiled.code+';return Sidebar;')(React,stub,stub,stub,stub,stub);
  const actions=[];
  const tree=Sidebar({page:'dashboard',setPage:p=>actions.push(p),refreshDownloadPage:()=>actions.push('refresh'),status:{},openSettingsSection:()=>{}});
  const nodes=[];
  function walk(node){if(!node||typeof node!=='object')return;nodes.push(node);React.Children.forEach(node.props?.children,walk)}
  walk(tree);
  for(const label of ['下载任务与设置','SQLite']) {
    actions.length=0;
    const target=nodes.find(node=>node.props?.label===label);
    assert.ok(target);
    assert.doesNotThrow(()=>target.props.onClick());
    assert.deepEqual(actions,['downloads','refresh']);
    assert.match(call,/refreshDownloadPage=\{refreshDownloadPage\}/);
  }
});

