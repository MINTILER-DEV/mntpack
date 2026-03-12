use anyhow::{Context, Result};
use reqwest::StatusCode;
use serde::Serialize;

use crate::{config::RuntimeContext, package::record::PackageRecord};

#[derive(Debug, Serialize)]
struct DispatchRequest<'a> {
    event_type: &'a str,
    client_payload: DispatchPayload<'a>,
}

#[derive(Debug, Serialize)]
struct DispatchPayload<'a> {
    owner: &'a str,
    repo: &'a str,
    package_name: &'a str,
    requested_version: Option<&'a str>,
    resolved_version: Option<&'a str>,
    commit: Option<&'a str>,
}

pub async fn dispatch_sync(
    runtime: &RuntimeContext,
    record: &PackageRecord,
    requested_version: Option<&str>,
) -> Result<()> {
    if !runtime.config.sync_dispatch.enabled {
        return Ok(());
    }

    let target_repo = runtime.config.sync_dispatch.repo.trim();
    if target_repo.is_empty() {
        return Ok(());
    }
    let token_env = runtime.config.sync_dispatch.token_env.trim();
    if token_env.is_empty() {
        return Ok(());
    }

    let token = match std::env::var(token_env) {
        Ok(value) if !value.trim().is_empty() => value,
        _ => {
            eprintln!(
                "warning: sync dispatch skipped; env var '{}' is not set",
                token_env
            );
            return Ok(());
        }
    };

    let event_type = runtime.config.sync_dispatch.event_type.trim();
    if event_type.is_empty() {
        return Ok(());
    }

    let payload = DispatchRequest {
        event_type,
        client_payload: DispatchPayload {
            owner: &record.owner,
            repo: &record.repo,
            package_name: &record.package_name,
            requested_version,
            resolved_version: record.version.as_deref(),
            commit: record.commit.as_deref(),
        },
    };

    let url = format!("https://api.github.com/repos/{target_repo}/dispatches");
    let client = reqwest::Client::builder()
        .user_agent("mntpack-sync-dispatch")
        .build()
        .context("failed to build dispatch http client")?;
    let response = client
        .post(&url)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .with_context(|| format!("failed to dispatch sync event to {target_repo}"))?;

    if response.status() == StatusCode::NO_CONTENT {
        return Ok(());
    }

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    eprintln!(
        "warning: sync dispatch failed for {target_repo}: status {} {}",
        status.as_u16(),
        body
    );
    Ok(())
}
