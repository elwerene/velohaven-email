use anyhow::{Context, Result};
use chrono::{NaiveDate, TimeZone, Utc};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EmptyExtraTokenFields,
    RedirectUrl, StandardTokenResponse, TokenResponse, TokenUrl,
    basic::{BasicClient, BasicTokenType},
};
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct Cleverreach {
    client_id: String,
    client_secret: String,
    group_id: String,
}

#[derive(Debug)]
pub struct Member {
    pub email: String,
    pub added_at: NaiveDate,
}

impl Cleverreach {
    pub async fn get_members(&self) -> Result<Vec<Member>> {
        let token_json = tokio::fs::read_to_string("token.json").await.ok();
        let mut token_result = token_json.and_then(|json| {
            serde_json::from_str::<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>>(
                &json,
            )
            .ok()
        });

        let client = BasicClient::new(ClientId::new(self.client_id.clone()))
            .set_client_secret(ClientSecret::new(self.client_secret.clone()))
            .set_auth_uri(AuthUrl::new(
                "https://rest.cleverreach.com/oauth/authorize.php".to_string(),
            )?)
            .set_token_uri(TokenUrl::new(
                "https://rest.cleverreach.com/oauth/token.php".to_string(),
            )?)
            .set_redirect_uri(RedirectUrl::new("http://localhost".to_string())?);

        let http_client = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        if let Some(token) = &token_result {
            if let Some(expires_in) = token.expires_in() {
                if expires_in < Duration::from_secs(60 * 60 * 48) {
                    token_result = Some(
                        client
                            .exchange_refresh_token(
                                token
                                    .refresh_token()
                                    .ok_or_else(|| anyhow::format_err!("No refresh token found"))?,
                            )
                            .request_async(&http_client)
                            .await?,
                    );
                }
            }
        } else {
            let (auth_url, _csrf_token) = client.authorize_url(CsrfToken::new_random).url();

            eprintln!("Open in browser:\n\n{}\n\n", auth_url);
            eprintln!("Please enter the auth_code: ");
            let auth_code = &mut String::new();
            std::io::stdin()
                .read_line(auth_code)
                .expect("Failed to read line");

            token_result = Some(
                client
                    .exchange_code(AuthorizationCode::new(auth_code.trim().to_string()))
                    .request_async(&http_client)
                    .await?,
            );
        }

        let Some(token) = token_result else {
            anyhow::bail!("No token found");
        };

        let token_json = serde_json::to_string(&token).context("Could not serialize token")?;
        tokio::fs::write("token.json", token_json)
            .await
            .context("Could not write token to file")?;

        #[derive(Deserialize)]
        struct CleverreachReceiver {
            email: String,
            activated: i64,
            deactivated: i64,
        }

        let receivers = http_client
            .get(format!(
                "https://rest.cleverreach.com/v3/groups/{}/receivers?pagesize=5000",
                self.group_id
            ))
            .bearer_auth(token.access_token().secret())
            .send()
            .await?
            .error_for_status()?
            .json::<Vec<CleverreachReceiver>>()
            .await?;

        log::info!(
            "Received {} receivers from cleverreach ({} are deactivated)",
            receivers.len(),
            receivers.iter().filter(|r| r.deactivated != 0).count()
        );

        let mut skip_until_broken_email = false;
        let members = receivers
            .into_iter()
            .filter(|receiver| {
                if receiver.email == "rene@reshx.de" {
                    skip_until_broken_email = true;
                    true
                } else {
                    skip_until_broken_email
                }
            })
            .filter(|receiver| receiver.deactivated == 0)
            .filter_map(|receiver| {
                let added_at = match Utc.timestamp_opt(receiver.activated, 0) {
                    chrono::LocalResult::None => {
                        log::error!("Could not parse date time \"{}\"", receiver.activated);
                        return None;
                    }
                    chrono::LocalResult::Single(date_time)
                    | chrono::LocalResult::Ambiguous(date_time, _) => date_time.date_naive(),
                };

                Some(Member {
                    email: receiver.email.trim().to_lowercase(),
                    added_at,
                })
            })
            .collect::<Vec<_>>();

        Ok(members)
    }
}
