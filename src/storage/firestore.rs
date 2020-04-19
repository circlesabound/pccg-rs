use crate::storage;
use chrono::Utc;
use hyper::body;
use hyper::client::HttpConnector;
use hyper::{header::HeaderName, Body, Client, Method, Request, StatusCode};
use hyper_tls::HttpsConnector;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    path::PathBuf,
    sync::Arc,
};
use tokio::fs;
use uuid::Uuid;

pub(crate) struct Firestore {
    client: Arc<Client<HttpsConnector<HttpConnector>>>,
    firebase_project: String,
    parent_path: String,
    _oauth_token: String,
}

impl Firestore {
    pub async fn new<P: Into<PathBuf>>(
        json_key_path: P,
        parent_path: String,
    ) -> storage::Result<Firestore> {
        // Create shared HTTP client
        let mut https = HttpsConnector::new();
        https.https_only(true);
        let client = Client::builder().build::<_, hyper::Body>(https);

        // Get OAuth token
        let json_key = read_json_key(json_key_path).await?;
        let jwt = build_jwt(&json_key.client_email, &json_key.private_key).await?;
        let oauth_token = get_oauth_token(jwt, &client).await?;

        Ok(Firestore {
            client: Arc::new(client),
            firebase_project: json_key.project_id,
            parent_path,
            _oauth_token: oauth_token,
        })
    }

    async fn list<T: TryFrom<Document>>(&self) -> storage::Result<Vec<T>> {
        let mut done = false;
        let mut next_page_token = None;
        while !done {
            let uri: String;
            if let Some(token) = next_page_token {
                uri = format!(
                    "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/{}?pageSize=100&pageToken={}",
                    self.firebase_project, self.parent_path, token
                );
            } else {
                uri = format!(
                    "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/{}?pageSize=100",
                    self.firebase_project, self.parent_path
                );
            }
            
            let req = Request::builder()
                .method(Method::GET)
                .uri(&uri)
                .header(HeaderName::from_static("accept"), "application/json")
                .header(
                    HeaderName::from_static("authorization"),
                    format!("Bearer {}", &self._oauth_token),
                )
                .body(Body::empty())
                .unwrap();
            info!("GET {}", uri);
            let resp = self.client.request(req).await.unwrap();
            let status = resp.status();
            let body_bytes = body::to_bytes(resp.into_body()).await.unwrap();
            info!(
                "HTTP {} {}",
                status,
                String::from_utf8(body_bytes.to_vec()).unwrap(),
            );
            match status {
                StatusCode::OK => {
                    let list_response: ListDocumentsResponse = serde_json::from_slice(&body_bytes).unwrap();
                    next_page_token = list_response.next_page_token;
                    done = true;
                    let mut ret = vec![];
                    for doc in list_response.documents {
                        let result = doc.try_into();
                        match result {
                            Ok(t) => ret.push(t),
                            Err(_) => todo!(),
                        };
                    }
                    return Ok(ret);
                },
                _ => {
                    todo!()
                },
            }
        }

        todo!()
    }

    async fn read<T: TryFrom<Document>>(&self, id: &Uuid) -> storage::Result<Option<T>> {
        let uri = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/{}/{}",
            self.firebase_project, self.parent_path, id
        );
        let req = Request::builder()
            .method(Method::GET)
            .uri(&uri)
            .header(HeaderName::from_static("accept"), "application/json")
            .header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", &self._oauth_token),
            )
            .body(Body::empty())
            .unwrap();
        info!("GET {}", uri);
        let resp = self.client.request(req).await.unwrap();
        let status = resp.status();
        let body_bytes = body::to_bytes(resp.into_body()).await.unwrap();
        info!(
            "HTTP {} {}",
            status,
            String::from_utf8(body_bytes.to_vec()).unwrap(),
        );
        match status {
            StatusCode::OK => {
                let doc: Document = serde_json::from_slice(&body_bytes)?;
                let result: Result<T, _> = doc.try_into();
                match result {
                    Ok(ret) => Ok(Some(ret)),
                    Err(e) => todo!(),
                }
            }
            _ => todo!(),
        }
    }

    async fn write<T: Into<Document>>(&self, id: &Uuid, value: T) -> storage::Result<()> {
        let uri = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/{}/{}",
            self.firebase_project, self.parent_path, id
        );
        let doc: Document = value.into();
        let req = Request::builder()
            .method(Method::PATCH)
            .uri(&uri)
            .header(HeaderName::from_static("accept"), "application/json")
            .header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", &self._oauth_token),
            )
            .body(Body::from(serde_json::to_string(&doc).unwrap()))
            .unwrap();
        info!("PATCH {} {:?}", uri, req);
        let resp = self.client.request(req).await.unwrap();
        let status = resp.status();
        let body_bytes = body::to_bytes(resp.into_body()).await.unwrap();
        info!(
            "HTTP {} {}",
            status,
            String::from_utf8(body_bytes.to_vec()).unwrap()
        );
        match status {
            StatusCode::OK => Ok(()),
            _ => todo!(),
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    #[serde(skip_serializing)]
    pub name: String,
    pub fields: HashMap<String, DocumentField>,
    #[serde(skip_serializing)]
    pub create_time: String,
    #[serde(skip_serializing)]
    pub update_time: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DocumentField {
    NullValue,
    StringValue(String),
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
    iss: String,   // Email address of the service account
    scope: String, // Space-delimited list of the permissions requested
    aud: String, // Inteneded target of assertion, should just be https://oauth2.googleapis.com/token
    exp: usize, // Expiration time of the assertion, as seconds since epoch. Maximum of 1 hour after issuance.
    iat: usize, // Assertion issuance time, as seconds since epoch.
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

    let token = encode(
        &Header::new(Algorithm::RS256),
        &claims,
        &EncodingKey::from_rsa_pem(private_key.as_ref())?,
    )?;
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

async fn get_oauth_token(
    jwt: String,
    http_client: &Client<HttpsConnector<HttpConnector>>,
) -> storage::Result<String> {
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
        }
        _ => {
            let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
            error!("Response: {} {}", status, body_str);
            Err(storage::Error::OAuth(format!(
                "OAuth flow returned HTTP {} with body content: {}",
                status, body_str
            )))
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListDocumentsResponse {
    documents: Vec<Document>,
    next_page_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Card;

    static JSON_KEY_PATH: &str = "secrets/service_account.json";

    #[tokio::test(threaded_scheduler)]
    async fn can_read_key_from_json() {
        tokio::spawn(async {
            read_json_key(JSON_KEY_PATH).await.unwrap();
        })
        .await
        .unwrap();
    }

    #[tokio::test(threaded_scheduler)]
    async fn test_build_jwt() {
        tokio::spawn(async {
            let key = read_json_key(JSON_KEY_PATH).await.unwrap();
            build_jwt(&key.client_email, &key.private_key)
                .await
                .unwrap();
        })
        .await
        .unwrap();
    }

    #[tokio::test(threaded_scheduler)]
    async fn test_get_oauth_token() {
        tokio::spawn(async {
            let key = read_json_key(JSON_KEY_PATH).await.unwrap();
            let jwt = build_jwt(&key.client_email, &key.private_key)
                .await
                .unwrap();

            let mut https = HttpsConnector::new();
            https.https_only(true);
            let client = Client::builder().build::<_, hyper::Body>(https);

            get_oauth_token(jwt, &client).await.unwrap();
        })
        .await
        .unwrap();
    }

    #[tokio::test(threaded_scheduler)]
    async fn test_write_then_read_card() {
        tokio::spawn(async {
            let firestore = Firestore::new(JSON_KEY_PATH, "cards".to_owned())
                .await
                .unwrap();
            let id_to_write = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
            let card_to_write = Card {
                id: id_to_write.clone(),
                name: "test name".to_owned(),
                description: "test description".to_owned(),
                image_uri: "https://localhost/test_uri.png".to_owned(),
            };
            firestore
                .write(&id_to_write, card_to_write.clone())
                .await
                .unwrap();

            let card: Card = firestore
                .read(&Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap())
                .await
                .unwrap()
                .unwrap();

            assert_eq!(card_to_write, card);
        })
        .await
        .unwrap();
    }

    #[tokio::test(threaded_scheduler)]
    async fn test_list_cards() {
        tokio::spawn(async {
            let firestore = Firestore::new(JSON_KEY_PATH, "cards".to_owned())
                .await
                .unwrap();
            let cards: Vec<Card> = firestore.list().await.unwrap();
        })
        .await
        .unwrap();
    }
}
