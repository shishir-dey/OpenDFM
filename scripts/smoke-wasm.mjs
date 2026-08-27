import { analyzeGerberFile, analyzeGerberSource } from './gerber-wasm.mjs';

const inputPath = process.argv[2];
const result = inputPath
  ? await analyzeGerberFile(inputPath)
  : await analyzeGerberSource('empty.gbr', new Uint8Array());

if (!result.fileName) throw new Error('Expected a Gerber file name.');
if (!inputPath && result.byteLength !== 0) throw new Error('Expected an empty Gerber input.');
if (result.layers.length !== 0) throw new Error('Expected no parsed layers.');
if (result.violations.length !== 0) throw new Error('Expected no DFM violations.');
if (result.svg !== undefined) throw new Error('Expected no SVG output.');

console.log(JSON.stringify({
  input: inputPath ?? 'empty fixture',
  fileName: result.fileName,
  byteLength: result.byteLength,
  layers: result.layers.length,
  violations: result.violations.length,
}, null, 2));
