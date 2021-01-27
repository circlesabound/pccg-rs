extern crate env_logger;
#[macro_use]
extern crate log;

use pccg_rs_engine::{constants, job_board::JobBoard, Api};
use pccg_rs_storage::firestore::{Firestore, FirestoreClient};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

static JSON_KEY_PATH: &str = "../secrets/service_account.json";
static UUID_NAMESPACE: &str = "6e81479f-5718-4d5c-aab7-6eb6de4465c2";

fn logging_init() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info");
    }
    let _ = env_logger::builder().is_test(true).try_init();
}

fn generate_uuid(string: &str) -> Uuid {
    let namespace = Uuid::parse_str(UUID_NAMESPACE).unwrap();
    Uuid::new_v5(&namespace, string.as_bytes())
}

async fn recreate_user(api: Arc<Api>, user_id: &Uuid) {
    if let Err(e) = api.delete_user(user_id).await {
        if let pccg_rs_engine::ErrorCode::UserNotFound = e.code {
            // Deleting user failed because it did not exist to begin with. This is fine
        } else {
            assert!(
                false,
                "Unexpected error when deleting user to recreate: {:?}",
                e
            );
        }
    }

    tokio::time::sleep(Duration::from_secs(2)).await;

    if let Err(e) = api.add_user(&user_id).await {
        assert!(false, "Unexpected error when recreating user: {:?}", e);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn claim_daily_increases_currency_once() {
    logging_init();

    let fs = Arc::new(Firestore::new(JSON_KEY_PATH).await.unwrap());
    let cards = FirestoreClient::new(Arc::clone(&fs), None, "_test_cards".to_owned());
    let users = FirestoreClient::new(Arc::clone(&fs), None, "_test_users".to_owned());
    let job_board = JobBoard::new(FirestoreClient::new(
        Arc::clone(&fs),
        None,
        "_test_jobs".to_owned(),
    ))
    .await;
    let api = Arc::new(Api::new(cards, job_board, users).await);

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Add a new user
    info!(
        "[{}] Deleting and adding new user",
        stringify!(claim_daily_increases_currency_once)
    );
    let user_id = generate_uuid(stringify!(claim_daily_increases_currency_once));
    recreate_user(Arc::clone(&api), &user_id).await;

    // Save the starting currency amount
    info!(
        "[{}] Fetching starting currency amount",
        stringify!(claim_daily_increases_currency_once)
    );
    let user = api.get_user(&user_id).await.unwrap().unwrap();
    let starting_currency = user.currency;

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Claim daily first time
    info!(
        "[{}] Claming daily once",
        stringify!(claim_daily_increases_currency_once)
    );
    let ret = api.claim_user_daily_reward(&user_id).await;

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Claim daily second time
    info!(
        "[{}] Claming daily twice",
        stringify!(claim_daily_increases_currency_once)
    );
    let ret2 = api.claim_user_daily_reward(&user_id).await;

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Fetch the updated currency amount
    info!(
        "[{}] Fetching updated curency amount",
        stringify!(claim_daily_increases_currency_once)
    );
    let user = api.get_user(&user_id).await.unwrap().unwrap();

    // Assert that the currency amount increased once
    info!(
        "[{}] Running assertions",
        stringify!(claim_daily_increases_currency_once)
    );
    assert!(ret.is_ok());
    assert!(ret2.is_err());
    assert!(user.currency > starting_currency);
    assert_eq!(
        user.currency - starting_currency,
        constants::DAILY_CURRENCY_REWARD
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn can_complete_finished_job() {
    logging_init();

    let fs = Arc::new(Firestore::new(JSON_KEY_PATH).await.unwrap());
    let cards = FirestoreClient::new(Arc::clone(&fs), None, "_test_cards".to_owned());
    let users = FirestoreClient::new(Arc::clone(&fs), None, "_test_users".to_owned());
    let job_board = JobBoard::new(FirestoreClient::new(
        Arc::clone(&fs),
        None,
        "_test_jobs".to_owned(),
    ))
    .await;
    let api = Arc::new(Api::new(cards, job_board, users).await);

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Add a new user
    info!(
        "[{}] Deleting and adding new user",
        stringify!(can_complete_finished_job)
    );
    let user_id = generate_uuid(stringify!(can_complete_finished_job));
    recreate_user(Arc::clone(&api), &user_id).await;

    // TODO
}

#[tokio::test(flavor = "multi_thread")]
async fn cannot_complete_unfinished_job() {
    logging_init();

    let fs = Arc::new(Firestore::new(JSON_KEY_PATH).await.unwrap());
    let cards = FirestoreClient::new(Arc::clone(&fs), None, "_test_cards".to_owned());
    let users = FirestoreClient::new(Arc::clone(&fs), None, "_test_users".to_owned());
    let job_board = JobBoard::new(FirestoreClient::new(
        Arc::clone(&fs),
        None,
        "_test_jobs".to_owned(),
    ))
    .await;
    let api = Arc::new(Api::new(cards, job_board, users).await);

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Add a new user
    info!(
        "[{}] Deleting and adding new user",
        stringify!(cannot_complete_unfinished_job)
    );
    let user_id = generate_uuid(stringify!(cannot_complete_unfinished_job));
    recreate_user(Arc::clone(&api), &user_id).await;

    // TODO
}
