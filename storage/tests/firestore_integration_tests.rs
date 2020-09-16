extern crate env_logger;
extern crate pccg_rs_storage;

use pccg_rs_storage::firestore::*;
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    sync::Arc,
};
use uuid::Uuid;

static JSON_KEY_PATH: &str = "../secrets/service_account.json";

fn logging_init() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info");
    }
    let _ = env_logger::builder().is_test(true).try_init();
}

#[tokio::test(threaded_scheduler)]
async fn can_upsert_then_get() {
    logging_init();

    let firestore = Firestore::new(JSON_KEY_PATH).await.unwrap();
    let firestore = FirestoreClient::new(Arc::new(firestore), None, "_test".to_owned());
    let id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
    let test_item = TestItem {
        id,
        number: 0,
        test_case: "can_upsert_then_get".to_owned(),
    };
    firestore
        .upsert(&id, test_item.clone(), None)
        .await
        .unwrap();
    let ret = firestore.get::<TestItem>(&id, None).await.unwrap().unwrap();
    assert_eq!(ret, test_item);
}

#[tokio::test(threaded_scheduler)]
async fn can_list_empty_collection() {
    logging_init();

    let firestore = Firestore::new(JSON_KEY_PATH).await.unwrap();
    let firestore = FirestoreClient::new(
        Arc::new(firestore),
        None,
        "_test_list_empty_collection".to_owned(),
    );
    let ret = firestore.list::<TestItem>().await.unwrap();
    assert_eq!(ret, vec![]);
}

#[tokio::test(threaded_scheduler)]
async fn can_list_non_empty_collection() {
    logging_init();

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
        test_case: "can_list_non_empty_collection".to_owned(),
    };
    firestore
        .upsert(&id, test_item.clone(), None)
        .await
        .unwrap();
    let ret = firestore.list::<TestItem>().await.unwrap();
    assert_eq!(ret.len(), 1);
    assert_eq!(ret[0], test_item);
}

#[tokio::test(threaded_scheduler)]
async fn can_list_empty_subcollection() {
    logging_init();

    let firestore = Firestore::new(JSON_KEY_PATH).await.unwrap();
    let firestore = FirestoreClient::new(Arc::new(firestore), None, "_test".to_owned());
    let id = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
    let test_item = TestItem {
        id,
        number: 2,
        test_case: "can_list_empty_subcollection".to_owned(),
    };
    firestore.upsert(&id, test_item, None).await.unwrap();
    let sub_fs =
        FirestoreClient::new_for_subcollection(&firestore, id.to_string(), "test".to_owned());
    let ret = sub_fs.list::<TestItem>().await.unwrap();
    assert_eq!(ret, vec![]);
}

#[tokio::test(threaded_scheduler)]
async fn can_list_non_empty_subcollection() {
    logging_init();

    let firestore = Firestore::new(JSON_KEY_PATH).await.unwrap();
    let firestore = FirestoreClient::new(Arc::new(firestore), None, "_test".to_owned());
    let id = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();
    let test_item = TestItem {
        id,
        number: 3,
        test_case: "can_list_non_empty_subcollection".to_owned(),
    };
    firestore
        .upsert(&id, test_item.clone(), None)
        .await
        .unwrap();
    let sub_fs =
        FirestoreClient::new_for_subcollection(&firestore, id.to_string(), "test".to_owned());
    sub_fs.upsert(&id, test_item.clone(), None).await.unwrap();
    let ret = sub_fs.list::<TestItem>().await.unwrap();
    assert_eq!(ret.len(), 1);
    assert_eq!(ret[0], test_item);
}

#[tokio::test(threaded_scheduler)]
async fn can_upsert_then_batch_get() {
    logging_init();

    let firestore = Firestore::new(JSON_KEY_PATH).await.unwrap();
    let firestore = FirestoreClient::new(Arc::new(firestore), None, "_test".to_owned());

    let id_1 = Uuid::parse_str("00000000-0000-0000-0000-000000000004").unwrap();
    let test_item_1 = TestItem {
        id: id_1,
        number: 0,
        test_case: "can_upsert_then_batch_get".to_owned(),
    };
    firestore
        .upsert(&id_1, test_item_1.clone(), None)
        .await
        .unwrap();

    let id_2 = Uuid::parse_str("00000000-0000-0000-0000-000000000005").unwrap();
    let test_item_2 = TestItem {
        id: id_2,
        number: 0,
        test_case: "can_upsert_then_batch_get".to_owned(),
    };
    firestore
        .upsert(&id_2, test_item_2.clone(), None)
        .await
        .unwrap();

    let id_3 = Uuid::parse_str("99999999-9999-9999-9999-999999999999").unwrap();

    let ret = firestore
        .batch_get::<TestItem>(&vec![id_1, id_2, id_3], None)
        .await
        .unwrap();
    assert_eq!(ret.len(), 3);
    assert!(ret.contains_key(&id_1));
    assert_ne!(ret[&id_1], None);
    assert_eq!(ret[&id_2], Some(test_item_2));
    assert!(ret.contains_key(&id_2));
    assert_ne!(ret[&id_2], None);
    assert_eq!(ret[&id_1], Some(test_item_1));
    assert!(ret.contains_key(&id_3));
    assert_eq!(ret[&id_3], None);
}

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
                    return Err(format!(
                        "Could not convert Document to TestItem: error parsing field 'number': {}",
                        e
                    ));
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
    logging_init();

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
