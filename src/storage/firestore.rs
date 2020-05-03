use crate::storage;
use chrono::{DateTime, Utc};
use hyper::{
    body::{self, Body},
    client::{Client, HttpConnector},
    header::HeaderName,
    Method, Request, StatusCode,
};
use hyper_tls::HttpsConnector;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use num::{Float, Integer};
use serde::{Deserialize, Serialize};
use std::{
    any::type_name,
    collections::HashMap,
    convert::{TryFrom, TryInto},
    path::PathBuf,
    str::FromStr,
    sync::Arc,
    time,
};
use tokio::{
    fs,
    sync::oneshot::{self, error::TryRecvError},
    sync::RwLock,
    task,
};
use uuid::Uuid;

pub struct FirestoreClient {
    firestore: Arc<Firestore>,
    parent_path: String,
    collection_id: String,
}

impl FirestoreClient {
    pub fn new(
        firestore: Arc<Firestore>,
        collection_parent_path: Option<String>,
        collection_id: String,
    ) -> FirestoreClient {
        let parent_path = match collection_parent_path {
            Some(parent_path) => format!(
                "projects/{}/databases/(default)/documents/{}",
                firestore.firebase_project_id, parent_path,
            ),
            None => format!(
                "projects/{}/databases/(default)/documents",
                firestore.firebase_project_id,
            ),
        };
        FirestoreClient {
            firestore,
            parent_path,
            collection_id,
        }
    }

    pub fn new_for_subcollection(
        firestore_client: &FirestoreClient,
        subcollection_relative_path: String,
        subcollection_id: String,
    ) -> FirestoreClient {
        let parent_path = format!(
            "{}/{}/{}",
            firestore_client.parent_path,
            firestore_client.collection_id,
            subcollection_relative_path
        );
        FirestoreClient {
            firestore: Arc::clone(&firestore_client.firestore),
            parent_path,
            collection_id: subcollection_id,
        }
    }

    pub async fn delete<T: TryFrom<Document>>(&self, id: &Uuid) -> storage::Result<()> {
        let name = format!(
            "{}/{}/{}",
            self.parent_path,
            self.collection_id,
            id.to_string()
        );
        self.firestore.delete::<T>(&name).await
    }

    pub async fn get<T: TryFrom<Document>>(&self, id: &Uuid) -> storage::Result<Option<T>> {
        let name = format!(
            "{}/{}/{}",
            self.parent_path,
            self.collection_id,
            id.to_string()
        );
        self.firestore.get::<T>(&name).await
    }

    pub async fn insert<T: Into<Document>>(&self, id: &Uuid, value: T) -> storage::Result<()> {
        self.firestore
            .create_document(
                &self.parent_path,
                &self.collection_id,
                &id.to_string(),
                value,
            )
            .await
    }

    pub async fn list<T: TryFrom<Document>>(&self) -> storage::Result<Vec<T>> {
        self.firestore
            .list::<T>(&self.parent_path, &self.collection_id)
            .await
    }

    pub async fn upsert<T: Into<Document>>(&self, id: &Uuid, value: T) -> storage::Result<()> {
        let name = format!(
            "{}/{}/{}",
            self.parent_path,
            self.collection_id,
            id.to_string()
        );
        self.firestore.patch(&name, value).await
    }
}

pub struct Firestore {
    client: Arc<Client<HttpsConnector<HttpConnector>>>,
    firebase_project_id: String,
    _oauth_token: Arc<RwLock<String>>,
    _oauth_refresh_handle: task::JoinHandle<()>,
    _oauth_refresh_cancellation: oneshot::Sender<()>,
}

impl Firestore {
    pub async fn new<P: Into<PathBuf>>(json_key_path: P) -> storage::Result<Firestore> {
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
                    info!("Renewing OAuth token");
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
            firebase_project_id: json_key.project_id,
            _oauth_token: oauth_token,
            _oauth_refresh_handle: handle,
            _oauth_refresh_cancellation: tx,
        })
    }

    async fn delete<T: TryFrom<Document>>(&self, name: &str) -> storage::Result<()> {
        let uri = format!("https://firestore.googleapis.com/v1/{}", name);
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
                String::from_utf8(body_bytes.to_vec())
                    .unwrap_or_else(|_| "<mangled body>".to_owned()),
            )),
        }
    }

    async fn get<T: TryFrom<Document>>(&self, name: &str) -> storage::Result<Option<T>> {
        let uri = format!("https://firestore.googleapis.com/v1/{}", name);
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

    async fn create_document<T: Into<Document>>(
        &self,
        parent: &str,
        collection_id: &str,
        document_id: &str,
        value: T,
    ) -> storage::Result<()> {
        let uri = format!(
            "https://firestore.googleapis.com/v1/{}/{}?documentId={}",
            parent, collection_id, document_id
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
                "Could not create document with id {} under parent '{}' collection '{}' as it already exists",
                document_id,
                parent,
                collection_id,
            ))),
            _ => Err(storage::Error::Other(format!(
                "Error creating document with id {} under parent '{}' collection '{}': HTTP {} {}",
                document_id,
                parent,
                collection_id,
                status,
                String::from_utf8(body_bytes.to_vec())
                    .unwrap_or_else(|_| "<mangled body>".to_owned()),
            ))),
        }
    }

    async fn list<T: TryFrom<Document>>(
        &self,
        parent: &str,
        collection_id: &str,
    ) -> storage::Result<Vec<T>> {
        const PAGE_SIZE: usize = 100;
        let mut ret = vec![];
        let mut next_page_token = None;
        loop {
            let uri: String;
            if let Some(token) = next_page_token {
                uri = format!(
                    "https://firestore.googleapis.com/v1/{}/{}?pageSize={}&pageToken={}",
                    parent, collection_id, PAGE_SIZE, token
                );
            } else {
                uri = format!(
                    "https://firestore.googleapis.com/v1/{}/{}?pageSize={}",
                    parent, collection_id, PAGE_SIZE
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
                    if let Some(docs) = list_response.documents {
                        for doc in docs {
                            let result = doc.try_into();
                            match result {
                                Ok(t) => ret.push(t),
                                Err(_) => todo!(),
                            };
                        }
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

    async fn patch<T: Into<Document>>(&self, document_name: &str, value: T) -> storage::Result<()> {
        // TODO return indication of whether it was an insert or an update if possible
        let uri = format!("https://firestore.googleapis.com/v1/{}", document_name);
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
                "Error patching document '{}': HTTP {} {}",
                document_name,
                status,
                String::from_utf8(body_bytes.to_vec())
                    .unwrap_or_else(|_| "<mangled body>".to_owned()),
            ))),
        }
    }

    async fn run_query<T: TryFrom<Document>>(
        &self,
        parent: &str,
        structured_query: StructuredQuery,
    ) -> storage::Result<T> {
        todo!()
    }
}

#[derive(Debug, Deserialize, Serialize)]
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

    pub fn extract_id(&self) -> Result<Uuid, String> {
        if let Some(id) = self.name.split('/').next_back() {
            if let Ok(id) = Uuid::parse_str(&id) {
                Ok(id)
            } else {
                Err(format!("Unable to convert id '{}' to a uuid", id))
            }
        } else {
            Err(format!("Invalid name '{}'", self.name))
        }
    }

    pub fn extract_double<T: From<f64> + Float>(&self, field_name: &str) -> Result<T, String> {
        if let Some(doc_field) = self.fields.get(field_name) {
            if let DocumentField::DoubleValue(ret) = doc_field {
                Ok((*ret).into())
            } else {
                Err(format!("Error parsing DoubleValue from {:?}", doc_field))
            }
        } else {
            Err(format!("Missing field {}", field_name))
        }
    }

    pub fn extract_integer<T: FromStr + Integer>(&self, field_name: &str) -> Result<T, String> {
        if let Some(doc_field) = self.fields.get(field_name) {
            if let DocumentField::IntegerValue(ret_str) = doc_field {
                if let Ok(ret) = ret_str.parse() {
                    Ok(ret)
                } else {
                    Err(format!(
                        "Error casting to {} from {}",
                        type_name::<T>(),
                        ret_str
                    ))
                }
            } else {
                Err(format!("Error parsing IntegerValue from {:?}", doc_field))
            }
        } else {
            Err(format!("Missing field {}", field_name))
        }
    }

    pub fn extract_string(&self, field_name: &str) -> Result<String, String> {
        if let Some(doc_field) = self.fields.get(field_name) {
            if let DocumentField::StringValue(ret_str) = doc_field {
                Ok(ret_str.to_string())
            } else {
                Err(format!("Error parsing StringValue from {:?}", doc_field))
            }
        } else {
            Err(format!("Missing field {}", field_name))
        }
    }

    pub fn extract_timestamp(&self, field_name: &str) -> Result<DateTime<Utc>, String> {
        if let Some(doc_field) = self.fields.get(field_name) {
            if let DocumentField::TimestampValue(dt) = doc_field {
                Ok(*dt)
            } else {
                Err(format!("Error parsing TimestampValue from {:?}", doc_field))
            }
        } else {
            Err(format!("Missing field {}", field_name))
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DocumentField {
    NullValue,
    ArrayValue(DocumentArrayValue),
    DoubleValue(f64),
    IntegerValue(String),
    MapValue(DocumentMapValue),
    StringValue(String),
    TimestampValue(DateTime<Utc>),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentArrayValue {
    pub values: Option<Vec<DocumentField>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DocumentMapValue {
    pub fields: Option<HashMap<String, DocumentField>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListDocumentsResponse {
    documents: Option<Vec<Document>>,
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunQueryResponse {
    transation: Option<String>,
    document: Option<Document>,
    read_time: Option<String>,
    skipped_results: Option<u32>,
}

// TODO
struct StructuredQuery {
    select: String,
    from: Vec<String>,
    r#where: String,
    order_by: Vec<String>,
    start_at: String,
    end_at: String,
    offset: u32,
    limit: u32,
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
            info!("Obtained OAuth token, took {:?}", sw.elapsed());
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
            info!("Failed to obtain OAuth token, took {:?}", sw.elapsed());
        }
    }

    ret
}

#[cfg(test)]
mod tests {
    use super::*;

    static JSON_KEY_PATH: &str = "secrets/service_account.json";

    #[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
    struct TestItem {
        pub id: Uuid,
        pub number: u32,
        pub test_case: String,
    }

    impl TryFrom<Document> for TestItem {
        type Error = String;

        fn try_from(value: Document) -> Result<Self, Self::Error> {
            let id = value.extract_id();
            if let Err(e) = id {
                return Err(format!(
                    "Could not convert Document to TestItem: error parsing id: {}",
                    e
                ));
            }
            let id = id.unwrap();

            let number;
            if let Some(DocumentField::IntegerValue(number_str)) = value.fields.get("number") {
                match number_str.parse() {
                    Ok(n) => number = n,
                    Err(e) => {
                        return Err(format!("Could not convert Document to TestItem: error parsing field 'number': {}", e));
                    }
                }
            } else {
                return Err(
                    "Could not convert Document to TestItem: missing field 'number'".to_owned(),
                );
            }

            let test_case;
            if let Some(DocumentField::StringValue(test_case_str)) = value.fields.get("test_case") {
                test_case = test_case_str.to_string();
            } else {
                return Err(
                    "Could not convert Document to TestItem: missing field 'test_case'".to_owned(),
                );
            }

            Ok(TestItem {
                id,
                number,
                test_case,
            })
        }
    }

    impl Into<Document> for TestItem {
        fn into(self) -> Document {
            let mut fields = HashMap::new();
            fields.insert(
                "id".to_owned(),
                DocumentField::StringValue(self.id.to_string()),
            );
            fields.insert(
                "number".to_owned(),
                DocumentField::IntegerValue(self.number.to_string()),
            );
            fields.insert(
                "test_case".to_owned(),
                DocumentField::StringValue(self.test_case),
            );
            Document::new(fields)
        }
    }

    #[test]
    fn can_convert_between_document_and_test_item() {
        let test_item = TestItem {
            id: Uuid::new_v4(),
            number: 42,
            test_case: "can_convert_between_document_and_test_item".to_owned(),
        };

        let mut doc: Document = test_item.clone().into();
        doc.name = format!("parent_path/{}", test_item.id.to_string());

        let test_item_from_doc: TestItem = doc.try_into().unwrap();

        assert_eq!(test_item, test_item_from_doc);
    }

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
    async fn can_build_jwt() {
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
    async fn can_get_oauth_token() {
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
    async fn upsert_then_get() {
        tokio::spawn(async {
            let firestore = Firestore::new(JSON_KEY_PATH).await.unwrap();
            let firestore = FirestoreClient::new(Arc::new(firestore), None, "_test".to_owned());
            let id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
            let test_item = TestItem {
                id,
                number: 0,
                test_case: "upsert_then_get".to_owned(),
            };
            firestore.upsert(&id, test_item.clone()).await.unwrap();
            let ret = firestore.get::<TestItem>(&id).await.unwrap().unwrap();
            assert_eq!(ret, test_item);
        })
        .await
        .unwrap();
    }

    #[ignore]
    #[tokio::test(threaded_scheduler)]
    async fn list_empty_collection() {
        tokio::spawn(async {
            let firestore = Firestore::new(JSON_KEY_PATH).await.unwrap();
            let firestore = FirestoreClient::new(
                Arc::new(firestore),
                None,
                "_test_list_empty_collection".to_owned(),
            );
            let ret = firestore.list::<TestItem>().await.unwrap();
            assert_eq!(ret, vec![]);
        })
        .await
        .unwrap();
    }

    #[ignore]
    #[tokio::test(threaded_scheduler)]
    async fn list_non_empty_collection() {
        tokio::spawn(async {
            let firestore = Firestore::new(JSON_KEY_PATH).await.unwrap();
            let firestore = FirestoreClient::new(
                Arc::new(firestore),
                None,
                "_test_list_non_empty_collection".to_owned(),
            );
            let id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
            let test_item = TestItem {
                id,
                number: 1,
                test_case: "list_non_empty_collection".to_owned(),
            };
            firestore.upsert(&id, test_item.clone()).await.unwrap();
            let ret = firestore.list::<TestItem>().await.unwrap();
            assert_eq!(ret.len(), 1);
            assert_eq!(ret[0], test_item);
        })
        .await
        .unwrap();
    }

    #[ignore]
    #[tokio::test(threaded_scheduler)]
    async fn list_empty_subcollection() {
        tokio::spawn(async {
            let firestore = Firestore::new(JSON_KEY_PATH).await.unwrap();
            let firestore = FirestoreClient::new(Arc::new(firestore), None, "_test".to_owned());
            let id = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
            let test_item = TestItem {
                id,
                number: 2,
                test_case: "list_empty_subcollection".to_owned(),
            };
            firestore.upsert(&id, test_item).await.unwrap();
            let sub_fs = FirestoreClient::new_for_subcollection(
                &firestore,
                id.to_string(),
                "test".to_owned(),
            );
            let ret = sub_fs.list::<TestItem>().await.unwrap();
            assert_eq!(ret, vec![]);
        })
        .await
        .unwrap();
    }

    #[ignore]
    #[tokio::test(threaded_scheduler)]
    async fn list_non_empty_subcollection() {
        tokio::spawn(async {
            let firestore = Firestore::new(JSON_KEY_PATH).await.unwrap();
            let firestore = FirestoreClient::new(Arc::new(firestore), None, "_test".to_owned());
            let id = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();
            let test_item = TestItem {
                id,
                number: 3,
                test_case: "list_non_empty_subcollection".to_owned(),
            };
            firestore.upsert(&id, test_item.clone()).await.unwrap();
            let sub_fs = FirestoreClient::new_for_subcollection(
                &firestore,
                id.to_string(),
                "test".to_owned(),
            );
            sub_fs.upsert(&id, test_item.clone()).await.unwrap();
            let ret = sub_fs.list::<TestItem>().await.unwrap();
            assert_eq!(ret.len(), 1);
            assert_eq!(ret[0], test_item);
        })
        .await
        .unwrap();
    }
}
