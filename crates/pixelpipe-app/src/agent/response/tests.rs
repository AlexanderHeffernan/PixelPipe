use std::sync::atomic::AtomicBool;

use pixelpipe_project::{AgentOperation, ProjectStore};
use tempfile::tempdir;

use super::*;

const ATTACHMENT: &str = "https://ampcode.com/user-content/attachments/fixture-file.png";

#[test]
fn imports_an_approved_attachment_and_derives_its_own_hash() {
    let project = tempdir().expect("project");
    let output = tempdir().expect("output");
    ProjectStore::init(project.path(), "Response fixture").expect("project init");
    let store = ProjectStore::discover(project.path()).expect("store");
    let png = fixture_png();
    let fetch = |url: &str, _: &AtomicBool| {
        assert_eq!(url, ATTACHMENT);
        Ok(png.clone())
    };
    let cancel = AtomicBool::new(false);
    let context = context(&store, output.path(), &fetch, &cancel);

    let response = process_response(
        &response_with(&candidate("reference-one", ATTACHMENT)),
        &context,
    )
    .expect("valid response");

    assert_eq!(response.candidates.len(), 1);
    assert_eq!(response.candidates[0].sha256, sha256_hex(&png));
    assert_eq!(response.candidate_bytes["reference-one"], png);
}

#[test]
fn rejects_unapproved_attachment_origins_before_fetching() {
    for url in [
        "http://ampcode.com/user-content/attachments/file.png",
        "https://example.com/user-content/attachments/file.png",
        "https://ampcode.com/user-content/attachments/nested/file.png",
        "https://ampcode.com/user-content/attachments/file.png?download=1",
    ] {
        let project = tempdir().expect("project");
        let output = tempdir().expect("output");
        ProjectStore::init(project.path(), "Response fixture").expect("project init");
        let store = ProjectStore::discover(project.path()).expect("store");
        let fetch = |_: &str, _: &AtomicBool| -> Result<Vec<u8>, AppError> {
            panic!("invalid URL must not be fetched")
        };
        let cancel = AtomicBool::new(false);
        let context = context(&store, output.path(), &fetch, &cancel);

        let error = process_response(&response_with(&candidate("reference-one", url)), &context)
            .expect_err("URL must be rejected");
        assert!(error.to_string().contains("approved Amp attachment"));
    }
}

#[test]
fn rejects_malformed_downloads_without_returning_partial_candidates() {
    let project = tempdir().expect("project");
    let output = tempdir().expect("output");
    ProjectStore::init(project.path(), "Response fixture").expect("project init");
    let store = ProjectStore::discover(project.path()).expect("store");
    let fetch = |url: &str, _: &AtomicBool| {
        if url.ends_with("second.png") {
            Ok(b"not a PNG".to_vec())
        } else {
            Ok(fixture_png())
        }
    };
    let cancel = AtomicBool::new(false);
    let context = context(&store, output.path(), &fetch, &cancel);
    let candidates = format!(
        "{},{}",
        candidate("reference-one", ATTACHMENT),
        candidate(
            "reference-two",
            "https://ampcode.com/user-content/attachments/second.png"
        )
    );

    let error = process_response(&response_with(&candidates), &context)
        .expect_err("whole response must fail");

    assert!(error.to_string().contains("PNG"));
}

#[test]
fn cancellation_discards_a_fetched_attachment() {
    let project = tempdir().expect("project");
    let output = tempdir().expect("output");
    ProjectStore::init(project.path(), "Response fixture").expect("project init");
    let store = ProjectStore::discover(project.path()).expect("store");
    let cancel = AtomicBool::new(false);
    let fetch = |_: &str, flag: &AtomicBool| {
        flag.store(true, Ordering::Relaxed);
        Ok(fixture_png())
    };
    let context = context(&store, output.path(), &fetch, &cancel);

    let error = process_response(
        &response_with(&candidate("reference-one", ATTACHMENT)),
        &context,
    )
    .expect_err("cancelled response must fail");

    assert!(error.to_string().contains("cancelled"));
}

fn context<'a>(
    store: &'a ProjectStore,
    output: &'a Path,
    fetch: &'a AttachmentFetcher<'a>,
    cancel: &'a AtomicBool,
) -> ResponseContext<'a> {
    ResponseContext {
        operation: AgentOperation::GenerateReferences,
        asset: "fixture",
        revision: None,
        store,
        output_directory: output,
        secrets: &[],
        cancel,
        attachment_fetcher: Some(fetch),
        progress: &|_| {},
    }
}

fn candidate(id: &str, url: &str) -> String {
    format!(r#"{{"id":"{id}","attachment_url":"{url}"}}"#)
}

fn response_with(candidates: &str) -> Vec<u8> {
    format!(
        r#"{{"schema":"pixelpipe.agent-response/v1","adapter":{{"adapter":"fixture","provider":"fixture","model":"fixture","capabilities":["generate_references"]}},"result":{{"type":"generated_references","candidates":[{candidates}]}}}}"#
    )
    .into_bytes()
}

fn fixture_png() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("PNG header");
        writer
            .write_image_data(&[
                255, 64, 32, 255, 0, 0, 0, 0, 32, 128, 255, 255, 255, 208, 80, 255,
            ])
            .expect("PNG data");
    }
    bytes
}
