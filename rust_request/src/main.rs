use std::env;

use anyhow::Result;

use crate::db_manager::DBManager;

mod db_manager;
mod request;
mod service;
const PORT: &str = "3000";

#[tokio::main]
async fn main() -> Result<()> {
    println!("healthy launch");
    #[cfg(feature = "browser-init")]
    {
        browser_init::integrate().await;
        return Ok(());
    }

    let code = env::var("LAUNCH_PARAM_USER_CODE").expect("invalid init code from lambda");
    let time = env::var("LAUNCH_PARAM_TIME").expect("invalid init time from lambda");
    println!("code received :{code}");
    let db = DBManager::init(&code, time).await;

    let res = service::new_releases_list(3, code, &db).await?;

    println!("result sending :{}", res);
    db.send_result(res).await?;
    println!("result sent");

    Ok(())
}
#[cfg(feature = "browser-init")]
mod browser_init {

    use axum::{extract::Query, response::Redirect, routing::get, serve};
    use axum_extra::extract::CookieJar;
    use cookie::Cookie;
    use random_string::generate;
    use serde::Deserialize;
    use url::Url;

    use crate::{
        PORT,
        db_manager::SendProgress,
        service::{self, CHARSET_STATE, CLIENT_DATA},
    };

    #[derive(Deserialize, Debug)]
    struct CallbackResponse {
        state: String,
        code: Option<String>,
        error: Option<String>,
    }

    pub async fn integrate() {
        let addr_port = format!("0.0.0.0:{}", PORT);
        let full_link = format!("http://{addr_port}/login");
        let app = axum::Router::new()
            .route("/login", get(login))
            .route("/logincallback", get(after_login));
        let listener = tokio::net::TcpListener::bind(addr_port).await.unwrap();
        let res = open::that(full_link.clone());
        if res.is_err() {
            println!("please visit: {full_link}");
        }
        serve(listener, app).await.unwrap();
    }
    async fn login() -> (CookieJar, Redirect) {
        let state = generate(16, CHARSET_STATE);
        let mut url = Url::parse("https://accounts.spotify.com/authorize").unwrap();
        let redir_uri = format!("http://127.0.0.1:{}/logincallback", PORT);
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &CLIENT_DATA.client_id)
            .append_pair("scope", "user-library-read")
            .append_pair("redirect_uri", &redir_uri)
            .append_pair("state", &state);
        let cookie = Cookie::build(("spotify_auth_state", state));
        let jar = CookieJar::new().add(cookie);
        println!("login in, at {}\n", url.as_str());
        (jar, Redirect::to(url.as_str()))
    }

    async fn after_login(
        jar: CookieJar,
        Query(params): Query<CallbackResponse>,
    ) -> Result<String, String> {
        let state = jar.get("spotify_auth_state").map(|c| c.value().to_string());
        if state.is_none() {
            return Err("No state in cookie".to_string());
        }
        let state = state.unwrap();
        if state != params.state {
            return Err("state mismatch, possible CSRF attack".to_string());
        }
        println!("state is {:?}\n", state);
        println!("{:?}\n", params);
        if let Some(err) = params.error {
            println!("err: {}", err);
            return Err(err);
        }
        let code = params.code.unwrap();

        let res = service::new_releases_list(3, code, &TestDB {})
            .await
            .unwrap();
        println!("{}", res);
        Ok(res)
    }

    pub struct TestDB;

    impl SendProgress for TestDB {
        async fn send(&self, progress: i32) {
            println!("progress :{progress}");
        }
    }
}
