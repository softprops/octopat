use clipboard::{ClipboardContext, ClipboardProvider};
use colored::Colorize;
use dialoguer::{
    theme::{ColorfulTheme, Theme},
    Input, MultiSelect, Password,
};
use keyring::Keyring;
use reqwest::{header::HeaderMap, Client};
use serde::{Deserialize, Serialize};
use std::{error, fmt, string};
use structopt::StructOpt;
use tokio::sync::broadcast;

#[derive(StructOpt)]
/// An interactive GitHub personal access token command line dispenser ✨
pub enum Opts {
    /// Create a new token
    Create,
    /// Lists current tokens
    List,
    /// Delete tokens
    Delete,
}

use enum_iterator::IntoEnumIterator;

#[derive(Clone, Deserialize, Serialize, Copy, IntoEnumIterator)]
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

impl fmt::Debug for Scope {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(serde_json::to_string(&self).unwrap().as_str())
    }
}

impl fmt::Display for Scope {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(serde_json::to_string(&self).unwrap().replace("\"","").as_str())
    }
}

#[derive(Serialize)]
struct AuthRequest {
    note: String,
    scopes: Option<Vec<Scope>>,
}

#[derive(Deserialize)]
struct AuthResponse {
    token: String,
}

#[derive(Deserialize)]
struct AccessTokenResponse {
    access_token: String,
}

#[derive(Deserialize, Clone)]
struct Authorization {
    id: usize,
    scopes: Vec<Scope>,
    note: Option<String>,
    token_last_eight: Option<String>,
}

impl string::ToString for Authorization {
    fn to_string(&self) -> String {
        format!(
            "{} {:?}",
            self.note.clone().unwrap_or("???".into()),
            self.scopes
        )
    }
}

#[derive(Debug)]
struct StrErr(String);
impl error::Error for StrErr {}
impl fmt::Display for StrErr {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

#[derive(Debug)] // todo: custom masking debug impl
struct App {
    client_id: String,
    client_secret: String,
}

impl App {
    fn prompt(theme: &dyn Theme) -> Result<App, Box<dyn std::error::Error>> {
        let keyring = Keyring::new("octopat", "default");
        let app = match keyring.get_password() {
            Ok(value) => match &value.split(':').collect::<Vec<_>>()[..] {
                [client_id, client_secret] => App {
                    client_id: client_id.to_string(),
                    client_secret: client_secret.to_string(),
                },
                _ => panic!("didn't expect to find this in there"),
            },
            _ => {
                println!("We'll need some credentials from a GitHub app to fetch a new token. Visit https://github.com/settings/developers to find them");
                let client_id: String = Input::with_theme(theme)
                    .with_prompt("Your client id")
                    .interact()?;
                let client_secret: String = Password::with_theme(theme)
                    .with_prompt("Your client secret")
                    .interact()?;
                keyring.set_password(format!("{}:{}", client_id, client_secret).as_str())?;
                App {
                    client_id,
                    client_secret,
                }
            }
        };
        Ok(app)
    }
}

struct Credentials {
    login: String,
    password: String,
    otp: Option<String>,
}

impl Credentials {
    fn prompt(theme: &dyn Theme) -> std::io::Result<Self> {
        let login: String = Input::with_theme(theme)
            .with_prompt("Your GitHub login")
            .interact()?;
        let password: String = Password::with_theme(theme)
            .with_prompt("Your GitHub password")
            .interact()?;
        let otp: Option<String> = match Input::with_theme(theme)
            .with_prompt("Your GitHub OTP (optional)")
            .default("-".to_string())
            .show_default(false)
            .interact()?
            .as_str()
        {
            "-" => None,
            other => Some(other.to_string()),
        };
        Ok(Credentials {
            login,
            password,
            otp,
        })
    }
}

fn request(
    method: reqwest::Method,
    url: &str,
    credentials: Credentials,
) -> Result<reqwest::RequestBuilder, Box<dyn error::Error>> {
    let Credentials {
        login,
        password,
        otp,
    } = credentials;
    let mut headers = HeaderMap::new();
    headers.append(
        "User-Agent",
        format!("octopat/{}", env!("CARGO_PKG_VERSION")).parse()?,
    );
    headers.append("Content-Type", "application/json".parse()?);
    if let Some(otp) = otp {
        headers.append("X-GitHub-OTP", otp.parse()?);
    }
    Ok(Client::new()
        .request(method, url)
        .headers(headers)
        .basic_auth(login, Some(password)))
}

// trait QueryParams {
//     fn query_params(&self) -> HashMap<String, String>;
// }

// impl <T> QueryParams for hyper::request::Request<T> {
//     fn query_params(&self) -> HashMap<String, String> {
//         req.uri()
//             .query()
//             .map(|v| {
//                 url::form_urlencoded::parse(v.as_bytes())
//                     .into_owned()
//                     .collect()
//             })
//             .unwrap_or_else(std::collections::HashMap::new)
//     }
// }

async fn create(theme: &dyn Theme) -> Result<(), Box<dyn error::Error>> {
    let App {
        client_id,
        client_secret,
    } = App::prompt(theme)?;
    let selections = Scope::into_enum_iter().collect::<Vec<_>>();
    let defaults = &[true, false]; // select common case (repo) scope by default
    let scopes = MultiSelect::with_theme(theme)
        .with_prompt("Select Permission scopes")
        .items(&selections[..])
        .defaults(defaults)
        .interact()?
        .into_iter()
        .fold(Vec::new(), |mut res, index| {
            res.push(selections[index]);
            res
        });
    println!("🧭 Navigating to GitHub for authorization");
    opener::open(format!("https://github.com/login/oauth/authorize?client_id={}&redirect_uri=http://localhost:4567/&scope={}", client_id.clone(), scopes.into_iter().map(|s| s.to_string()).collect::<Vec<_>>().join("%20")))?;

    let (tx, mut rx) = broadcast::channel(1);
    // spin up a tiny http service to handle local redirection
    // of oauth access tokens
    let server = hyper::Server::bind(&([127, 0, 0, 1], 4567).into()).serve(
        hyper::service::make_service_fn(move |_| {
            let client_id = client_id.clone();
            let client_secret = client_secret.clone();
            let tx = tx.clone();
            async {
                Ok::<_, hyper::Error>(hyper::service::service_fn(move |req| {
                    let client_id = client_id.clone();
                    let client_secret = client_secret.clone();
                    let tx = tx.clone();
                    async move {
                        match req.uri().path() {
                            // because browsers always request this
                            "/favicon.ico" => Ok::<_, hyper::Error>(hyper::Response::default()),
                            _ => {
                                println!("👍 Received response. You can close the browser tab now");
                                let params = req
                                    .uri()
                                    .query()
                                    .map(|v| {
                                        url::form_urlencoded::parse(v.as_bytes())
                                            .into_owned()
                                            .collect()
                                    })
                                    .unwrap_or_else(std::collections::HashMap::new);

                                let AccessTokenResponse { access_token } = Client::new()
                                    .post("https://github.com/login/oauth/access_token")
                                    .header("Accept", "application/json")
                                    .form(&[
                                        ("client_id", client_id.clone()),
                                        ("client_secret", client_secret.clone()),
                                        ("code", params.get("code").unwrap().to_string()),
                                    ])
                                    .send()
                                    .await
                                    .unwrap()
                                    .json()
                                    .await
                                    .unwrap();
                                let mut clip: ClipboardContext = ClipboardProvider::new().unwrap();
                                clip.set_contents(access_token).unwrap();

                                println!("✨{}", "Token copied to clipboard".bold());
                                tx.send(()).unwrap();
                                Ok::<_, hyper::Error>(
                                    hyper::Response::builder()
                                        .status(hyper::StatusCode::OK)
                                        .body(hyper::Body::from(
                                            "Octopat says can close this browser tab",
                                        ))
                                        .unwrap(),
                                )
                            }
                        }
                    }
                }))
            }
        }),
    );

    tokio::select! {
        _ = rx.recv() => {
        }
        _ = server => {
        }
    }

    Ok(())
}

async fn list(theme: &dyn Theme) -> Result<(), Box<dyn error::Error>> {
    let credentials = Credentials::prompt(theme)?;
    let res = request(
        reqwest::Method::GET,
        "https://api.github.com/authorizations",
        credentials,
    )?
    .send()
    .await?;
    if !res.status().is_success() {
        Err(StrErr(res.text().await?))?;
    } else {
        let authorizations: Vec<Authorization> = res.json().await?;
        for auth in authorizations {
            println!("{}", auth.to_string());
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let theme = ColorfulTheme::default();
    match Opts::from_args() {
        Opts::Create => create(&theme).await?,
        Opts::List => list(&theme).await?,
        _ => (),
    }

    Ok(())
}
