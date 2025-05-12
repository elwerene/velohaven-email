use anyhow::{Context, Result};
use chrono::Duration;
use reqwest_dav::{Auth, ClientBuilder, Depth, list_cmd::ListEntity};
use serde::Deserialize;
use std::collections::HashSet;
use url_encor::Encoder;

#[derive(Debug, Deserialize)]
pub struct NextcloudConfig {
    pub server: String,
    pub username: String,
}

#[derive(Debug)]
pub struct Template {
    pub body: String,
    pub name: String,
    pub subject: String,
    pub duration: Duration,
}

#[derive(Debug)]
pub struct NextcloudData {
    pub unsubscribed: HashSet<String>,
    pub templates: Vec<Template>,
}

impl NextcloudConfig {
    pub async fn get_data(&self) -> Result<NextcloudData> {
        let client = ClientBuilder::new()
            .set_host(format!(
                "https://{}/public.php/dav/files/{}/",
                self.server, self.username
            ))
            .set_auth(Auth::Basic(self.username.clone(), "".to_owned()))
            .build()?;

        let list = client
            .get("unsubscribed.txt")
            .await
            .context("Could not get unsubscribed.txt")?
            .text()
            .await
            .context("Could not get body from unsubscribed.txt")?;
        let unsubscribed: HashSet<String> = list
            .lines()
            .filter_map(|line| {
                let email = line.trim().to_lowercase();
                if email.is_empty() {
                    return None;
                }
                Some(email)
            })
            .collect();

        let entries = client.list("templates", Depth::Number(1)).await?;
        let mut templates = vec![];
        for entry in entries {
            if let ListEntity::File(file) = entry {
                let Some(name) = file
                    .href
                    .split('/')
                    .next_back()
                    .and_then(|name| name.strip_suffix(".html"))
                else {
                    log::error!("Could not get name from file: {:?}", file);
                    continue;
                };
                let name = name.url_decode();
                let mut parts = name.split('-');
                let Some(duration) = parts.next() else {
                    log::error!("Could not get duration from name {name}");
                    continue;
                };
                let duration = match parse_duration::parse(duration) {
                    Err(err) => {
                        log::error!("Could not parse duration from name {name}: {err:?}");
                        continue;
                    }
                    Ok(duration) => match Duration::from_std(duration) {
                        Ok(duration) => duration,
                        Err(err) => {
                            log::error!("Could not convert duration from name {name}: {err:?}");
                            continue;
                        }
                    },
                };
                let Some(subject) = parts.next() else {
                    log::error!("Could not get subject from name {name}");
                    continue;
                };
                let subject = subject.trim().to_string();
                let body = client
                    .get(&format!("templates/{name}.html"))
                    .await
                    .with_context(|| format!("Could not get file {name}.html"))?
                    .text()
                    .await
                    .with_context(|| format!("Could not get body from file {name}.html"))?;
                templates.push(Template {
                    body,
                    name,
                    subject,
                    duration,
                });
            }
        }

        Ok(NextcloudData {
            unsubscribed,
            templates,
        })
    }
}
