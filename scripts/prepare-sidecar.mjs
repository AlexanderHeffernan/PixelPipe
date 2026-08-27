import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repository = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const release = process.argv.includes("--release");
const profile = release ? "release" : "debug";
const target = execFileSync("rustc", ["--print", "host-tuple"], {
  encoding: "utf8",
}).trim();
const extension = process.platform === "win32" ? ".exe" : "";

execFileSync(
  "cargo",
  ["build", "-p", "pixelate-cli", ...(release ? ["--release"] : [])],
  { cwd: repository, stdio: "inherit" },
);

const source = path.join(repository, "target", profile, `pixelate${extension}`);
const destination = path.join(
  repository,
  "apps",
  "desktop",
  "src-tauri",
  "binaries",
  `pixelate-${target}${extension}`,
);
fs.mkdirSync(path.dirname(destination), { recursive: true });
fs.copyFileSync(source, destination);
