extern crate env_logger;
#[macro_use]
extern crate log;

use pccg_rs_engine::{constants, job_board::JobBoard, Api};
use pccg_rs_storage::firestore::{Firestore, FirestoreClient};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

static JSON_KEY_PATH: &str = "../secrets/service_account.json";

fn logging_init() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info");
    }
    let _ = env_logger::builder().is_test(true).try_init();
}

#[tokio::test(threaded_scheduler)]
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

    tokio::time::delay_for(Duration::from_secs(2)).await;

    // Add a new user
    info!("[claim_daily_increases_currency_once] Deleting and adding new user");
    let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
    api.delete_user(&user_id).await.unwrap();
    tokio::time::delay_for(Duration::from_secs(2)).await;
    api.add_user(&user_id).await.unwrap();

    // Save the starting currency amount
    info!("[claim_daily_increases_currency_once] Fetching starting currency amount");
    let user = api.get_user(&user_id).await.unwrap().unwrap();
    let starting_currency = user.currency;

    tokio::time::delay_for(Duration::from_secs(2)).await;

    // Claim daily first time
    info!("[claim_daily_increases_currency_once] Claming daily once");
    let ret = api.claim_user_daily_reward(&user_id).await;

    tokio::time::delay_for(Duration::from_secs(2)).await;

    // Claim daily second time
    info!("[claim_daily_increases_currency_once] Claming daily twice");
    let ret2 = api.claim_user_daily_reward(&user_id).await;

    tokio::time::delay_for(Duration::from_secs(2)).await;

    // Fetch the updated currency amount
    info!("[claim_daily_increases_currency_once] Fetching updated curency amount");
    let user = api.get_user(&user_id).await.unwrap().unwrap();

    // Assert that the currency amount increased once
    info!("[claim_daily_increases_currency_once] Running assertions");
    assert!(ret.is_ok());
    assert!(ret2.is_err());
    assert!(user.currency > starting_currency);
    assert_eq!(
        user.currency - starting_currency,
        constants::DAILY_CURRENCY_REWARD
    );
}
