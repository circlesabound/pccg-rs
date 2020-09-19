use crate as storage;
use chrono::{DateTime, SubsecRound, Utc};
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
    sync::mpsc,
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

    pub async fn begin_transaction(
        &self,
        transaction_type: TransactionType,
    ) -> storage::Result<Transaction> {
        let database = format!(
            "projects/{}/databases/(default)/documents",
            self.firestore.firebase_project_id,
        );
        let transaction_opts = match transaction_type {
            TransactionType::ReadOnly => TransactionOptions::ReadOnly(ReadOnlyTransactionOptions {
                read_time: Utc::now().trunc_subsecs(6),
            }),
            TransactionType::ReadWrite => {
                TransactionOptions::ReadWrite(ReadWriteTransactionOptions {
                    retry_transaction: None, // TODO figure out what this actually does
                })
            }
        };
        self.firestore
            .begin_transaction(&database, transaction_opts)
            .await
    }

    pub async fn batch_get<T: TryFrom<Document>>(
        &self,
        ids: &Vec<Uuid>,
        transaction: Option<&Transaction>,
    ) -> storage::Result<HashMap<Uuid, Option<T>>> {
        let mut id_to_name_map = HashMap::new();
        for id in ids.iter() {
            let name = format!(
                "{}/{}/{}",
                self.parent_path,
                self.collection_id,
                id.to_string(),
            );
            id_to_name_map.insert(id.clone(), name);
        }

        let database = format!(
            "projects/{}/databases/(default)",
            self.firestore.firebase_project_id,
        );

        match self
            .firestore
            .batch_get(
                &database,
                id_to_name_map.values().cloned().collect(),
                transaction,
            )
            .await
        {
            Ok(mut doc_map) => {
                Ok(id_to_name_map
                    .into_iter()
                    .map(|(id, name)| {
                        let doc = doc_map.remove(&name).unwrap();
                        let opt = match doc {
                            Some(doc) => match doc.try_into() {
                                Ok(t) => Some(t),
                                Err(_) => {
                                    // For now treat conversion error as missing doc
                                    error!("Failed to convert Document to requested type");
                                    None
                                }
                            },
                            None => None,
                        };
                        (id, opt)
                    })
                    .collect())
            }
            Err(e) => Err(e),
        }
    }

    pub async fn commit_transaction(&self, transaction: Transaction) -> storage::Result<()> {
        self.firestore.commit(transaction).await
    }

    pub async fn delete<T: TryFrom<Document>>(
        &self,
        id: &Uuid,
        transaction: Option<&Transaction>,
    ) -> storage::Result<()> {
        let name = format!(
            "{}/{}/{}",
            self.parent_path,
            self.collection_id,
            id.to_string()
        );
        match transaction {
            Some(t) => {
                let write = Write::Delete { delete: name };
                match t.append_write(write).await {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e.into()),
                }
            }
            None => self.firestore.delete::<T>(&name).await,
        }
    }

    pub async fn get<T: TryFrom<Document>>(
        &self,
        id: &Uuid,
        transaction: Option<&Transaction>,
    ) -> storage::Result<Option<T>> {
        let name = format!(
            "{}/{}/{}",
            self.parent_path,
            self.collection_id,
            id.to_string()
        );
        self.firestore.get::<T>(&name, transaction).await
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

    pub async fn upsert<T: Into<Document>>(
        &self,
        id: &Uuid,
        value: T,
        transaction: Option<&Transaction>,
    ) -> storage::Result<()> {
        let name = format!(
            "{}/{}/{}",
            self.parent_path,
            self.collection_id,
            id.to_string()
        );
        match transaction {
            Some(t) => {
                let mut doc = value.into();
                doc.name = name;
                let write = Write::Update { update: doc };
                match t.append_write(write).await {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e.into()),
                }
            }
            None => self.firestore.patch(&name, value).await,
        }
    }
}

pub struct Firestore {
    client: Arc<Client<HttpsConnector<HttpConnector>>>,
    firebase_project_id: String,
    _oauth_token: Arc<RwLock<String>>,
    _oauth_refresh_handle: task::JoinHandle<()>,
    _oauth_refresh_cancellation: oneshot::Sender<()>,
    _drop_tx: mpsc::Sender<(String, String)>,
    _drop_handle: task::JoinHandle<()>,
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
        let (oauth_tx, mut oauth_rx) = oneshot::channel();
        let client_clone = Arc::clone(&client);
        let client_email = json_key.client_email.clone();
        let private_key = json_key.private_key.clone();
        let oauth_token_clone = Arc::clone(&oauth_token);
        let oauth_handle = tokio::spawn(async move {
            while let Err(TryRecvError::Empty) = oauth_rx.try_recv() {
                // Refresh token 10 minutes before expiration
                let delay_duration = tokio::time::Duration::from_secs(oauth_expires_in - 600);
                tokio::time::delay_for(delay_duration).await;
                if let Err(TryRecvError::Closed) = oauth_rx.try_recv() {
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

        // Start background task to clean up dropped transactions
        let (drop_tx, mut drop_rx) = mpsc::channel::<(String, String)>(50);
        let client_clone = Arc::clone(&client);
        let oauth_token_clone = Arc::clone(&oauth_token);
        let drop_handle = tokio::spawn(async move {
            while let Some((database, transaction_id)) = drop_rx.recv().await {
                let uri = format!("https://firestore.googleapis.com/v1/{}:rollback", database,);
                let body = RollbackRequest {
                    transaction: transaction_id.clone(),
                };
                let req = build_firestore_request(
                    Method::POST,
                    &uri,
                    &*oauth_token_clone.read().await,
                    Some(&body),
                )
                .await
                .unwrap();
                debug!("POST {} {:?}", uri, req);
                let resp = client_clone.request(req).await.unwrap();
                let status = resp.status();
                let body_bytes = body::to_bytes(resp.into_body()).await.unwrap_or_default();
                debug!(
                    "HTTP {} {}",
                    status,
                    String::from_utf8(body_bytes.to_vec())
                        .unwrap_or_else(|_| "<mangled body>".to_owned()),
                );
                // TODO handle it properly?
                match status {
                    StatusCode::OK => debug!("Successfully dropped transaction {}", transaction_id),
                    _ => error!("Error dropping transaction {}", transaction_id),
                }
            }
            debug!("Stopping background task to clean up dropped transactions");
        });

        Ok(Firestore {
            client,
            firebase_project_id: json_key.project_id,
            _oauth_token: oauth_token,
            _oauth_refresh_handle: oauth_handle,
            _oauth_refresh_cancellation: oauth_tx,
            _drop_tx: drop_tx,
            _drop_handle: drop_handle,
        })
    }

    async fn begin_transaction(
        &self,
        database: &str,
        transaction_opts: TransactionOptions,
    ) -> storage::Result<Transaction> {
        let uri = format!(
            "https://firestore.googleapis.com/v1/{}:beginTransaction",
            database,
        );
        let body = BeginTransactionRequest {
            options: transaction_opts,
        };
        let req = build_firestore_request(
            Method::POST,
            &uri,
            &*self._oauth_token.read().await,
            Some(&body),
        )
        .await?;
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
            StatusCode::OK => {
                let resp: BeginTransactionResponse = serde_json::from_slice(&body_bytes)?;
                Ok(Transaction::new(
                    database.to_owned(),
                    self._drop_tx.clone(),
                    Arc::clone(&self.client),
                    Arc::clone(&self._oauth_token),
                    resp.transaction,
                ))
            }
            _ => {
                error!("Non-success status code {} in begin_transaction", status);
                Err(storage::Error::Other(format!(
                    "Non-success status code {} in begin_transaction",
                    status
                )))
            }
        }
    }

    async fn batch_get(
        &self,
        database: &str,
        documents: Vec<String>,
        transaction: Option<&Transaction>,
    ) -> storage::Result<HashMap<String, Option<Document>>> {
        let uri = format!(
            "https://firestore.googleapis.com/v1/{}/documents:batchGet",
            database
        );

        let body: BatchGetRequest;
        let mut ret = HashMap::new();

        if let Some(t) = transaction {
            // If part of a transaction, only request the docs that are not already cached
            let mut filtered_doc_names = vec![];
            for doc_name in documents.into_iter() {
                if let Some(doc) = t.read_cache.read().await.get(&doc_name).cloned() {
                    debug!("Transaction read cache hit for {}", doc_name);
                    ret.insert(doc_name, Some(doc));
                } else {
                    filtered_doc_names.push(doc_name);
                }
            }

            body = BatchGetRequest {
                documents: filtered_doc_names,
                transaction: Some(t.transaction_id.clone()),
            };
        } else {
            body = BatchGetRequest {
                documents,
                transaction: None,
            };
        }

        let req = build_firestore_request(
            Method::POST,
            &uri,
            &*self._oauth_token.read().await,
            Some(&body),
        )
        .await?;
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
            StatusCode::OK => {
                let batch_get_docs: Vec<BatchGetDocument> = serde_json::from_slice(&body_bytes)?;
                for batch_get_doc in batch_get_docs.into_iter() {
                    match batch_get_doc {
                        BatchGetDocument::Found { found } => {
                            let doc_name = found.name.clone();
                            ret.insert(doc_name, Some(found));
                        }
                        BatchGetDocument::Missing { missing } => {
                            ret.insert(missing, None);
                        }
                    }
                }
                Ok(ret)
            }
            _ => {
                error!("Non-success status code {} in batch_get", status);
                Err(storage::Error::Other(format!(
                    "Non-success status code {} in batch_get",
                    status,
                )))
            }
        }
    }

    async fn commit(&self, transaction: Transaction) -> storage::Result<()> {
        transaction.commit().await
    }

    async fn delete<T: TryFrom<Document>>(&self, name: &str) -> storage::Result<()> {
        let uri = format!("https://firestore.googleapis.com/v1/{}", name);
        let req = build_firestore_request::<()>(
            Method::DELETE,
            &uri,
            &*self._oauth_token.read().await,
            None,
        )
        .await?;
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
        let req = build_firestore_request(
            Method::POST,
            &uri,
            &*self._oauth_token.read().await,
            Some(&doc),
        )
        .await?;
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
            _ => {
                if let Ok(e) = serde_json::from_slice::<FirestoreErrorResponse>(&body_bytes) {
                    match e.status {
                        FirestoreErrorCode::ABORTED =>
                            Err(storage::Error::Transaction(
                                "Document contention, try again later".to_owned(),
                            )),
                        FirestoreErrorCode::ALREADY_EXISTS =>
                            Err(storage::Error::Conflict(format!(
                                "Could not create document with id {} under parent '{}' collection '{}' as it already exists",
                                document_id,
                                parent,
                                collection_id,
                            ))),
                        _ =>
                            Err(storage::Error::Other(format!(
                                "Error creating document with id {} under parent '{}' collection '{}': HTTP {} {}",
                                document_id,
                                parent,
                                collection_id,
                                status,
                                String::from_utf8(body_bytes.to_vec())
                                    .unwrap_or_else(|_| "<mangled body>".to_owned()),
                            ))),
                    }
                } else {
                    Err(storage::Error::Other(format!(
                        "Error creating document with id {} under parent '{}' collection '{}': HTTP {} {}",
                        document_id,
                        parent,
                        collection_id,
                        status,
                        String::from_utf8(body_bytes.to_vec())
                            .unwrap_or_else(|_| "<mangled body>".to_owned()),
                    )))
                }
            }
        }
    }

    async fn get<T: TryFrom<Document>>(
        &self,
        name: &str,
        transaction: Option<&Transaction>,
    ) -> storage::Result<Option<T>> {
        // Check transaction read cache
        if let Some(t) = transaction {
            if let Some(doc) = t.read_cache.read().await.get(name).cloned() {
                // Cache hit
                match doc.try_into() {
                    Ok(ret) => {
                        debug!("Transaction read cache hit for {}", name);
                        return Ok(Some(ret));
                    },
                    Err(_) => error!("Transaction read cache hit but failed to convert Document into requested type."),
                }
            }
        }

        let uri = match transaction {
            Some(t) => format!(
                "https://firestore.googleapis.com/v1/{}?transaction={}",
                name,
                percent_encoding::utf8_percent_encode(
                    &t.transaction_id,
                    percent_encoding::NON_ALPHANUMERIC
                )
                .to_string()
            ),
            None => format!("https://firestore.googleapis.com/v1/{}", name),
        };
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

                // Add to transaction read cache
                if let Some(t) = transaction {
                    t.cache_read(name.to_owned(), doc.clone()).await;
                }

                let result: Result<T, _> = doc.try_into();
                match result {
                    Ok(ret) => Ok(Some(ret)),
                    Err(_) => Err(storage::Error::Other(
                        "Failed to convert from Document to requested type.".to_owned(),
                    )),
                }
            }
            StatusCode::NOT_FOUND => Ok(None),
            _ => {
                error!("Non-success status code {} in get", status);
                Err(storage::Error::Other(format!(
                    "Non-success status code {} in get",
                    status
                )))
            }
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

            let req = build_firestore_request::<()>(
                Method::GET,
                &uri,
                &*self._oauth_token.read().await,
                None,
            )
            .await?;
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
                                Err(_) => {
                                    error!("Failed to convert from Document to requested type.");
                                }
                            };
                        }
                    }

                    next_page_token = list_response.next_page_token;
                }
                _ => {
                    error!("Non-success status code {} for list", status);
                    return Err(storage::Error::Other(format!(
                        "Non-success status code {} for list",
                        status
                    )));
                }
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
        let req = build_firestore_request(
            Method::PATCH,
            &uri,
            &*self._oauth_token.read().await,
            Some(&doc),
        )
        .await?;
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

    #[allow(dead_code)]
    #[allow(unused_variables)]
    async fn rollback(&self, transaction_id: String) -> storage::Result<()> {
        // TODO Firestore::rollback
        todo!()
    }

    #[allow(dead_code)]
    async fn run_query<T: TryFrom<Document>>(
        &self,
        _parent: &str,
        _structured_query: StructuredQuery,
    ) -> storage::Result<T> {
        // TODO Firestore::run_query
        todo!()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
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

    pub fn extract_double<T: From<f64> + From<i32> + Float>(
        &self,
        field_name: &str,
    ) -> Result<T, String> {
        if let Some(doc_field) = self.fields.get(field_name) {
            doc_field.extract_double()
        } else {
            Err(format!("Missing field {}", field_name))
        }
    }

    pub fn extract_integer<T: FromStr + Integer>(&self, field_name: &str) -> Result<T, String> {
        if let Some(doc_field) = self.fields.get(field_name) {
            doc_field.extract_integer()
        } else {
            Err(format!("Missing field {}", field_name))
        }
    }

    pub fn extract_string(&self, field_name: &str) -> Result<String, String> {
        if let Some(doc_field) = self.fields.get(field_name) {
            doc_field.extract_string()
        } else {
            Err(format!("Missing field {}", field_name))
        }
    }

    pub fn extract_timestamp(&self, field_name: &str) -> Result<DateTime<Utc>, String> {
        if let Some(doc_field) = self.fields.get(field_name) {
            doc_field.extract_timestamp()
        } else {
            Err(format!("Missing field {}", field_name))
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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

impl DocumentField {
    pub fn extract_double<T: From<f64> + From<i32> + Float>(&self) -> Result<T, String> {
        if let DocumentField::DoubleValue(ret) = self {
            Ok((*ret).into())
        } else if let DocumentField::IntegerValue(ret_str) = self {
            // Firestore is dumb and casts integral double values back into IntegerValues
            // Just use i32 here
            if let Ok(ret) = ret_str.parse::<i32>() {
                Ok(ret.into())
            } else {
                Err(format!(
                    "Error casting to {} from value {}",
                    type_name::<T>(),
                    ret_str
                ))
            }
        } else {
            Err(format!("Error parsing DoubleValue from {:?}", self))
        }
    }

    pub fn extract_integer<T: FromStr + Integer>(&self) -> Result<T, String> {
        if let DocumentField::IntegerValue(ret_str) = self {
            if let Ok(ret) = ret_str.parse() {
                Ok(ret)
            } else {
                Err(format!(
                    "Error casting to {} from value {}",
                    type_name::<T>(),
                    ret_str
                ))
            }
        } else {
            Err(format!("Error parsing IntegerValue from {:?}", self))
        }
    }

    pub fn extract_string(&self) -> Result<String, String> {
        if let DocumentField::StringValue(ret_str) = self {
            Ok(ret_str.to_string())
        } else {
            Err(format!("Error parsing StringValue from {:?}", self))
        }
    }

    pub fn extract_timestamp(&self) -> Result<DateTime<Utc>, String> {
        if let DocumentField::TimestampValue(dt) = self {
            Ok(*dt)
        } else {
            Err(format!("Error parsing TimestampValue from {:?}", self))
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentArrayValue {
    pub values: Option<Vec<DocumentField>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DocumentMapValue {
    pub fields: Option<HashMap<String, DocumentField>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListDocumentsResponse {
    documents: Option<Vec<Document>>,
    next_page_token: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BeginTransactionRequest {
    options: TransactionOptions,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BeginTransactionResponse {
    transaction: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommitRequest {
    writes: Vec<Write>,
    transaction: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RollbackRequest {
    transaction: String,
}

#[derive(Debug, Serialize)]
#[serde(untagged, rename_all = "camelCase")]
enum Write {
    Update { update: Document },
    Delete { delete: String },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BatchGetRequest {
    documents: Vec<String>,
    transaction: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged, rename_all = "camelCase")]
enum BatchGetDocument {
    Found { found: Document },
    Missing { missing: String },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunQueryResponse {
    transaction: Option<String>,
    document: Option<Document>,
    read_time: Option<String>,
    skipped_results: Option<u32>,
}

// TODO
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
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
struct FirestoreErrorResponse {
    code: u32,
    message: String,
    status: FirestoreErrorCode,
}

/// Canonical error codes from https://firebase.google.com/docs/firestore/use-rest-api#error_codes
#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
enum FirestoreErrorCode {
    ABORTED,
    ALREADY_EXISTS,
    DEADLINE_EXCEEDED,
    FAILED_PRECONDITION,
    INTERNAL,
    INVALID_ARGUMENT,
    NOT_FOUND,
    PERMISSION_DENIED,
    RESOURCE_EXHAUTED,
    UNAUTHENTICATED,
    UNAVAILABLE,
}

pub struct Transaction {
    database: String,
    drop_tx: mpsc::Sender<(String, String)>,
    http_client: Arc<Client<HttpsConnector<HttpConnector>>>,
    oauth_token: Arc<RwLock<String>>,
    read_cache: Arc<RwLock<HashMap<String, Document>>>,
    transaction_id: String,
    writes: Arc<std::sync::Mutex<Option<Vec<Write>>>>,
}

impl Transaction {
    fn new(
        database: String,
        drop_tx: mpsc::Sender<(String, String)>,
        http_client: Arc<Client<HttpsConnector<HttpConnector>>>,
        oauth_token: Arc<RwLock<String>>,
        id: String,
    ) -> Transaction {
        Transaction {
            database,
            drop_tx,
            http_client,
            oauth_token,
            read_cache: Arc::new(RwLock::new(HashMap::new())),
            transaction_id: id,
            writes: Arc::new(std::sync::Mutex::new(Some(vec![]))),
        }
    }

    pub async fn abort(mut self) {
        let mut mutex_guard = self.writes.lock().expect("Poisoned lock");
        if let None = mutex_guard.take() {
            warn!("Attempted to abort invalid transaction");
        } else {
            let database = self.database.clone();
            let id = self.transaction_id.clone();
            if let Err(err) = self.drop_tx.send((database, id)).await {
                error!("Failed to abort transaction. Error: {}", err);
            };
        }
    }

    fn blocking_abort(
        mut drop_tx: mpsc::Sender<(String, String)>,
        database: String,
        transaction_id: String,
    ) {
        if let Err(err) = drop_tx.try_send((database, transaction_id)) {
            error!(
                "Failed to abort transaction with blocking abort. Error: {}",
                err
            );
        };
    }

    async fn append_write(&self, write: Write) -> Result<(), TransactionError> {
        let mut mutex_guard = self.writes.lock().expect("Poisoned lock");
        match mutex_guard.as_mut() {
            Some(v) => {
                v.push(write);
                Ok(())
            }
            None => Err(TransactionError::InvalidState),
        }
    }

    pub async fn commit(self) -> storage::Result<()> {
        let database = self.database.clone();
        let http_client = Arc::clone(&self.http_client);
        let oauth_token = Arc::clone(&self.oauth_token);
        let transaction_id = self.transaction_id.clone();
        let writes;
        match self.try_into_writes().await {
            Ok(w) => writes = w,
            Err(TransactionError::InvalidState) => {
                warn!("Attempted to commit a transaction that is in an invalid state.");
                return Ok(());
            }
        }
        let body = CommitRequest {
            writes,
            transaction: transaction_id,
        };

        Transaction::commit_internal(database, http_client, oauth_token, body).await
    }

    async fn commit_internal(
        database: String,
        http_client: Arc<Client<HttpsConnector<HttpConnector>>>,
        oauth_token: Arc<RwLock<String>>,
        request_body: CommitRequest,
    ) -> storage::Result<()> {
        let uri = format!("https://firestore.googleapis.com/v1/{}:commit", database);
        debug!("{}", serde_json::to_string_pretty(&request_body)?);
        let req = build_firestore_request(
            Method::POST,
            &uri,
            &*oauth_token.read().await,
            Some(&request_body),
        )
        .await?;
        debug!("POST {} {:?}", uri, req);
        let resp = http_client.request(req).await?;
        let status = resp.status();
        let body_bytes = body::to_bytes(resp.into_body()).await.unwrap_or_default();
        debug!(
            "HTTP {} {}",
            status,
            String::from_utf8(body_bytes.to_vec()).unwrap_or_else(|_| "<mangled body>".to_owned()),
        );
        match status {
            StatusCode::OK => {
                // TODO interpret write results
                Ok(())
            }
            StatusCode::CONFLICT => {
                // Write contention
                Err(storage::Error::Transaction(
                    "Document contention, try again later".to_owned(),
                ))
            }
            _ => {
                error!("Non-success status code {} in commit", status);
                Err(storage::Error::Other(format!(
                    "Non-success status code {} in commit",
                    status
                )))
            }
        }
    }

    async fn cache_read(&self, name: String, doc: Document) {
        let mut write_guard = self.read_cache.write().await;
        write_guard.insert(name, doc);
    }

    async fn try_into_writes(self) -> Result<Vec<Write>, TransactionError> {
        let mut mutex_guard = self.writes.lock().expect("Poisoned lock");
        mutex_guard.take().ok_or(TransactionError::InvalidState)
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        let mut mutex_guard = self.writes.lock().expect("Poisoned lock");
        if let Some(_) = mutex_guard.take() {
            info!(
                "Transaction {} is being dropped without an explicit abort or commit. Running an abort to clear the transaction",
                self.transaction_id
            );

            Transaction::blocking_abort(
                self.drop_tx.clone(),
                self.database.clone(),
                self.transaction_id.clone(),
            );
        }
    }
}

#[derive(Debug)]
pub enum TransactionType {
    ReadOnly,
    ReadWrite,
}

#[derive(Debug)]
enum TransactionError {
    InvalidState,
}

impl Into<storage::Error> for TransactionError {
    fn into(self) -> storage::Error {
        storage::Error::Other(format!("{:?}", self))
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum TransactionOptions {
    ReadOnly(ReadOnlyTransactionOptions),
    ReadWrite(ReadWriteTransactionOptions),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadOnlyTransactionOptions {
    read_time: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadWriteTransactionOptions {
    retry_transaction: Option<String>,
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

/// OpenID Connect claims data structure
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    /// Email address of the service account
    iss: String,
    /// Space-delimited list of the permissions requested
    scope: String,
    /// Intended target of assertion, should just be https://oauth2.googleapis.com/token
    aud: String,
    /// Expiration time of the assertion, as seconds since epoch. Maximum of 1 hour after issuance
    exp: usize,
    /// Assertion issuance time, as seconds since epoch
    iat: usize,
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
    let response = http_client.request(request).await?;
    let status = response.status();
    let body_bytes = body::to_bytes(response.into_body())
        .await
        .unwrap_or_default();

    match status {
        StatusCode::OK => {
            let body: OAuth2Response = serde_json::from_slice(&body_bytes)?;
            debug!("Response: {} {:?}", status, body);
            info!("Obtained OAuth token, took {:?}", sw.elapsed());
            Ok((body.access_token, body.expires_in))
        }
        _ => {
            let body_str = String::from_utf8(body_bytes.to_vec())
                .unwrap_or_else(|_| "<mangled body>".to_owned());
            info!("Failed to obtain OAuth token, took {:?}", sw.elapsed());
            Err(storage::Error::OAuth(format!(
                "OAuth flow returned HTTP {} with body content: {}",
                status, body_str
            )))
        }
    }
}

async fn build_firestore_request<T>(
    method: Method,
    uri: &String,
    auth_token: &String,
    body: Option<&T>,
) -> storage::Result<Request<Body>>
where
    T: Sized + Serialize,
{
    let b = Request::builder()
        .method(method)
        .uri(uri)
        .header(HeaderName::from_static("accept"), "application/json")
        .header(
            HeaderName::from_static("authorization"),
            format!("Bearer {}", auth_token),
        );

    let body = match body {
        None => Body::empty(),
        Some(b) => Body::from(serde_json::to_string(b)?),
    };

    let req = b.body(body).unwrap();

    Ok(req)
}

#[cfg(test)]
mod tests {
    extern crate env_logger;
    use super::*;

    fn logging_init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    static FAKE_JSON_KEY_PATH: &str = "fake_service_account.json";

    #[tokio::test(threaded_scheduler)]
    async fn can_read_key_from_json() {
        logging_init();

        match read_json_key(FAKE_JSON_KEY_PATH).await {
            Ok(_) => assert!(true),
            Err(e) => assert!(false, format!("{:?}", e)),
        }
    }

    #[tokio::test(threaded_scheduler)]
    async fn can_build_jwt() {
        logging_init();

        let key = read_json_key(FAKE_JSON_KEY_PATH).await.unwrap();
        match build_jwt(&key.client_email, &key.private_key).await {
            Ok(_) => assert!(true),
            Err(e) => assert!(false, format!("{:?}", e)),
        }
    }

    #[cfg(feature = "test_requires_secrets")]
    static JSON_KEY_PATH: &str = "../secrets/service_account.json";

    #[cfg(feature = "test_requires_secrets")]
    #[tokio::test(threaded_scheduler)]
    async fn can_get_oauth_token() {
        logging_init();

        let key = read_json_key(JSON_KEY_PATH).await.unwrap();
        let jwt = build_jwt(&key.client_email, &key.private_key)
            .await
            .unwrap();

        let mut https = HttpsConnector::new();
        https.https_only(true);
        let client = Client::builder().build::<_, hyper::Body>(https);

        get_oauth_token(jwt, &client).await.unwrap();
    }
}
