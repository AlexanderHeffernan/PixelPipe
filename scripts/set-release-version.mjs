import fs from "node:fs";

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  throw new Error(
    "Usage: node scripts/set-release-version.mjs <major.minor.patch>",
  );
}

updateJson("apps/desktop/package.json", (value) => {
  value.version = version;
});
updateJson("apps/desktop/package-lock.json", (value) => {
  value.version = version;
  value.packages[""].version = version;
});
updateJson("apps/desktop/src-tauri/tauri.conf.json", (value) => {
  value.version = version;
});

replace("Cargo.toml", /(^version = ")[^"]+("$)/m);
for (const name of [
  "pixelate-app",
  "pixelate-cli",
  "pixelate-core",
  "pixelate-desktop",
  "pixelate-project",
]) {
  replace(
    "Cargo.lock",
    new RegExp(
      `(\\[\\[package\\]\\]\\nname = "${name}"\\nversion = ")[^"]+("\\n)`,
    ),
  );
}

function updateJson(path, update) {
  const value = JSON.parse(fs.readFileSync(path, "utf8"));
  update(value);
  fs.writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function replace(path, pattern) {
  const contents = fs.readFileSync(path, "utf8");
  if (!pattern.test(contents))
    throw new Error(`Could not find version in ${path}`);
  fs.writeFileSync(path, contents.replace(pattern, `$1${version}$2`));
}
