//! GitHub personal access token dispenser
use clipboard::{ClipboardContext, ClipboardProvider};
use colored::Colorize;
use dialoguer::{
    theme::{ColorfulTheme, Theme},
    Input, MultiSelect, Password,
};
use enum_iterator::IntoEnumIterator;
use hyper::service::{make_service_fn, service_fn};
use keyring::Keyring;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt};
use structopt::StructOpt;
use tokio::sync::broadcast;

/// An interactive GitHub personal access token command line dispenser ✨
#[derive(StructOpt)]
pub struct Opts {
    /// An optional port to listen for responses from GitHub on (defaults to 4567)
    #[structopt(long, short)]
    port: Option<u16>,
    /// Alias for name of GitHub app to store on keychain (defaults to "default")
    #[structopt(long, short)]
    alias: Option<String>,
}

#[derive(Clone, Deserialize, Serialize, Copy, IntoEnumIterator, PartialEq)]
pub enum Scope {
    #[serde(rename = "repo")]
    Repo,
    #[serde(rename = "repo:status")]
    RepoStatus,
    #[serde(rename = "repo_deployment")]
    RepoDeployment,
    #[serde(rename = "public_repo")]
    PublicRepo,
    #[serde(rename = "repo:invite")]
    RepoInvite,
    #[serde(rename = "security_events")]
    SecurityEvents,
    #[serde(rename = "admin:repo_hook")]
    AdminRepoHook,
    #[serde(rename = "write:repo_hook")]
    WriteRepoHook,
    #[serde(rename = "read:repo_hook")]
    ReadRepoHook,
    #[serde(rename = "admin:org")]
    AdminOrg,
    #[serde(rename = "write:org")]
    WriteOrg,
    #[serde(rename = "read:org")]
    ReadOrg,
    #[serde(rename = "admin:public_key")]
    AdminPublicKey,
    #[serde(rename = "write:public_key")]
    WritePublicKey,
    #[serde(rename = "read:public_key")]
    ReadPublicKey,
    #[serde(rename = "admin:org_hook")]
    AdminOrgHook,
    #[serde(rename = "gist")]
    Gist,
    #[serde(rename = "notifications")]
    Notifications,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "user:email")]
    UserEmail,
    #[serde(rename = "read:user")]
    ReadUser,
    #[serde(rename = "user:follow")]
    UserFollow,
    #[serde(rename = "delete_repo")]
    DeleteRepo,
    #[serde(rename = "write:discussion")]
    WriteDiscussion,
    #[serde(rename = "read:discussion")]
    ReadDiscussion,
    #[serde(rename = "write:packages")]
    WritePackages,
    #[serde(rename = "read:packages")]
    ReadPackages,
    #[serde(rename = "delete:packages")]
    DeletePackages,
    #[serde(rename = "admin:gpg_key")]
    AdminGpgKey,
    #[serde(rename = "write:gpg_key")]
    WriteGpgKey,
    #[serde(rename = "read:gpg_key")]
    ReadGpgKey,
    #[serde(rename = "workflow")]
    Workflow,
}

impl fmt::Display for Scope {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(
            serde_json::to_string(&self)
                .expect("failed to serialize scope to a string")
                .replace("\"", "")
                .as_str(),
        )
    }
}

#[derive(Deserialize)]
struct AccessTokenResponse {
    access_token: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct App {
    client_id: String,
    client_secret: String,
}

impl App {
    fn prompt(
        theme: &dyn Theme,
        alias: impl AsRef<str>,
    ) -> Result<App, anyhow::Error> {
        let keyring = Keyring::new("octopat", alias.as_ref());
        let app = match keyring
            .get_password()
            .map_err(|e| anyhow::anyhow!(e.to_string()))
            .and_then(|value| serde_json::from_str(&value).map_err(anyhow::Error::from))
        {
            Ok(app) => app,
            _ => {
                println!("We'll need some credentials from a GitHub app to fetch a new token");
                println!("Visit https://github.com/settings/developers to find them or create a new application");
                let client_id: String = Input::with_theme(theme)
                    .with_prompt("Your client id")
                    .interact()?;
                let client_secret: String = Password::with_theme(theme)
                    .with_prompt("Your client secret")
                    .interact()?;
                let app = App {
                    client_id,
                    client_secret,
                };
                keyring
                    .set_password(&serde_json::to_string(&app)?)
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                app
            }
        };
        Ok(app)
    }
}

fn authorization_url(
    client_id: impl AsRef<str>,
    scopes: Vec<Scope>,
    port: u16,
) -> String {
    format!(
        "https://github.com/login/oauth/authorize?client_id={client_id}&redirect_uri=http://localhost:{port}/&scope={scope}",
        client_id = client_id.as_ref(),
        scope = scopes.into_iter().map(|s| s.to_string()).collect::<Vec<_>>().join("%20"),
        port = port
    )
}

async fn exchange_token(
    app: &App,
    code: impl AsRef<str>,
) -> Result<AccessTokenResponse, reqwest::Error> {
    let App {
        client_id,
        client_secret,
    } = app;
    Ok(Client::new()
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id.as_ref()),
            ("client_secret", client_secret.as_ref()),
            ("code", code.as_ref()),
        ])
        .send()
        .await?
        .json()
        .await?)
}

trait Params {
    fn query_params(&self) -> HashMap<String, String>;
    fn query_param(
        &self,
        name: &str,
    ) -> Option<String> {
        self.query_params().get(name).map(String::clone)
    }
}

impl<T> Params for hyper::Request<T> {
    fn query_params(&self) -> HashMap<String, String> {
        self.uri()
            .query()
            .map(|v| {
                url::form_urlencoded::parse(v.as_bytes())
                    .into_owned()
                    .collect()
            })
            .unwrap_or_else(HashMap::new)
    }
}

async fn create(
    port: u16,
    alias: String,
    theme: &dyn Theme,
) -> Result<(), anyhow::Error> {
    let app = App::prompt(theme, alias)?;
    let selections = Scope::into_enum_iter().collect::<Vec<_>>();
    let scopes = MultiSelect::with_theme(theme)
        .with_prompt("Select Permission scopes")
        .items(&selections[..])
        .defaults(
            &selections
                .iter()
                .map(|scope| *scope == Scope::Repo)
                .collect::<Vec<_>>(),
        )
        .paged(true)
        .interact()?
        .into_iter()
        .fold(Vec::new(), |mut res, index| {
            res.push(selections[index]);
            res
        });
    println!("🧭 Navigating to GitHub for authorization");
    opener::open(authorization_url(app.client_id.as_str(), scopes, port))?;

    let (tx, mut rx) = broadcast::channel(1);
    // spin up a tiny http service to handle local redirection
    // of oauth access tokens
    let server =
        hyper::Server::bind(&([127, 0, 0, 1], port).into()).serve(make_service_fn(move |_| {
            let app = app.clone();
            let tx = tx.clone();
            async {
                Ok::<_, anyhow::Error>(service_fn(move |req| {
                    let app = app.clone();
                    let tx = tx.clone();
                    async move {
                        match req.uri().path() {
                            // because browsers always request this
                            "/favicon.ico" => Ok::<_, anyhow::Error>(hyper::Response::default()),
                            _ => {
                                println!("👍 Received response. You can close the browser tab now");
                                match req.query_param("code") {
                                    Some(code) => {
                                        let AccessTokenResponse { access_token } =
                                            exchange_token(&app, code).await?;
                                        let mut clip: ClipboardContext = ClipboardProvider::new()
                                            .expect("failed to get access to clipboard");
                                        clip.set_contents(access_token)
                                            .expect("failed to set clipboard contents");

                                        println!("✨{}", "Token copied to clipboard".bold());
                                        tx.send(()).unwrap(); // tokio error doesn't impl std error?
                                        Ok::<_, anyhow::Error>(
                                            hyper::Response::builder()
                                                .header("Content-Type", "text/html")
                                                .body(hyper::Body::from(
                                                    include_str!("../pages/success.html")
                                                        .replace("{client_id}", &app.client_id),
                                                ))?,
                                        )
                                    }
                                    _ => {
                                        tx.send(()).unwrap();
                                        Ok::<_, anyhow::Error>(
                                            // tokio error doesn't impl std error?
                                            hyper::Response::builder()
                                                .header("Content-Type", "text/html")
                                                .body(hyper::Body::from(include_str!(
                                                    "../pages/error.html"
                                                )))?,
                                        )
                                    }
                                }
                            }
                        }
                    }
                }))
            }
        }));

    tokio::select! {
        _ = rx.recv() => {
        }
        _ = server => {
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let Opts { port, alias } = Opts::from_args();
    create(
        port.unwrap_or(4567),
        alias.unwrap_or_else(|| "default".into()),
        &ColorfulTheme::default(),
    )
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_url_returns_expected_url() {
        assert_eq!(
            authorization_url("client_id", vec![Scope::AdminOrg, Scope::AdminRepoHook], 4567),
            "https://github.com/login/oauth/authorize?client_id=client_id&redirect_uri=http://localhost:4567/&scope=admin:org%20admin:repo_hook"
        )
    }

    #[test]
    fn scope_deserializes_into_identifier() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            serde_json::to_string(&Scope::AdminGpgKey)?,
            "\"admin:gpg_key\""
        );
        Ok(())
    }

    #[test]
    fn request_query_params() -> Result<(), Box<dyn std::error::Error>> {
        let mut expect = HashMap::new();
        expect.insert("baz".to_string(), "boom".to_string());
        assert_eq!(
            hyper::Request::builder()
                .uri("https://foo.bar?baz=boom")
                .body(())?
                .query_params(),
            expect
        );
        Ok(())
    }

    #[test]
    fn request_query_param() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            hyper::Request::builder()
                .uri("https://foo.bar?baz=boom")
                .body(())?
                .query_param("baz"),
            Some("boom".into())
        );
        Ok(())
    }
}
