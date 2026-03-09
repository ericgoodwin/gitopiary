use std::path::PathBuf;
use serde::Deserialize;
use tokio::process::Command;
use anyhow::Result;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GhPullRequest {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub is_draft: bool,
    pub head_ref_name: String,
    pub url: String,
}

#[derive(Debug)]
pub struct PrInfo {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub is_draft: bool,
    pub head_ref: String,
    pub url: String,
}

pub async fn fetch_prs(repo_path: &PathBuf) -> Result<Vec<PrInfo>> {
    let output = Command::new("gh")
        .arg("pr")
        .arg("list")
        .arg("--json")
        .arg("number,title,state,isDraft,headRefName,url")
        .arg("--limit")
        .arg("50")
        .current_dir(repo_path)
        .output()
        .await;

    let output = match output {
        Ok(o) => o,
        Err(_) => return Ok(vec![]),
    };

    if !output.status.success() {
        return Ok(vec![]);
    }

    let prs: Vec<GhPullRequest> = match serde_json::from_slice(&output.stdout) {
        Ok(p) => p,
        Err(_) => return Ok(vec![]),
    };

    Ok(prs
        .into_iter()
        .map(|p| PrInfo {
            number: p.number,
            title: p.title,
            state: p.state,
            is_draft: p.is_draft,
            head_ref: p.head_ref_name,
            url: p.url,
        })
        .collect())
}
