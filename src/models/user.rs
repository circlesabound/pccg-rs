use crate::storage::firestore::{Document, DocumentArrayValue, DocumentField};
use chrono::{DateTime, TimeZone, Utc};
use std::{collections::HashMap, convert::TryFrom};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct User {
    pub id: Uuid,
    pub cards: Vec<Uuid>,
    pub currency: u32,
    pub daily_last_claimed: DateTime<Utc>,
}

impl User {
    pub fn new(id: Uuid) -> User {
        User {
            id,
            cards: vec![],
            currency: 0,
            daily_last_claimed: Utc.timestamp(0, 0),
        }
    }
}

impl TryFrom<Document> for User {
    type Error = &'static str;

    fn try_from(value: Document) -> Result<Self, Self::Error> {
        let id = value.name.split('/').next_back().unwrap();
        if let Some(DocumentField::ArrayValue(arr_opt)) = value.fields.get("cards") {
            if let Some(DocumentField::IntegerValue(currency)) = value.fields.get("currency") {
                if let Ok(currency) = currency.parse() {
                    if let Some(DocumentField::TimestampValue(daily_last_claimed)) =
                        value.fields.get("daily_last_claimed")
                    {
                        let mut card_ids: Vec<Uuid> = vec![];

                        if let Some(arr) = &arr_opt.values {
                            for doc_field in arr {
                                if let DocumentField::StringValue(id_str) = doc_field {
                                    match Uuid::parse_str(&id_str) {
                                        Ok(uuid) => card_ids.push(uuid),
                                        Err(_) => return Err("Could not convert Document to User"),
                                    };
                                } else {
                                    return Err("Could not convert Document to User");
                                }
                            }
                        }

                        return Ok(User {
                            id: Uuid::parse_str(id).unwrap(),
                            cards: card_ids,
                            currency,
                            daily_last_claimed: *daily_last_claimed,
                        });
                    }
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
            "cards".to_owned(),
            DocumentField::ArrayValue(DocumentArrayValue {
                values: Some(
                    self.cards
                        .into_iter()
                        .map(|id| DocumentField::StringValue(id.to_string()))
                        .collect(),
                ),
            }),
        );
        fields.insert(
            "currency".to_owned(),
            DocumentField::IntegerValue(self.currency.to_string()),
        );
        fields.insert(
            "daily_last_claimed".to_owned(),
            DocumentField::TimestampValue(self.daily_last_claimed),
        );
        Document::new(fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn can_convert_between_document_and_user() {
        let mut user = User::new(Uuid::new_v4());
        user.cards.push(Uuid::new_v4());

        let user_clone = user.clone();
        let mut doc: Document = user_clone.into();
        doc.name = format!("parent_path/{}", user.id.to_string());

        let user_from_doc: User = doc.try_into().unwrap();

        assert_eq!(user, user_from_doc);
    }
}
