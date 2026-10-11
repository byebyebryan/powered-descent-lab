import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const context = vm.createContext({});
vm.runInContext(readFileSync(new URL('../pd-report/src/input_coverage.js', import.meta.url), 'utf8'), context);
const model = context.PdInputCoverageModel;
const band = {collection:'layered',group:'mixed',band:'b3'};
const data = {previews:Array.from({length:97}, (_,index)=>({...band,index,physical_sha256:String(index).padStart(64,'a'),direction:index%2?-1:1})).concat(Array.from({length:41}, (_,index)=>({...band,band:'b0',index:100+index,direction:1,physical_sha256:'b'.repeat(64)})))};
const state = {band:model.keyOf(band),direction:'all',q:'',page:1};

test('one band and twelve previews per page even with many matching inputs', ()=>{
  const first=model.pageSelection(data,state);
  assert.equal(first.total,97);assert.equal(first.pages,9);assert.equal(first.missions.length,12);
  assert.ok(first.missions.every(m=>m.band==='b3'));
  const last=model.pageSelection(data,{...state,page:999});
  assert.equal(last.page,9);assert.equal(last.missions.length,1);assert.equal(last.missions[0].index,96);
  assert.equal(model.pageSelection(data,{...state,band:''}).missions.length,0);
});

test('physical direction stays a filter inside a shared morphology band', ()=>{
  const right=model.pageSelection(data,{...state,direction:'1'});
  const left=model.pageSelection(data,{...state,direction:'-1'});
  assert.equal(right.total+left.total,97);
  assert.ok(right.missions.every(m=>m.direction===1));
  assert.ok(left.missions.every(m=>m.direction===-1));
});

test('search and invalid pagination keep the selected preview denominator', ()=>{
  assert.equal(model.pageSelection(data,{...state,q:'000000'}).total,0);
  assert.equal(model.pageSelection(data,{...state,page:-3}).page,1);
  assert.equal(model.pageSelection(data,{...state,page:'bad'}).page,1);
  assert.equal(model.pageSelection(data,{...state,page:2.8}).page,2);
  assert.equal(data.previews.length,138);
});
