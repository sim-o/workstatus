extern crate percent_encoding;
extern crate reqwest as r;
extern crate serde;

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use r::Error;


#[derive(Deserialize, Debug)]
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

pub struct Gitlab<'a> {
    client: reqwest::Client,
    host: &'a str,
    token: &'a str,
}

impl<'a> Gitlab<'a> {
    fn new(host: &'a str, token: &'a str) -> Gitlab<'a> {
        Gitlab {
            client: reqwest::Client::new(),
            host,
            token,
        }
    }

    fn get<T: DeserializeOwned>(&self, path: String) -> Result<T, Error> {
        self.client.get(format!("{:}{:}", self.host, path).as_str())
            .header("Private-Token", self.token)
            .send()?
            .json()
    }
}

static IGNORE_AUTHORS: &[&str] = &[];

pub fn merge_request_count() -> Result<usize, Error> {
    let client = Gitlab::new("", "");

    let project_name = utf8_percent_encode(, &NON_ALPHANUMERIC).to_string();
    let project: Project = client.get(format!("/api/v4/projects/{:}", project_name))?;

    let merge_requests: Vec<MergeRequest> = client.get(format!("/api/v4/projects/{:}/merge_requests?state=opened&per_page=100", project.id))?;

    let approvals = merge_requests.iter()
        .filter(|mr| !IGNORE_AUTHORS.contains(&&*mr.author.username))
        .map(|mr| client.get::<MergeRequestApproval>(format!("/api/v4/projects/{:}/merge_requests/{:}/approvals", project.id, mr.iid)))
        .filter_map(|mra| mra.ok())
        .filter(|mra| mra.approvals_left > 0 && mra.user_can_approve && !mra.user_has_approved)
        .count();

    Ok(approvals)
}