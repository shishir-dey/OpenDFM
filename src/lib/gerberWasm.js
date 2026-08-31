import initWasm, { analyze_gerber as analyzeGerberWasm } from '../../wasm/pkg/opendfm_wasm.js';

let wasmPromise;

function loadWasm() {
  if (!wasmPromise) wasmPromise = initWasm();
  return wasmPromise;
}

export async function analyzeGerberFiles(files) {
  await loadWasm();
  const results = await Promise.all(
    files.map(async (file) => {
      const source = new Uint8Array(await file.arrayBuffer());
      return analyzeGerberWasm(file.name, source);
    }),
  );

  const layers = results.flatMap((result, fileIndex) =>
    result.layers.map((layer, layerIndex) => ({
      ...layer,
      id: `gerber-layer-${fileIndex}-${layerIndex}`,
    })),
  ).sort((a, b) => layerOrder(a.kind) - layerOrder(b.kind));

  const drawable = layers.filter((layer) => !layer.bounds.isEmpty);
  const minX = drawable.length ? Math.min(...drawable.map((layer) => layer.bounds.minX)) : 0;
  const minY = drawable.length ? Math.min(...drawable.map((layer) => layer.bounds.minY)) : 0;
  const maxX = drawable.length ? Math.max(...drawable.map((layer) => layer.bounds.maxX)) : 100;
  const maxY = drawable.length ? Math.max(...drawable.map((layer) => layer.bounds.maxY)) : 100;
  const width = Math.max(maxX - minX, 1);
  const height = Math.max(maxY - minY, 1);
  const padding = Math.max(width, height) * 0.06;
  const fitViewBox = {
    x: minX - padding,
    y: minY - padding,
    width: width + padding * 2,
    height: height + padding * 2,
  };

  return {
    layers,
    fitViewBox,
    board: {
      widthMm: drawable.length ? width : 0,
      heightMm: drawable.length ? height : 0,
      layerCount: layers.filter((layer) => layer.kind.includes('copper')).length,
      holeCount: sum(results, 'holeCount'),
      padCount: sum(results, 'padCount'),
      minimumTrackWidthMm: minimumPositive(results, 'minimumTrackWidthMm'),
      minimumHoleDiameterMm: minimumPositive(results, 'minimumHoleDiameterMm'),
    },
    warnings: results.flatMap((result) =>
      result.warnings.map((warning) => `${result.fileName}: ${warning}`),
    ),
  };
}

function sum(results, key) {
  return results.reduce((total, result) => total + (result.board[key] || 0), 0);
}

function minimumPositive(results, key) {
  const values = results.map((result) => result.board[key]).filter((value) => value > 0);
  return values.length ? Math.min(...values) : 0;
}

function layerOrder(kind) {
  const order = {
    'bottom-silkscreen': 10,
    'bottom-soldermask': 20,
    'bottom-copper': 30,
    'inner-copper': 40,
    'top-copper': 50,
    'top-soldermask': 60,
    'top-paste': 70,
    'bottom-paste': 75,
    'top-silkscreen': 80,
    outline: 90,
    drill: 100,
  };
  return order[kind] ?? 45;
}
