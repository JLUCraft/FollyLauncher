const fs = require("fs");
const path = require("path");

const brandId = process.argv[2];

if (!brandId) {
  console.error("Usage: pnpm brand <brand-id>");
  process.exit(1);
}

const repoRoot = path.join(__dirname, "../..");
const brandPath = path.join(repoRoot, "brands", brandId, "brand.json");

if (!fs.existsSync(brandPath)) {
  console.error(`Brand definition not found: brands/${brandId}/brand.json`);
  process.exit(1);
}

const brand = JSON.parse(fs.readFileSync(brandPath, "utf8"));

if (brand.id !== brandId) {
  console.error(
    `brand.json id "${brand.id}" does not match requested brand "${brandId}"`
  );
  process.exit(1);
}

if (!/^[a-zA-Z0-9-]+(\.[a-zA-Z0-9-]+)+$/.test(brand.identifier)) {
  console.error(
    `Invalid identifier "${brand.identifier}": expected reverse-DNS form ` +
      "with at least two segments (e.g. com.example.launcher)"
  );
  process.exit(1);
}

if (!/^[a-z][a-z0-9+.-]*$/.test(brand.deeplinkScheme)) {
  console.error(`Invalid deeplink scheme "${brand.deeplinkScheme}"`);
  process.exit(1);
}

// Rewrite native identity in tauri.conf.json.
const tauriConfigPath = path.join(repoRoot, "src-tauri/tauri.conf.json");
const tauriConfig = JSON.parse(fs.readFileSync(tauriConfigPath, "utf8"));
tauriConfig.productName = brand.productName;
tauriConfig.identifier = brand.identifier;
tauriConfig.bundle.copyright = brand.copyright;
tauriConfig.plugins["deep-link"].desktop.schemes = [brand.deeplinkScheme];

const iconNames = [
  "32x32.png",
  "64x64.png",
  "128x128.png",
  "128x128@2x.png",
  "icon.icns",
  "icon.ico",
  "icon.png",
];
const iconDir = path.join(repoRoot, "brands", brandId, "icons");
if (fs.existsSync(iconDir)) {
  const present = iconNames.filter((name) =>
    fs.existsSync(path.join(iconDir, name))
  );
  if (present.length > 0) {
    tauriConfig.bundle.icon = present.map(
      (name) => `../brands/${brandId}/icons/${name}`
    );
  }
}

fs.writeFileSync(tauriConfigPath, JSON.stringify(tauriConfig, null, 2) + "\n");

// Expose the brand to the frontend at runtime.
const genPath = path.join(repoRoot, "src/brand.gen.json");
fs.writeFileSync(genPath, JSON.stringify(brand, null, 2) + "\n");

console.log(
  `Applied brand "${brandId}": productName=${brand.productName}, ` +
    `identifier=${brand.identifier}, scheme=${brand.deeplinkScheme}`
);
