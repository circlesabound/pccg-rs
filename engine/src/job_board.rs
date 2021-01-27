use crate as engine;
use chrono::Utc;
use dashmap::DashMap;
use engine::ErrorCode;
use pccg_rs_models::{Job, JobPrototype};
use pccg_rs_storage::firestore::FirestoreClient;
use rand::{rngs::StdRng, SeedableRng};
use std::sync::Arc;
use tokio::{
    sync::{
        oneshot::{self, error::TryRecvError},
        Mutex,
    },
    task,
    time::Duration,
};
use uuid::Uuid;

pub struct JobBoard {
    available_jobs_cache: Arc<DashMap<JobTier, Vec<JobPrototype>>>,
    _refresh_jobs_cancellation: oneshot::Sender<()>,
    _refresh_jobs_handle: task::JoinHandle<()>,
    _refresh_jobs_last_checked: Arc<Mutex<chrono::Date<Utc>>>,
}

impl JobBoard {
    pub async fn new(prototypes_client: FirestoreClient) -> JobBoard {
        let available_jobs_cache = Arc::new(DashMap::new());

        let _refresh_jobs_last_checked = Arc::new(Mutex::new(chrono::MIN_DATE));
        let (_refresh_jobs_cancellation, mut rx) = oneshot::channel();
        let available_jobs_cache_clone = Arc::clone(&available_jobs_cache);
        let _refresh_jobs_last_checked_clone = Arc::clone(&_refresh_jobs_last_checked);
        let _refresh_jobs_handle = tokio::spawn(async move {
            while let Err(TryRecvError::Empty) = rx.try_recv() {
                let current_date = chrono::Utc::now().date();
                let mut _refresh_jobs_last_checked_clone =
                    _refresh_jobs_last_checked_clone.lock().await;
                // Refresh on day roll over
                if *_refresh_jobs_last_checked_clone < current_date {
                    info!("Generating jobs for {}", current_date);
                    if let Err(e) = JobBoard::generate_jobs(
                        Arc::clone(&available_jobs_cache_clone),
                        &current_date,
                        &prototypes_client,
                    )
                    .await
                    {
                        error!("Error generating jobs for {}: {:?}", current_date, e);
                    } else {
                        *_refresh_jobs_last_checked_clone = current_date;
                    }
                } else {
                    debug!("Jobs already generated for {}", current_date);
                }
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });

        JobBoard {
            available_jobs_cache,
            _refresh_jobs_cancellation,
            _refresh_jobs_handle,
            _refresh_jobs_last_checked,
        }
    }

    pub async fn create_job(
        &self,
        prototype_id: &Uuid,
        user_id: Uuid,
        character_ids: Vec<Uuid>,
    ) -> engine::Result<Job> {
        let beginner = self.available_jobs_cache.get(&JobTier::Beginner).unwrap();
        let intermediate = self
            .available_jobs_cache
            .get(&JobTier::Intermediate)
            .unwrap();
        let expert = self.available_jobs_cache.get(&JobTier::Expert).unwrap();

        match beginner
            .iter()
            .chain(intermediate.iter())
            .chain(expert.iter())
            .find(|p| p.id == *prototype_id)
        {
            Some(prototype) => {
                let job = Job::new(prototype, user_id, character_ids);
                Ok(job)
            }
            None => Err(engine::Error::new(ErrorCode::JobNotFound, None)),
        }
    }

    pub async fn list_available_jobs(&self, tier: &JobTier) -> Vec<JobPrototype> {
        match self.available_jobs_cache.get(tier) {
            Some(jobs) => (*jobs).clone(),
            None => {
                warn!("No available jobs!");
                vec![]
            }
        }
    }

    async fn generate_jobs(
        cache: Arc<DashMap<JobTier, Vec<JobPrototype>>>,
        date: &chrono::Date<Utc>,
        prototypes_client: &FirestoreClient,
    ) -> engine::Result<()> {
        let days_since_epoch = (*date - chrono::MIN_DATE).num_days();
        let _rng: StdRng = SeedableRng::seed_from_u64(days_since_epoch as u64);

        // TODO only offer a subset of all jobs, based on the day
        let beginner_jobs =
            JobBoard::get_job_prototypes(prototypes_client, JobTier::Beginner).await?;
        let intermediate_jobs =
            JobBoard::get_job_prototypes(prototypes_client, JobTier::Intermediate).await?;
        let expert_jobs = JobBoard::get_job_prototypes(prototypes_client, JobTier::Expert).await?;

        cache.insert(JobTier::Beginner, beginner_jobs);
        cache.insert(JobTier::Intermediate, intermediate_jobs);
        cache.insert(JobTier::Expert, expert_jobs);

        Ok(())
    }

    async fn get_job_prototypes(
        client: &FirestoreClient,
        tier: JobTier,
    ) -> engine::Result<Vec<JobPrototype>> {
        let subcollection_relative_path = match tier {
            JobTier::Beginner => "beginner",
            JobTier::Intermediate => "intermediate",
            JobTier::Expert => "expert",
        }
        .to_owned();

        let client = FirestoreClient::new_for_subcollection(
            client,
            subcollection_relative_path,
            "prototypes".to_owned(),
        );

        Ok(client.list::<JobPrototype>().await?)
    }
}

#[derive(Eq, Hash, PartialEq)]
pub enum JobTier {
    Beginner,
    Intermediate,
    Expert,
}
