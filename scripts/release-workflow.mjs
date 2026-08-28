import fs from "node:fs";

const updaterPlatforms = [
  "darwin-aarch64",
  "darwin-aarch64-app",
  "darwin-x86_64",
  "darwin-x86_64-app",
  "linux-x86_64",
];
const cliAssets = [
  "pixelate-aarch64-apple-darwin",
  "pixelate-aarch64-apple-darwin.sig",
  "pixelate-x86_64-apple-darwin",
  "pixelate-x86_64-apple-darwin.sig",
  "pixelate-x86_64-unknown-linux-gnu",
  "pixelate-x86_64-unknown-linux-gnu.sig",
];

function releaseByTag(releases, tag) {
  const matches = releases.filter((release) => release?.tag_name === tag);
  if (matches.length !== 1) {
    throw new Error(
      `Expected one release tagged ${tag}, found ${matches.length}.`,
    );
  }
  if (!Number.isInteger(matches[0].id))
    throw new Error(`${tag} has no release ID.`);
  return matches[0];
}

function manifestAsset(release, tag) {
  if (release?.tag_name !== tag)
    throw new Error(`Release does not match ${tag}.`);
  const matches =
    release.assets?.filter((asset) => asset.name === "latest.json") ?? [];
  if (matches.length !== 1 || matches[0].state !== "uploaded") {
    throw new Error(`Expected one uploaded latest.json for ${tag}.`);
  }
  return matches[0];
}

function validateManifest(manifest, tag) {
  const version = tag.replace(/^v/, "");
  if (`v${version}` !== tag || manifest.version !== version) {
    throw new Error(`Updater manifest version does not match ${tag}.`);
  }
  for (const platform of updaterPlatforms) {
    const entry = manifest.platforms?.[platform];
    if (!entry?.url || !entry?.signature) {
      throw new Error(`Updater manifest is missing ${platform}.`);
    }
  }
}

function validateAssets(release, tag) {
  const uploaded = new Set(
    (release.assets ?? [])
      .filter((asset) => asset.state === "uploaded")
      .map((asset) => asset.name),
  );
  for (const asset of cliAssets) {
    if (!uploaded.has(asset)) throw new Error(`Release is missing ${asset}.`);
  }
  const version = tag?.replace(/^v/, "");
  for (const asset of [
    `Pixelate_${version}_amd64.AppImage`,
    `Pixelate_${version}_amd64.AppImage.sig`,
    `Pixelate_${version}_amd64.deb`,
    `Pixelate_${version}_amd64.deb.sig`,
  ]) {
    if (!uploaded.has(asset)) throw new Error(`Release is missing ${asset}.`);
  }
}

const [command, file, tag] = process.argv.slice(2);
const json = () => JSON.parse(fs.readFileSync(file, "utf8"));

if (command === "release-id") console.log(releaseByTag(json(), tag).id);
else if (command === "manifest-id") console.log(manifestAsset(json(), tag).id);
else if (command === "validate-manifest") validateManifest(json(), tag);
else if (command === "validate-assets") validateAssets(json(), tag);
else if (command === "verify-published") {
  const release = json();
  if (release.tag_name !== tag || release.draft !== false) {
    throw new Error(`${tag} was not published correctly.`);
  }
} else {
  throw new Error(
    "Usage: release-workflow.mjs (release-id|manifest-id|validate-manifest|validate-assets|verify-published) <json> [tag]",
  );
}
