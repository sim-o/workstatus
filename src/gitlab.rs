extern crate percent_encoding;
extern crate reqwest as r;
extern crate serde;

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use r::Error;
use crate::gitlab::PipelineStatus::{Success, Skipped};


#[derive(Deserialize, Debug, Copy, Clone)]
struct Project {
    id: u32,
}

#[derive(Deserialize, Debug)]
struct MergeRequest {
    id: u32,
    iid: u32,
    author: MergeRequestAuthor,
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
    Running, Pending, Success, Failed, Canceled, Skipped, Manual,
}

#[derive(Deserialize, Debug)]
struct Pipeline {
    id: i32,
    status: PipelineStatus,
    #[serde(rename = "ref")]
    ref_name: String,
}

#[derive(Deserialize, Debug)]
struct PipelineDetail {
    id: i32,
    before_sha: String,
}

pub struct Gitlab<'a> {
    client: reqwest::Client,
    host: &'a str,
    token: &'a str,
    project_name: &'a str,
    project: Option<Project>,
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
                let project_name = utf8_percent_encode(, &NON_ALPHANUMERIC).to_string();
                let project: Project = self.get(&format!("/api/v4/projects/{:}", project_name))?;
                let project_id = project.id;
                self.project = Some(project);
                project_id
            },
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

        let status = pipelines.get(0)
            // exclude scheduled jobs
            .filter(|p| {
                let url = format!("/api/v4/projects/{:}/pipelines/{:}", project_id, p.id);
                let det: Option<PipelineDetail> = self.get(&url).ok();
                det.map(|d| !d.before_sha.trim_matches('0').is_empty())
                    .unwrap_or(false)
            })
            .map(|p| p.status)
            .unwrap_or_else(|| {
                println!("no details found");
                Skipped
            });

        Ok(status)
    }
}
