extern crate percent_encoding;
extern crate reqwest as r;
extern crate serde;

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use r::Error;
use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::gitlab::PipelineStatus::Skipped;
use std::collections::HashSet;
use std::hash::Hash;

#[derive(Deserialize, Debug, Copy, Clone)]
struct Project {
    id: u32,
}

#[derive(Deserialize, Debug)]
struct MergeRequest {
    id: u32,
    iid: u32,
    author: MergeRequestAuthor,
    source_branch: String,
    sha: String,
}

#[derive(Deserialize, Debug)]
struct MergeRequestAuthor {
    id: u32,
    username: String,
}

#[derive(Deserialize, Debug)]
struct MergeRequestApproval {
    approvals_left: u32,
    user_can_approve: bool,
    user_has_approved: bool,
}

#[derive(Deserialize, Debug, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum PipelineStatus {
    Running,
    Pending,
    Success,
    Failed,
    Canceled,
    Skipped,
    Manual,
}

#[derive(Deserialize, Debug)]
struct Pipeline {
    id: i32,
    status: PipelineStatus,
    #[serde(rename = "ref")]
    ref_name: String,
    sha: String,
}

#[derive(Deserialize, Debug)]
struct PipelineDetail {
    id: i32,
    before_sha: String,
}

#[derive(Deserialize, Debug)]
struct Commit {
    id: String,
    message: String,
    author_name: String,
    author_email: String,
    committer_name: String,
    committer_email: String,
}

#[derive(Deserialize, Debug)]
struct Branch {
    name: String,
    merged: bool,
    commit: Commit,
}

pub struct Gitlab<'a> {
    client: reqwest::Client,
    host: &'a str,
    token: &'a str,
    project_name: &'a str,
    project: Option<Project>,
}

pub struct MergeRequestStatus {
    pub branch: String,
    pub status: PipelineStatus,
}

impl<'a> Gitlab<'a> {
    pub fn new(host: &'a str, token: &'a str, project_name: &'a str) -> Gitlab<'a> {
        Gitlab {
            client: reqwest::Client::new(),
            host,
            token,
            project_name,
            project: None,
        }
    }

    fn get<T: DeserializeOwned>(&self, path: &String) -> Result<T, Error> {
        self.client.get(format!("{:}{:}", self.host, path).as_str())
            .header("Private-Token", self.token)
            .send()?
            .json()
    }

    fn get_project_id(&mut self) -> Result<u32, Error> {
        let project_id = match self.project {
            Some(project) => project.id,
            None => {
                let project_name = utf8_percent_encode("PSX/psx", &NON_ALPHANUMERIC).to_string();
                let project: Project = self.get(&format!("/api/v4/projects/{:}", project_name))?;
                let project_id = project.id;
                self.project = Some(project);
                project_id
            }
        };
        Ok(project_id)
    }

    pub fn merge_request_count(&mut self, ignore_authors: &Vec<String>) -> Result<usize, Error> {
        let project_id = self.get_project_id()?;
        let merge_requests: Vec<MergeRequest> = self.get(&format!("/api/v4/projects/{:}/merge_requests?state=opened&per_page=100", project_id))?;

        let approvals = merge_requests.iter()
            .filter(|mr| !ignore_authors.contains(&mr.author.username))
            .map(|mr| self.get::<MergeRequestApproval>(&format!("/api/v4/projects/{:}/merge_requests/{:}/approvals", project_id, mr.iid)))
            .filter_map(|mra| mra.ok())
            .filter(|mra| mra.approvals_left > 0 && mra.user_can_approve && !mra.user_has_approved)
            .count();

        Ok(approvals)
    }

    pub fn pipeline_status(&mut self, ref_name: &str) -> Result<PipelineStatus, Error> {
        let project_id = self.get_project_id()?;

        let pipelines: Vec<Pipeline> = self.get(&format!(
            "/api/v4/projects/{:}/pipelines?ref={:}&per_page=100",
            project_id, ref_name))?;

        let status = pipelines.iter()
            // exclude scheduled jobs
            .filter(|p| {
                let url = format!("/api/v4/projects/{:}/pipelines/{:}", project_id, p.id);
                let det: Option<PipelineDetail> = self.get(&url).ok();
                det.map(|d| !d.before_sha.trim_matches('0').is_empty())
                    .unwrap_or(false)
            })
            .next()
            .map(|p| p.status)
            .unwrap_or_else(|| {
                println!("no details found");
                Skipped
            });

        Ok(status)
    }

    pub fn user_merge_requests(&mut self, usernames: &Vec<String>) -> Result<Vec<MergeRequestStatus>, Error> {
        let project_id = self.get_project_id()?;

        let merge_requests: Vec<MergeRequest> = self.get(&format!("/api/v4/projects/{:}/merge_requests?state=opened&per_page=100", project_id))?;
        let result = merge_requests.iter()
            .filter(|mr| {
                let branch: Result<Branch, _> = self.get(&format!("/api/v4/projects/{:}/repository/branches/{:}", project_id, mr.source_branch));
                match branch {
                    Ok(branch) => {
                        !branch.merged && branch.commit.id == mr.sha && usernames.iter().any(|u|
                            branch.commit.committer_name == *u ||
                                branch.commit.committer_email == *u ||
                                branch.commit.message.contains(u))
                    }
                    Err(e) => {
                        println!("error fetching branch {:?}", e);
                        false
                    }
                }
            })
            .filter(|mr| {
                let pipelines: Result<Vec<Pipeline>, _> = self.get(&format!("/api/v4/projects/{:}/pipelines?status=failed&sha={:}", project_id, mr.sha));
                match pipelines {
                    Ok(pipelines) => !pipelines.is_empty(),
                    Err(e) => {
                        println!("error fetching branch {:?}", e);
                        false
                    }
                }
            })
            .map(|mr| MergeRequestStatus {
                branch: mr.source_branch.clone(),
                status: PipelineStatus::Failed,
            })
            .collect();

        Ok(result)
    }
}
