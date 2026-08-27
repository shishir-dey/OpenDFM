import { readFile } from 'node:fs/promises';
import { basename } from 'node:path';
import initWasm, { analyze_gerber as analyzeGerberWasm } from '../wasm/pkg/opendfm_wasm.js';

let processorPromise;

export function loadGerberWasm() {
  if (!processorPromise) {
    processorPromise = readFile(
      new URL('../wasm/pkg/opendfm_wasm_bg.wasm', import.meta.url),
    ).then((wasmBytes) => initWasm({ module_or_path: wasmBytes }));
  }

  return processorPromise;
}

export async function analyzeGerberSource(fileName, source) {
  await loadGerberWasm();
  const bytes = source instanceof Uint8Array ? source : new Uint8Array(source);
  return analyzeGerberWasm(fileName, bytes);
}

export async function analyzeGerberFile(inputPath) {
  const source = await readFile(inputPath);
  return analyzeGerberSource(basename(inputPath), source);
}
