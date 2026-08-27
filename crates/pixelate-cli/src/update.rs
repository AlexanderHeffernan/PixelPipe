use std::{env, fs, io::Write, path::Path};

use minisign_verify::{PublicKey, Signature};
use semver::Version;
use serde::Deserialize;
use serde_json::json;

const LATEST_URL: &str =
    "https://github.com/AlexanderHeffernan/Pixelate/releases/latest/download/latest.json";
const RELEASES_URL: &str = "https://github.com/AlexanderHeffernan/Pixelate/releases/download";
const PUBLIC_KEY: &str = "RWR3qMarrrU56LS4kYgS19RmVbArmidUmshk6QkHwT97k4JsK2pttkU8";
const MAX_DOWNLOAD_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Deserialize)]
struct LatestRelease {
    version: String,
}

pub(crate) fn update_cli() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let current = Version::parse(env!("CARGO_PKG_VERSION"))?;
    let latest: LatestRelease = serde_json::from_slice(&download(LATEST_URL)?)?;
    let available = Version::parse(&latest.version)?;
    if available <= current {
        return Ok(json!({
            "ok": true,
            "updated": false,
            "version": current.to_string(),
            "message": "Pixelate CLI is up to date",
        }));
    }

    let target = release_target()?;
    let executable = env::current_exe()?;
    if is_bundled_cli(&executable) {
        return Err(
            "this CLI is bundled with the Pixelate app; update it from Pixelate Settings".into(),
        );
    }

    let asset = format!("pixelate-{target}");
    let base = format!("{RELEASES_URL}/v{available}/{asset}");
    let binary = download(&base)?;
    let signature = String::from_utf8(download(&format!("{base}.sig"))?)?;
    verify(&binary, &signature)?;
    replace_executable(&executable, &binary)?;

    Ok(json!({
        "ok": true,
        "updated": true,
        "previous_version": current.to_string(),
        "version": available.to_string(),
        "executable": executable,
    }))
}

fn download(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut response = ureq::get(url)
        .header(
            "User-Agent",
            concat!("pixelate/", env!("CARGO_PKG_VERSION")),
        )
        .call()?;
    Ok(response
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD_BYTES)
        .read_to_vec()?)
}

fn verify(binary: &[u8], signature: &str) -> Result<(), Box<dyn std::error::Error>> {
    let key = PublicKey::from_base64(PUBLIC_KEY)?;
    let signature = Signature::decode(signature)?;
    key.verify(binary, &signature, false)?;
    Ok(())
}

fn release_target() -> Result<&'static str, Box<dyn std::error::Error>> {
    match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        _ => Err("automatic CLI updates currently support macOS only".into()),
    }
}

fn is_bundled_cli(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str().to_string_lossy().ends_with(".app"))
}

#[cfg(unix)]
fn replace_executable(path: &Path, binary: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    let temporary = path.with_file_name(format!(".pixelate-update-{}", std::process::id()));
    let result = (|| {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(binary)?;
        file.sync_all()?;
        let mode = fs::metadata(path)?.permissions().mode();
        fs::set_permissions(&temporary, fs::Permissions::from_mode(mode))?;
        fs::rename(&temporary, path)?;
        Ok::<(), std::io::Error>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(Into::into)
}

#[cfg(not(unix))]
fn replace_executable(_path: &Path, _binary: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    Err("automatic CLI updates currently support macOS only".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_a_cli_inside_an_app_bundle() {
        assert!(is_bundled_cli(Path::new(
            "/Applications/Pixelate.app/Contents/MacOS/pixelate"
        )));
        assert!(!is_bundled_cli(Path::new("/usr/local/bin/pixelate")));
    }
}
