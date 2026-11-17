#!/bin/sh
# Export PilonDesigner's pipe catalogue (web/src/model/catalog.ts) to
# crates/creabim-services/data/pipe-types.json. Needs Node 22 or later.
#   tools/pipe-types/export.sh [path/to/pilondesigner]
set -e
here=$(cd "$(dirname "$0")" && pwd)
pd=${1:-$here/../../../pilondesigner}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/catalogs"
cp "$pd/web/src/model/catalog.ts" "$tmp/"
cp "$pd"/web/src/model/catalogs/*.json "$pd/web/src/model/catalogs/herz.ts" "$tmp/catalogs/"
# Node runs TypeScript as it is once imports name their files
sed -i -E "s#from '(\./catalogs/[^']+\.json)';#from '\1' with { type: 'json' };#; s#from './catalogs/herz';#from './catalogs/herz.ts';#" "$tmp/catalog.ts"
sed -i "s#from '../catalog';#from '../catalog.ts';#" "$tmp/catalogs/herz.ts"
cat > "$tmp/dump.mjs" <<'JS'
import { catalog } from './catalog.ts';
import { writeFileSync } from 'node:fs';
writeFileSync(process.argv[2], JSON.stringify(Object.values(catalog), null, 1) + '\n');
console.log(Object.keys(catalog).length + ' pipe types');
JS
node --experimental-strip-types --no-warnings "$tmp/dump.mjs" "$here/../../crates/creabim-services/data/pipe-types.json"
