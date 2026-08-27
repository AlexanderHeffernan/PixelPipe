import { execFileSync } from "node:child_process";
import fs from "node:fs";

const config = JSON.parse(
  fs.readFileSync("apps/desktop/src-tauri/tauri.conf.json", "utf8"),
);
const baseVersion = parseVersion(config.version);
const parseTags = (output) =>
  output
    .trim()
    .split("\n")
    .filter(Boolean)
    .map((tag) => parseVersion(tag.slice(1)));
const tags = parseTags(
  execFileSync("git", ["tag", "--list", "v[0-9]*"], { encoding: "utf8" }),
);
const headTags = parseTags(
  execFileSync("git", ["tag", "--points-at", "HEAD", "v[0-9]*"], {
    encoding: "utf8",
  }),
);

if (headTags.length > 0) {
  console.log(formatVersion(latest(headTags)));
  process.exit(0);
}

const previous = latest(tags);
const next =
  previous && compare(previous, baseVersion) >= 0
    ? { ...previous, patch: previous.patch + 1 }
    : baseVersion;
console.log(formatVersion(next));

function parseVersion(value) {
  const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(value);
  if (!match) throw new Error(`Expected stable semver, received: ${value}`);
  return { major: +match[1], minor: +match[2], patch: +match[3] };
}

function latest(versions) {
  return versions.reduce(
    (result, version) =>
      !result || compare(version, result) > 0 ? version : result,
    null,
  );
}

function compare(left, right) {
  return (
    left.major - right.major ||
    left.minor - right.minor ||
    left.patch - right.patch
  );
}

function formatVersion(version) {
  return `${version.major}.${version.minor}.${version.patch}`;
}
