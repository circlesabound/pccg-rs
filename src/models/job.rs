use super::stats::StatsF;
use crate::storage::firestore::{Document, DocumentArrayValue, DocumentField};
use chrono::{DateTime, Utc};
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    hash::Hash,
};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Job {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub recommended_stats: StatsF,
    pub completion_time: DateTime<Utc>,

    pub user_id: Uuid,
    pub character_ids: Vec<Uuid>,
}

impl Job {
    pub fn new(prototype: &JobPrototype, user_id: Uuid, character_ids: Vec<Uuid>) -> Job {
        Job {
            id: Uuid::new_v4(),
            name: prototype.name.clone(),
            description: prototype.description.clone(),
            recommended_stats: prototype.recommended_stats.clone(),
            completion_time: Utc::now() + chrono::Duration::minutes(prototype.duration_mins as i64),
            user_id,
            character_ids,
        }
    }

    pub fn can_complete(&self) -> bool {
        self.completion_time > Utc::now()
    }
}

impl TryFrom<Document> for Job {
    type Error = String;

    fn try_from(value: Document) -> Result<Self, Self::Error> {
        let id = value.extract_id()?;
        let name = value.extract_string("name")?;
        let description = value.extract_string("description")?;
        let recommended_stats = value.fields.get("recommended_stats")
            .ok_or_else(|| format!("Missing field 'recommended_stats'"))?
            .try_into()?;
        let completion_time = value.extract_timestamp("completion_time")?;

        let user_id_str = value.extract_string("user_id")?;
        let user_id = Uuid::parse_str(&user_id_str).unwrap();
        let character_ids = match value.fields.get("character_ids")
            .ok_or_else(|| format!("Missing field 'character_ids"))? {
            DocumentField::ArrayValue(dav) => {
                dav.values.as_ref().unwrap()
                    .into_iter()
                    .map(|df| Uuid::parse_str(&df.extract_string().unwrap()).unwrap())
                    .collect()
            },
            df => return Err(format!("Error parsing ArrayValue from {:?}", df))
        };

        Ok(Job {
            id,
            name,
            description,
            recommended_stats,
            completion_time,
            user_id,
            character_ids,
        })
    }
}

impl Into<Document> for Job {
    fn into(self) -> Document {
        let mut fields = HashMap::new();
        fields.insert("name".to_owned(), DocumentField::StringValue(self.name));
        fields.insert(
            "description".to_owned(),
            DocumentField::StringValue(self.description),
        );
        fields.insert(
            "recommended_stats".to_owned(),
            self.recommended_stats.into(),
        );
        fields.insert(
            "completion_time".to_owned(),
            DocumentField::TimestampValue(self.completion_time),
        );
        fields.insert("user_id".to_owned(), DocumentField::StringValue(self.user_id.to_string()));
        fields.insert(
            "character_ids".to_owned(),
            DocumentField::ArrayValue(DocumentArrayValue {
                values: Some(self.character_ids.into_iter().map(|id| DocumentField::StringValue(id.to_string())).collect())
            }),
        );
        Document::new(fields)
    }
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct JobPrototype {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub recommended_stats: StatsF,
    pub duration_mins: u32,
}

impl Hash for JobPrototype {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl Eq for JobPrototype {}

impl TryFrom<Document> for JobPrototype {
    type Error = String;

    fn try_from(value: Document) -> Result<Self, Self::Error> {
        let id = value.extract_id()?;
        let name = value.extract_string("name")?;
        let description = value.extract_string("description")?;
        let recommended_stats = value.fields.get("recommended_stats")
            .ok_or_else(|| format!("Missing field 'recommended_stats'"))?
            .try_into()?;
        let duration_mins = value.extract_integer("duration_mins")?;

        Ok(JobPrototype {
            id,
            name,
            description,
            recommended_stats,
            duration_mins,
        })
    }
}

impl Into<Document> for JobPrototype {
    fn into(self) -> Document {
        let mut fields = HashMap::new();
        fields.insert("name".to_owned(), DocumentField::StringValue(self.name));
        fields.insert(
            "description".to_owned(),
            DocumentField::StringValue(self.description),
        );
        fields.insert(
            "recommended_stats".to_owned(),
            self.recommended_stats.into(),
        );
        fields.insert(
            "duration_mins".to_owned(),
            DocumentField::IntegerValue(self.duration_mins.to_string()),
        );
        Document::new(fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_convert_between_document_and_job() {
        let job = Job {
            id: Uuid::new_v4(),
            name: "test job".to_owned(),
            description: "test description".to_owned(),
            recommended_stats: StatsF {
                physical: 15.2,
                mental: 29.5,
                tactical: 8.0,
            },
            completion_time: chrono::Utc::now(),
            user_id: Uuid::new_v4(),
            character_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
        };

        let mut doc: Document = job.clone().into();
        doc.name = format!("parent_path/{}", job.id.to_string());

        let from_doc: Job = doc.try_into().unwrap();

        assert_eq!(job, from_doc);
    }

    #[test]
    fn can_convert_between_document_and_job_prototype() {
        let job_prototype = JobPrototype {
            id: Uuid::new_v4(),
            name: "test job".to_owned(),
            description: "test description".to_owned(),
            recommended_stats: StatsF {
                physical: 15.2,
                mental: 29.5,
                tactical: 8.0,
            },
            duration_mins: 3600,
        };

        let mut doc: Document = job_prototype.clone().into();
        doc.name = format!("parent_path/{}", job_prototype.id.to_string());

        let from_doc: JobPrototype = doc.try_into().unwrap();

        assert_eq!(job_prototype, from_doc);
    }
}
