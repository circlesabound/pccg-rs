use chrono::{DateTime, TimeZone, Utc};
use pccg_rs_storage::firestore::{Document, DocumentField};
use std::{collections::HashMap, convert::TryFrom};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct User {
    pub id: Uuid,
    pub currency: u32,
    pub daily_last_claimed: DateTime<Utc>,
    pub staged_card: Option<Uuid>,
}

impl User {
    pub fn new(id: Uuid) -> User {
        User {
            id,
            currency: 0,
            daily_last_claimed: Utc.timestamp(0, 0),
            staged_card: None,
        }
    }
}

impl TryFrom<Document> for User {
    type Error = &'static str;

    fn try_from(value: Document) -> Result<Self, Self::Error> {
        let id = value.name.split('/').next_back().unwrap();
        if let Some(DocumentField::IntegerValue(currency)) = value.fields.get("currency") {
            if let Ok(currency) = currency.parse() {
                if let Some(DocumentField::TimestampValue(daily_last_claimed)) =
                    value.fields.get("daily_last_claimed")
                {
                    let staged_card = match value.fields.get("staged_card") {
                        Some(DocumentField::StringValue(staged_id_str)) => {
                            Some(Uuid::parse_str(staged_id_str).unwrap())
                        }
                        _ => None,
                    };

                    return Ok(User {
                        id: Uuid::parse_str(id).unwrap(),
                        currency,
                        daily_last_claimed: *daily_last_claimed,
                        staged_card,
                    });
                }
            }
        }

        Err("Could not convert Document to User")
    }
}

impl Into<Document> for User {
    fn into(self) -> Document {
        let mut fields = HashMap::new();
        fields.insert(
            "currency".to_owned(),
            DocumentField::IntegerValue(self.currency.to_string()),
        );
        fields.insert(
            "daily_last_claimed".to_owned(),
            DocumentField::TimestampValue(self.daily_last_claimed),
        );
        if let Some(staged_card_id) = self.staged_card {
            fields.insert(
                "staged_card".to_owned(),
                DocumentField::StringValue(staged_card_id.to_string()),
            );
        }
        Document::new(fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn can_convert_between_document_and_user() {
        let user = User::new(Uuid::new_v4());

        let user_clone = user.clone();
        let mut doc: Document = user_clone.into();
        doc.name = format!("parent_path/{}", user.id.to_string());

        let user_from_doc: User = doc.try_into().unwrap();

        assert_eq!(user, user_from_doc);
    }
}
