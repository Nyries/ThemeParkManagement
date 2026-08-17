// TPM-185: propagates the root VERSION file into the package.json/Cargo.toml
// `version` fields, which nothing in this repo reads at runtime — the engine and
// client get their version straight from VERSION at build time (apps/engine/build.rs,
// apps/client/vite.config.ts). This script only keeps those manifest fields from
// looking stale; run it by hand after bumping VERSION.
import { readFileSync, writeFileSync } from "fs";
import { fileURLToPath } from "url";
import path from "path";

const rootDir = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const version = readFileSync(path.join(rootDir, "VERSION"), "utf-8").trim();

const packageJsonPaths = [
  "package.json",
  "apps/client/package.json",
  "apps/gateway/package.json",
];

for (const relativePath of packageJsonPaths) {
  const filePath = path.join(rootDir, relativePath);
  const pkg = JSON.parse(readFileSync(filePath, "utf-8"));
  pkg.version = version;
  writeFileSync(filePath, JSON.stringify(pkg, null, 2) + "\n");
  console.log(`${relativePath} -> ${version}`);
}

const cargoTomlPath = path.join(rootDir, "apps/engine/Cargo.toml");
const cargoToml = readFileSync(cargoTomlPath, "utf-8");
const updatedCargoToml = cargoToml.replace(
  /^version = ".*"$/m,
  `version = "${version}"`,
);
writeFileSync(cargoTomlPath, updatedCargoToml);
console.log(`apps/engine/Cargo.toml -> ${version}`);
