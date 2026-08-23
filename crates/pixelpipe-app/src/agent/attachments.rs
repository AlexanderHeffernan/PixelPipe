use std::{
    io::Read,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use reqwest::{StatusCode, Url, blocking::Client, redirect::Policy};

use crate::AppError;

const MAX_CANDIDATE_BYTES: u64 = 32 * 1024 * 1024;
const AMP_ATTACHMENT_PREFIX: &str = "/user-content/attachments/";

pub(super) fn validate_amp_attachment_url(value: &str) -> Result<Url, AppError> {
    let url = Url::parse(value)
        .map_err(|_| AppError::AgentProtocol("invalid Amp attachment URL".to_owned()))?;
    let valid = url.scheme() == "https"
        && url.host_str() == Some("ampcode.com")
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && url.path().starts_with(AMP_ATTACHMENT_PREFIX)
        && url.path().len() > AMP_ATTACHMENT_PREFIX.len()
        && !url.path()[AMP_ATTACHMENT_PREFIX.len()..].contains('/');
    if !valid {
        return Err(AppError::AgentProtocol(
            "attachment URL is not an approved Amp attachment".to_owned(),
        ));
    }
    Ok(url)
}

pub(super) fn download_amp_attachment(
    value: &str,
    api_key: &str,
    cancel: &AtomicBool,
) -> Result<Vec<u8>, AppError> {
    let url = validate_amp_attachment_url(value)?;
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .redirect(Policy::limited(3))
        .build()
        .map_err(|source| {
            AppError::AgentProcess(format!("cannot prepare attachment import: {source}"))
        })?;
    let response = client
        .get(url)
        .bearer_auth(api_key)
        .send()
        .map_err(|source| {
            AppError::AgentProcess(format!("cannot import Amp attachment: {source}"))
        })?;
    if response.status() != StatusCode::OK {
        return Err(AppError::AgentProcess(format!(
            "Amp attachment import returned HTTP {}",
            response.status().as_u16()
        )));
    }
    if response
        .content_length()
        .is_some_and(|size| size > MAX_CANDIDATE_BYTES)
    {
        return Err(AppError::AgentProcess(
            "Amp attachment exceeds the 32 MiB candidate limit".to_owned(),
        ));
    }
    read_bounded(response, cancel)
}

fn read_bounded(
    mut response: reqwest::blocking::Response,
    cancel: &AtomicBool,
) -> Result<Vec<u8>, AppError> {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 16_384];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(AppError::AgentProcess("task cancelled by user".to_owned()));
        }
        let read = response.read(&mut chunk).map_err(|source| {
            AppError::AgentProcess(format!("cannot read Amp attachment: {source}"))
        })?;
        if read == 0 {
            break;
        }
        if bytes.len().saturating_add(read)
            > usize::try_from(MAX_CANDIDATE_BYTES).unwrap_or(usize::MAX)
        {
            return Err(AppError::AgentProcess(
                "Amp attachment exceeds the 32 MiB candidate limit".to_owned(),
            ));
        }
        bytes.extend_from_slice(&chunk[..read]);
    }
    Ok(bytes)
}
