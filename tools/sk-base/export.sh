#!/bin/sh
# Export PilonDesigner's materials (web/src/model/materials.ts, with the
# PORFIX catalogue) and its usual Slovak build-ups of walls, floors and
# roofs (web/src/model/types.ts) to crates/creabim-core/data/sk-base.json.
# Needs Node 22 or later.
#   tools/sk-base/export.sh [path/to/pilondesigner]
set -e
here=$(cd "$(dirname "$0")" && pwd)
pd=${1:-$here/../../../pilondesigner}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/catalogs"
cp "$pd/web/src/model/materials.ts" "$tmp/"
cp "$pd/web/src/model/catalogs/porfix.ts" "$tmp/catalogs/"
# Node runs TypeScript as it is once imports name their files
sed -i "s#from './catalogs/porfix';#from './catalogs/porfix.ts';#" "$tmp/materials.ts"
# the build-ups: from the layer helper to the last list of them
{
  echo "import { defaultMaterials, layerFrom } from './materials.ts';"
  sed -n '/^const M = new Map(defaultMaterials()/,/^export const typeThickness/p' "$pd/web/src/model/types.ts" | sed '$d'
} > "$tmp/types.ts"
cat > "$tmp/dump.mjs" <<'JS'
import { defaultMaterials } from './materials.ts';
import { defaultWallTypes, defaultSlabTypes, defaultRoofTypes } from './types.ts';
import { writeFileSync } from 'node:fs';
const hex = (c) => '#' + c.toString(16).padStart(6, '0');
const materials = defaultMaterials().map(({ color, ...m }) => ({ ...m, color: hex(color) }));
const types = (list) => list.map((t) => ({
  id: t.id, name: t.name, boundary: t.boundary, fire: t.fireResistance, rw: t.rw, curtain: t.curtain,
  layers: t.layers.map((l) => ({ material: l.materialId, mm: Math.round(l.thickness * 1e5) / 100, core: l.core, vent: l.vent })),
}));
const out = { materials, walls: types(defaultWallTypes()), floors: types(defaultSlabTypes()), roofs: types(defaultRoofTypes()) };
writeFileSync(process.argv[2], JSON.stringify(out, null, 1) + '\n');
console.log(`${materials.length} materials, ${out.walls.length} walls, ${out.floors.length} floors, ${out.roofs.length} roofs`);
JS
mkdir -p "$here/../../crates/creabim-core/data"
node --experimental-strip-types --no-warnings "$tmp/dump.mjs" "$here/../../crates/creabim-core/data/sk-base.json"
