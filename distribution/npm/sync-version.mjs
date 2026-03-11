import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const version = process.argv[2];

if (!version) {
  console.error("Usage: node distribution/npm/sync-version.mjs <version>");
  process.exit(1);
}

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const root = scriptDir;
const mainPackagePath = path.join(root, "package.json");
const mainPackage = JSON.parse(fs.readFileSync(mainPackagePath, "utf8"));

mainPackage.version = version;
for (const dep of Object.keys(mainPackage.optionalDependencies ?? {})) {
  mainPackage.optionalDependencies[dep] = version;
}
fs.writeFileSync(mainPackagePath, `${JSON.stringify(mainPackage, null, 2)}\n`);

const platformRoot = path.join(root, "platform-packages");
if (fs.existsSync(platformRoot)) {
  for (const entry of fs.readdirSync(platformRoot, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const pkgPath = path.join(platformRoot, entry.name, "package.json");
    if (!fs.existsSync(pkgPath)) continue;
    const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
    pkg.version = version;
    fs.writeFileSync(pkgPath, `${JSON.stringify(pkg, null, 2)}\n`);
  }
}

console.log(`distribution/npm synced to ${version}`);
