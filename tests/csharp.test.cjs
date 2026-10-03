const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const {spawnSync} = require('node:child_process');
const crypto = require('node:crypto');
const project = path.resolve(__dirname, '..');
const exe = process.env.CROPTIX_CSHARP_EXE || path.join(project, 'CrOptix Patcher.exe');
const main = "(()=>{ const playerControls='player-controls-root'; const nextEpisodeVM={}; })();";
const manifest = '{"name":"Extensão","content_scripts":[{"world":"MAIN","js":["pip1.js","katamari.js"]}],"web_accessible_resources":[{"resources":["pip.js","fonts/*"]}]}';
const prefix = "'croptix-rust-fixes:v2';\n" + fs.readFileSync(path.join(project, 'assets/runtime.js'), 'utf8') + "'croptix-rust-fixes:end';\n";
const hash = data => crypto.createHash('sha256').update(data).digest('hex');
function fixture(t, source = main, json = manifest) {
  const folder = fs.mkdtempSync(path.join(os.tmpdir(), 'croptix-csharp-test-'));
  t.after(() => fs.rmSync(folder, {recursive:true, force:true}));
  fs.writeFileSync(path.join(folder,'katamari.js'),source);
  fs.writeFileSync(path.join(folder,'manifest.json'),json);
  return folder;
}
function run(folder, command='--apply') {
  assert.ok(fs.existsSync(exe), 'O executável C# ainda não foi implementado/compilado');
  const result = spawnSync(exe,[command,folder],{encoding:'utf8',timeout:10000});
  assert.ifError(result.error);
  return result.status;
}
const read = (folder,name) => fs.readFileSync(path.join(folder,name),'utf8');
test('C# insere exatamente o runtime Rust e preserva outros recursos', t => {
  const folder=fixture(t); assert.equal(run(folder),0);
  assert.equal(read(folder,'katamari.js'),prefix+main);
  const actual=JSON.parse(read(folder,'manifest.json'));
  assert.deepEqual(actual.content_scripts[0].js,['katamari.js']);
  assert.deepEqual(actual.web_accessible_resources[0].resources,['fonts/*']);
  assert.equal(actual.name,'Extensão');
});
test('reaplicar não duplica patch nem cria backup; restaurar é byte exato',t=>{
  const folder=fixture(t);assert.equal(run(folder),0);
  const backups=path.join(folder,'.croptix-rust-backups');
  const snapshots=fs.readdirSync(backups);const first=read(folder,'katamari.js');
  assert.equal(run(folder),0);assert.equal(read(folder,'katamari.js'),first);
  assert.deepEqual(fs.readdirSync(backups),snapshots);
  assert.equal(run(folder,'--restore'),0);
  assert.equal(read(folder,'katamari.js'),main);assert.equal(read(folder,'manifest.json'),manifest);
});
test('patch Rust atual é reconhecido sem mudança',t=>{
  const json='{"content_scripts":[{"world":"MAIN","js":["katamari.js"]}]}';
  const folder=fixture(t,prefix+main,json);assert.equal(run(folder),0);
  assert.equal(read(folder,'katamari.js'),prefix+main);assert.equal(read(folder,'manifest.json'),json);
  assert.equal(fs.existsSync(path.join(folder,'.croptix-rust-backups')),false);
});
test('migra Rust v1 e restaura exatamente a versão anterior',t=>{
  const old="'croptix-rust-fixes:v1';\n"+fs.readFileSync(path.join(project,'assets/runtime-v1.js'),'utf8')+"'croptix-rust-fixes:end';\n"+main;
  const folder=fixture(t,old);assert.equal(run(folder),0);assert.equal(read(folder,'katamari.js'),prefix+main);
  assert.equal(run(folder,'--restore'),0);assert.equal(read(folder,'katamari.js'),old);
});
for(const [name,source,json] of [
  ['manifest inválido',main,'{broken'],
  ['mundo isolado',main,'{"content_scripts":[{"world":"ISOLATED","js":["katamari.js"]}]}'],
  ['patch adulterado',prefix.replace('let pipSession = null','let pipSession = undefined')+main,manifest],
  ['Katamari incompatível','(()=>{})();',manifest]
]) test('recusa '+name+' sem modificar arquivos',t=>{
  const folder=fixture(t,source,json);assert.notEqual(run(folder),0);
  assert.equal(read(folder,'katamari.js'),source);assert.equal(read(folder,'manifest.json'),json);
  assert.equal(fs.existsSync(path.join(folder,'.croptix-rust-backups')),false);
});
test('não restaura sobre atualização externa',t=>{
  const folder=fixture(t);assert.equal(run(folder),0);fs.writeFileSync(path.join(folder,'katamari.js'),'updated');
  assert.notEqual(run(folder,'--restore'),0);assert.equal(read(folder,'katamari.js'),'updated');
});
test('restaura backup com metadados e hashes no formato Rust',t=>{
  const json='{"content_scripts":[{"world":"MAIN","js":["katamari.js"]}]}';
  const folder=fixture(t,prefix+main,json);
  const snapshot=path.join(folder,'.croptix-rust-backups','snapshot-123');fs.mkdirSync(snapshot,{recursive:true});
  fs.writeFileSync(path.join(snapshot,'katamari.js'),main);fs.writeFileSync(path.join(snapshot,'manifest.json'),json);
  fs.writeFileSync(path.join(snapshot,'info.json'),JSON.stringify({folder:'\\\\?\\'+fs.realpathSync.native(folder),created:'123',before:[hash(main),hash(json)],after:[hash(prefix+main),hash(json)]}));
  assert.equal(run(folder,'--restore'),0);assert.equal(read(folder,'katamari.js'),main);
});
test('backup corrompido não altera os arquivos atuais',t=>{
  const folder=fixture(t);assert.equal(run(folder),0);
  const backups=path.join(folder,'.croptix-rust-backups');const snapshot=path.join(backups,fs.readdirSync(backups)[0]);
  fs.writeFileSync(path.join(snapshot,'katamari.js'),'corrupted');const before=read(folder,'katamari.js');
  assert.notEqual(run(folder,'--restore'),0);assert.equal(read(folder,'katamari.js'),before);
});
test('seletor usa o Explorer sem abrir janela durante o teste',t=>{
  const folder=fixture(t);assert.equal(run(folder,'--picker-self-test'),0);
});
test('pasta incompleta não modifica o Katamari',t=>{
  const folder=fixture(t);fs.unlinkSync(path.join(folder,'manifest.json'));
  assert.notEqual(run(folder),0);assert.equal(read(folder,'katamari.js'),main);
});
test('migra o patch Node assinado e recusa assinatura adulterada',t=>{
  const code=';(function(){})();\n';
  const old='// <CROPTIX-INDEPENDENT-FIXES v1>\n// SHA256: '+hash(code)+'\n'+code+'// </CROPTIX-INDEPENDENT-FIXES>\n'+main;
  const folder=fixture(t,old);assert.equal(run(folder),0);assert.equal(read(folder,'katamari.js'),prefix+main);
  assert.equal(run(folder,'--restore'),0);assert.equal(read(folder,'katamari.js'),old);
  fs.writeFileSync(path.join(folder,'katamari.js'),old.replace(code,';(function(){bad})();\n'));
  assert.notEqual(run(folder),0);assert.equal(read(folder,'katamari.js'),old.replace(code,';(function(){bad})();\n'));
});
test('backup redirecionado para outra pasta é recusado',t=>{
  const folder=fixture(t),outside=fixture(t);const link=path.join(folder,'.croptix-rust-backups');
  fs.symlinkSync(outside,link,'junction');
  assert.notEqual(run(folder),0);assert.equal(read(folder,'katamari.js'),main);
  assert.deepEqual(fs.readdirSync(outside).sort(),['katamari.js','manifest.json']);
});
test('ignora snapshot mais novo inválido ao restaurar',t=>{
  const folder=fixture(t);assert.equal(run(folder),0);
  const snapshot=path.join(folder,'.croptix-rust-backups','snapshot-9999999999999999999999');
  fs.mkdirSync(snapshot);fs.writeFileSync(path.join(snapshot,'info.json'),'{broken');
  assert.equal(run(folder,'--restore'),0);assert.equal(read(folder,'katamari.js'),main);
});
test('falha ao gravar manifesto desfaz mudança já feita no Katamari',t=>{
  const folder=fixture(t),file=path.join(folder,'manifest.json');fs.chmodSync(file,0o444);
  try {
    assert.notEqual(run(folder),0);
    assert.equal(read(folder,'katamari.js'),main);assert.equal(read(folder,'manifest.json'),manifest);
    assert.equal(fs.readdirSync(folder).some(name=>name.endsWith('.tmp')),false);
  } finally {fs.chmodSync(file,0o666);}
});
test('Katamari com UTF-8 inválido é recusado sem alteração',t=>{
  const folder=fixture(t);const bytes=Buffer.from([0xff,0xfe,0x41]);fs.writeFileSync(path.join(folder,'katamari.js'),bytes);
  assert.notEqual(run(folder),0);assert.deepEqual(fs.readFileSync(path.join(folder,'katamari.js')),bytes);
  assert.equal(read(folder,'manifest.json'),manifest);
});
const rustExe=process.env.CROPTIX_RUST_EXE;
test('equivalência byte a byte e restauração cruzada com executável Rust real',{skip:!rustExe},t=>{
  const folder=fixture(t);const other=fixture(t);
  const rust=(dir,command)=>{const result=spawnSync(rustExe,[command,dir],{encoding:'utf8',timeout:10000});assert.ifError(result.error);assert.equal(result.status,0,result.stderr);};
  rust(folder,'--apply');assert.equal(run(other),0);
  assert.equal(read(other,'katamari.js'),read(folder,'katamari.js'));
  assert.equal(read(other,'manifest.json'),read(folder,'manifest.json'));
  assert.equal(run(folder,'--restore'),0);assert.equal(read(folder,'katamari.js'),main);assert.equal(read(folder,'manifest.json'),manifest);
  rust(other,'--restore');assert.equal(read(other,'katamari.js'),main);assert.equal(read(other,'manifest.json'),manifest);
});
