use crate::storage;
use chrono::{DateTime, Utc};
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
    time,
};
use tokio::{
    fs,
    sync::oneshot::error::TryRecvError,
    sync::{oneshot, RwLock},
    task,
};
use uuid::Uuid;

pub struct Firestore {
    client: Arc<Client<HttpsConnector<HttpConnector>>>,
    firebase_project: String,
    parent_path: String,
    _oauth_token: Arc<RwLock<String>>,
    _oauth_refresh_handle: task::JoinHandle<()>,
    _oauth_refresh_cancellation: oneshot::Sender<()>,
}

impl Firestore {
    pub async fn new<P: Into<PathBuf>>(
        json_key_path: P,
        parent_path: String,
    ) -> storage::Result<Firestore> {
        // Create shared HTTP client
        let mut https = HttpsConnector::new();
        https.https_only(true);
        let client = Arc::new(Client::builder().build::<_, hyper::Body>(https));

        // Get OAuth token
        let json_key = read_json_key(json_key_path).await?;
        let jwt = build_jwt(&json_key.client_email, &json_key.private_key).await?;
        let (oauth_token, oauth_expires_in) = get_oauth_token(jwt, &client).await?;
        let oauth_token = Arc::new(RwLock::new(oauth_token));
        let mut oauth_expires_in: u64 = oauth_expires_in as u64;

        // Start background task to refresh OAuth token
        let (tx, mut rx) = oneshot::channel();
        let client_clone = Arc::clone(&client);
        let client_email = json_key.client_email.clone();
        let private_key = json_key.private_key.clone();
        let oauth_token_clone = Arc::clone(&oauth_token);
        let handle = tokio::spawn(async move {
            while let Err(TryRecvError::Empty) = rx.try_recv() {
                // Refresh token 10 minutes before expiration
                let delay_duration = tokio::time::Duration::from_secs(oauth_expires_in - 600);
                tokio::time::delay_for(delay_duration).await;
                if let Err(TryRecvError::Closed) = rx.try_recv() {
                    debug!("Stopping background task to refresh OAuth token");
                    break;
                } else {
                    debug!("Renewing OAuth token");
                    match build_jwt(&client_email, &private_key).await {
                        Ok(jwt) => match get_oauth_token(jwt, &client_clone).await {
                            Ok(ret) => {
                                let mut oauth_token = oauth_token_clone.write().await;
                                *oauth_token = ret.0;
                                oauth_expires_in = ret.1 as u64;
                                debug!("Successfully renewed OAuth token");
                            }
                            Err(e) => {
                                error!("Failed to get OAuth token, will retry renewal flow in 10s. Error: {}", e);
                                oauth_expires_in = 10;
                            }
                        },
                        Err(e) => {
                            error!(
                                "Failed to build JWT, will retry renewal flow in 10s. Error: {}",
                                e
                            );
                            oauth_expires_in = 10;
                        }
                    }
                }
            }
        });

        Ok(Firestore {
            client,
            firebase_project: json_key.project_id,
            parent_path,
            _oauth_token: oauth_token,
            _oauth_refresh_handle: handle,
            _oauth_refresh_cancellation: tx,
        })
    }

    pub async fn delete<T: TryFrom<Document>>(&self, id: &Uuid) -> storage::Result<()> {
        let uri = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/{}/{}",
            self.firebase_project, self.parent_path, id
        );
        let req = Request::builder()
            .method(Method::DELETE)
            .uri(&uri)
            .header(HeaderName::from_static("accept"), "application/json")
            .header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", *self._oauth_token.read().await),
            )
            .body(Body::empty())
            .unwrap();
        debug!("DELETE {}", uri);
        let resp = self.client.request(req).await?;
        let status = resp.status();
        let body_bytes = body::to_bytes(resp.into_body()).await.unwrap_or_default();
        debug!(
            "HTTP {}  {}",
            status,
            String::from_utf8(body_bytes.to_vec()).unwrap_or_else(|_| "<mangled body>".to_owned()),
        );
        match status {
            StatusCode::OK => Ok(()),
            _ => Err(storage::Error::Other(
                String::from_utf8(body_bytes.to_vec()).unwrap_or_else(|_| "<mangled body>".to_owned())
            ))
        }
    }

    pub async fn get<T: TryFrom<Document>>(&self, id: &Uuid) -> storage::Result<Option<T>> {
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
                format!("Bearer {}", *self._oauth_token.read().await),
            )
            .body(Body::empty())
            .unwrap();
        debug!("GET {}", uri);
        let resp = self.client.request(req).await?;
        let status = resp.status();
        let body_bytes = body::to_bytes(resp.into_body()).await.unwrap_or_default();
        debug!(
            "HTTP {} {}",
            status,
            String::from_utf8(body_bytes.to_vec()).unwrap_or_else(|_| "<mangled body>".to_owned()),
        );
        match status {
            StatusCode::OK => {
                let doc: Document = serde_json::from_slice(&body_bytes)?;
                let result: Result<T, _> = doc.try_into();
                match result {
                    Ok(ret) => Ok(Some(ret)),
                    Err(_) => todo!(),
                }
            }
            StatusCode::NOT_FOUND => Ok(None),
            _ => todo!(),
        }
    }

    pub async fn insert<T: Into<Document>>(&self, id: &Uuid, value: T) -> storage::Result<()> {
        let uri = format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/{}?documentId={}",
            self.firebase_project, self.parent_path, id
        );
        let doc: Document = value.into();
        let req = Request::builder()
            .method(Method::POST)
            .uri(&uri)
            .header(HeaderName::from_static("accept"), "application/json")
            .header(
                HeaderName::from_static("authorization"),
                format!("Bearer {}", *self._oauth_token.read().await),
            )
            .body(Body::from(serde_json::to_string(&doc)?))
            .unwrap();
        debug!("POST {} {:?}", uri, req);
        let resp = self.client.request(req).await?;
        let status = resp.status();
        let body_bytes = body::to_bytes(resp.into_body()).await.unwrap_or_default();
        debug!(
            "HTTP {} {}",
            status,
            String::from_utf8(body_bytes.to_vec()).unwrap_or_else(|_| "<mangled body>".to_owned()),
        );
        match status {
            StatusCode::OK => Ok(()),
            StatusCode::CONFLICT => Err(storage::Error::Conflict(format!(
                "Value with id {} already exists under path '{}'",
                id.to_string(),
                self.parent_path
            ))),
            _ => Err(storage::Error::Other(format!(
                "Error inserting id {} under path '{}': HTTP {} {}",
                id.to_string(),
                self.parent_path,
                status,
                String::from_utf8(body_bytes.to_vec())
                    .unwrap_or_else(|_| "<mangled body>".to_owned()),
            ))),
        }
    }

    pub async fn list<T: TryFrom<Document>>(&self) -> storage::Result<Vec<T>> {
        const PAGE_SIZE: usize = 100;
        let mut ret = vec![];
        let mut next_page_token = None;
        loop {
            let uri: String;
            if let Some(token) = next_page_token {
                uri = format!(
                    "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/{}?pageSize={}&pageToken={}",
                    self.firebase_project, self.parent_path, PAGE_SIZE, token
                );
            } else {
                uri = format!(
                    "https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents/{}?pageSize={}",
                    self.firebase_project, self.parent_path, PAGE_SIZE
                );
            }

            let req = Request::builder()
                .method(Method::GET)
                .uri(&uri)
                .header(HeaderName::from_static("accept"), "application/json")
                .header(
                    HeaderName::from_static("authorization"),
                    format!("Bearer {}", *self._oauth_token.read().await),
                )
                .body(Body::empty())
                .unwrap();
            debug!("GET {}", uri);
            let resp = self.client.request(req).await?;
            let status = resp.status();
            let body_bytes = body::to_bytes(resp.into_body()).await.unwrap_or_default();
            debug!(
                "HTTP {} {}",
                status,
                String::from_utf8(body_bytes.to_vec())
                    .unwrap_or_else(|_| "<mangled body>".to_owned()),
            );

            let list_response: ListDocumentsResponse;
            match status {
                StatusCode::OK => {
                    list_response = serde_json::from_slice(&body_bytes)?;
                    for doc in list_response.documents {
                        let result = doc.try_into();
                        match result {
                            Ok(t) => ret.push(t),
                            Err(_) => todo!(),
                        };
                    }

                    next_page_token = list_response.next_page_token;
                }
                _ => todo!(),
            }

            if let None = next_page_token {
                break;
            }
        }

        Ok(ret)
    }

    pub async fn upsert<T: Into<Document>>(&self, id: &Uuid, value: T) -> storage::Result<()> {
        // TODO return indication of whether it was an insert or an update
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
                format!("Bearer {}", *self._oauth_token.read().await),
            )
            .body(Body::from(serde_json::to_string(&doc)?))
            .unwrap();
        debug!("PATCH {} {:?}", uri, req);
        let resp = self.client.request(req).await?;
        let status = resp.status();
        let body_bytes = body::to_bytes(resp.into_body()).await.unwrap_or_default();
        debug!(
            "HTTP {} {}",
            status,
            String::from_utf8(body_bytes.to_vec()).unwrap_or_else(|_| "<mangled body>".to_owned()),
        );
        match status {
            StatusCode::OK => Ok(()),
            _ => Err(storage::Error::Other(format!(
                "Error upserting id {} under path '{}': HTTP {} {}",
                id.to_string(),
                self.parent_path,
                status,
                String::from_utf8(body_bytes.to_vec())
                    .unwrap_or_else(|_| "<mangled body>".to_owned()),
            ))),
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

impl Document {
    pub fn new(fields: HashMap<String, DocumentField>) -> Document {
        Document {
            name: "".to_owned(),
            fields,
            create_time: "".to_owned(),
            update_time: "".to_owned(),
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DocumentField {
    NullValue,
    ArrayValue(DocumentArrayValue),
    DoubleValue(String),
    IntegerValue(String),
    StringValue(String),
    TimestampValue(DateTime<Utc>),
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentArrayValue {
    pub values: Option<Vec<DocumentField>>,
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
) -> storage::Result<(String, usize)> {
    let sw = time::Instant::now();

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
    let response = http_client.request(request).await?;
    let status = response.status();
    let body_bytes = body::to_bytes(response.into_body())
        .await
        .unwrap_or_default();
    let ret;

    match status {
        StatusCode::OK => {
            let body: OAuth2Response = serde_json::from_slice(&body_bytes)?;
            debug!("Response: {} {:?}", status, body);
            ret = Ok((body.access_token, body.expires_in));
        }
        _ => {
            let body_str = String::from_utf8(body_bytes.to_vec())
                .unwrap_or_else(|_| "<mangled body>".to_owned());
            error!(
                "Non-success when requesting OAuth token. Response: {} {}",
                status, body_str
            );
            ret = Err(storage::Error::OAuth(format!(
                "OAuth flow returned HTTP {} with body content: {}",
                status, body_str
            )));
        }
    }

    info!("Obtained OAuth token, took {:?}", sw.elapsed());
    ret
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
    use crate::models::{Card, User};
    use chrono::SubsecRound;

    static JSON_KEY_PATH: &str = "secrets/service_account.json";

    #[ignore]
    #[tokio::test(threaded_scheduler)]
    async fn can_read_key_from_json() {
        tokio::spawn(async {
            read_json_key(JSON_KEY_PATH).await.unwrap();
        })
        .await
        .unwrap();
    }

    #[ignore]
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

    #[ignore]
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

    #[ignore]
    #[tokio::test(threaded_scheduler)]
    async fn test_upsert_then_get_card() {
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
                .upsert(&id_to_write, card_to_write.clone())
                .await
                .unwrap();

            let card: Card = firestore
                .get(&Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap())
                .await
                .unwrap()
                .unwrap();

            assert_eq!(card_to_write, card);
        })
        .await
        .unwrap();
    }

    #[ignore]
    #[tokio::test(threaded_scheduler)]
    async fn test_upsert_then_get_user() {
        tokio::spawn(async {
            let firestore = Firestore::new(JSON_KEY_PATH, "users".to_owned())
                .await
                .unwrap();
            let id_to_write = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
            let user_to_write = User {
                id: id_to_write,
                cards: vec![id_to_write],
                currency: 50,
                daily_last_claimed: Utc::now().trunc_subsecs(6),
            };
            firestore
                .upsert(&id_to_write, user_to_write.clone())
                .await
                .unwrap();

            let user: User = firestore
                .get(&Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap())
                .await
                .unwrap()
                .unwrap();

            assert_eq!(user_to_write, user);
        })
        .await
        .unwrap();
    }

    #[ignore]
    #[tokio::test(threaded_scheduler)]
    async fn test_list_cards() {
        tokio::spawn(async {
            let firestore = Firestore::new(JSON_KEY_PATH, "cards".to_owned())
                .await
                .unwrap();
            let cards: Vec<Card> = firestore.list().await.unwrap();
            assert!(cards.len() > 1);
        })
        .await
        .unwrap();
    }
}
