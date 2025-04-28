use crate::config::CONFIG;
use anyhow::{Context, Result};
use chrono::{Duration, NaiveDate};
use reqwest_dav::{Auth, ClientBuilder, Depth, list_cmd::ListEntity};
use url_encor::Encoder;

#[derive(Debug)]
pub struct Member {
    pub email: String,
    pub added_at: NaiveDate,
}

#[derive(Debug)]
pub struct Template {
    pub body: String,
    pub name: String,
    pub subject: String,
    pub duration: Duration,
}

#[derive(Debug)]
pub struct Data {
    pub members: Vec<Member>,
    pub templates: Vec<Template>,
}

pub async fn get() -> Result<Data> {
    let client = ClientBuilder::new()
        .set_host(format!(
            "https://{}/public.php/dav/files/{}/",
            CONFIG.server, CONFIG.username
        ))
        .set_auth(Auth::Basic(CONFIG.username.clone(), "".to_owned()))
        .build()?;

    let list = client
        .get("liste.txt")
        .await
        .context("Could not get liste.txt")?
        .text()
        .await
        .context("Could not get body from liste.txt")?;
    let members: Vec<Member> = list
        .lines()
        .filter_map(|line| {
            let mut parts = line.split(';');
            let Some(email) = parts.next() else {
                log::error!("Could not get email from line: {}", line);
                return None;
            };
            let email = email.trim().to_owned();

            let Some(added_at) = parts.next() else {
                log::error!("Could not get added_at from line: {}", line);
                return None;
            };
            let added_at = match added_at.parse::<NaiveDate>() {
                Ok(added_at) => added_at,
                Err(err) => {
                    log::error!("Could not parse added_at \"{added_at}\": {err:?}");
                    return None;
                }
            };

            Some(Member { email, added_at })
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

    Ok(Data { members, templates })
}
