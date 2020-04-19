use chrono::Utc;
use crate::storage::{self, StorageDriver};
use futures::executor::block_on;
use hyper::{Body, Client, Method, Request, StatusCode};
use hyper::body;
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::task;
use uuid::Uuid;

pub(crate) struct Firestore<T> {
    client: Arc<Client<HttpsConnector<HttpConnector>>>,
    _oauth_token: String,
    _item_type: std::marker::PhantomData<T>,
}

impl<T> Firestore<T> {
    pub async fn new<P: Into<PathBuf>>(json_key_path: P) -> storage::Result<Firestore<T>> {
        // Create shared HTTP client
        let mut https = HttpsConnector::new();
        https.https_only(true);
        let client = Client::builder().build::<_, hyper::Body>(https);

        // Get OAuth token
        let json_key = read_json_key(json_key_path).await?;
        let jwt = build_jwt(&json_key.client_email, &json_key.private_key).await?;
        let oauth_token = get_oauth_token(jwt, &client).await?;
        info!("Got oauth token: {}", oauth_token);

        Ok(Firestore {
            client: Arc::new(client),
            _oauth_token: oauth_token,
            _item_type: std::marker::PhantomData,
        })
    }
}

impl<T: DeserializeOwned + Serialize + Send + Sync> StorageDriver for Firestore<T> {
    type Item = T;

    fn list_ids(&self) -> storage::Result<Vec<Uuid>> {
        block_on(async {
            unimplemented!()
        })
    }

    fn read(&self, id: &Uuid) -> storage::Result<Option<T>> {
        block_on(async {
            unimplemented!()
        })
    }

    fn read_all(&self) -> storage::Result<Vec<T>> {
        block_on(async {
            unimplemented!()
        })
    }

    fn write(&self, id: &Uuid, value: &T) -> storage::Result<()> {
        block_on(async {
            unimplemented!()
        })
    }
}

#[derive(Debug, Deserialize)]
struct JsonKey {
    r#type: String,
    project_id: String,
    private_key_id: String,
    private_key: String,
    client_email: String,
    auth_uri: String,
    token_uri: String,
    auth_provider_x509_cert_url: String,
    client_x509_cert_url: String,
}

async fn read_json_key<P: Into<PathBuf>>(json_key_path: P) -> storage::Result<JsonKey> {
    let contents = fs::read_to_string(json_key_path.into()).await?;
    Ok(serde_json::from_str(&contents)?)
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,    // Email address of the service account
    scope: String,  // Space-delimited list of the permissions requested
    aud: String,    // Inteneded target of assertion, should just be https://oauth2.googleapis.com/token
    exp: usize,     // Expiration time of the assertion, as seconds since epoch. Maximum of 1 hour after issuance.
    iat: usize,     // Assertion issuance time, as seconds since epoch.
}

async fn build_jwt(email: &str, private_key: &str) -> storage::Result<String> {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        iss: email.to_string(),
        scope: "https://www.googleapis.com/auth/datastore".to_owned(),
        aud: "https://oauth2.googleapis.com/token".to_owned(),
        exp: now + 3600,
        iat: now,
    };

    let token = encode(&Header::new(Algorithm::RS256), &claims, &EncodingKey::from_rsa_pem(private_key.as_ref())?)?;
    Ok(token)
}

#[derive(Debug, Serialize, Deserialize)]
struct OAuth2Request {
    grant_type: String,
    assertion: String,
}

#[derive(Debug, Deserialize)]
struct OAuth2Response {
    access_token: String,
    expires_in: usize,
    token_type: String,
}

async fn get_oauth_token(jwt: String, http_client: &Client<HttpsConnector<HttpConnector>>) -> storage::Result<String> {
    let request_body = OAuth2Request {
        grant_type: "urn:ietf:params:oauth:grant-type:jwt-bearer".to_owned(),
        assertion: jwt,
    };
    let request = Request::builder()
        .method(Method::POST)
        .uri("https://oauth2.googleapis.com/token")
        .body(Body::from(serde_json::to_string_pretty(&request_body)?))
        .unwrap();
    debug!("Request: {:?}", request);
    let response = http_client.request(request).await.unwrap();
    let status = response.status();
    let body_bytes = body::to_bytes(response.into_body()).await.unwrap();
    match status {
        StatusCode::OK => {
            let body: OAuth2Response = serde_json::from_slice(&body_bytes)?;
            debug!("Response: {} {:?}", status, body);
            Ok(body.access_token)
        },
        _ => {
            let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
            error!("Response: {} {}", status, body_str);
            Err(storage::Error::OAuth(format!("OAuth flow returned HTTP {} with body content: {}", status, body_str)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static JSON_KEY_PATH: &str = "secrets/service_account.json";

    #[tokio::test(threaded_scheduler)]
    async fn can_read_key_from_json() {
        tokio::spawn(async {
            read_json_key(JSON_KEY_PATH).await.unwrap();
        }).await.unwrap();
    }

    #[tokio::test(threaded_scheduler)]
    async fn test_build_jwt() {
        tokio::spawn(async {
            let key = read_json_key(JSON_KEY_PATH).await.unwrap();
            build_jwt(&key.client_email, &key.private_key).await.unwrap();
        }).await.unwrap();
    }

    #[tokio::test(threaded_scheduler)]
    async fn test_get_oauth_token() {
        tokio::spawn(async {
            let key = read_json_key(JSON_KEY_PATH).await.unwrap();
            let jwt = build_jwt(&key.client_email, &key.private_key).await.unwrap();

            let mut https = HttpsConnector::new();
            https.https_only(true);
            let client = Client::builder().build::<_, hyper::Body>(https);

            get_oauth_token(jwt, &client).await.unwrap();
        }).await.unwrap();
    }
}
