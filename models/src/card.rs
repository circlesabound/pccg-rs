use crate::stats::{StatsF, StatsI};
use pccg_rs_storage::firestore::{Document, DocumentField};
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
};
use uuid::Uuid;

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Card {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub image_uri: String,
    pub stat_base: StatsI,
    pub stat_multiplier: StatsF,
}

impl TryFrom<Document> for Card {
    type Error = String;

    fn try_from(value: Document) -> Result<Self, Self::Error> {
        let id = value.extract_id()?;
        let name = value.extract_string("name")?;
        let description = value.extract_string("description")?;
        let image_uri = value.extract_string("image_uri")?;

        let stat_base;
        if let Some(df) = value.fields.get("stat_base") {
            stat_base = df.try_into()?;
        } else {
            return Err(format!("Missing field 'stat_base'"));
        }

        let stat_multiplier;
        if let Some(df) = value.fields.get("stat_multiplier") {
            stat_multiplier = df.try_into()?;
        } else {
            return Err(format!("Missing field 'stat_multiplier'"));
        }

        Ok(Card {
            id,
            name,
            description,
            image_uri,
            stat_base,
            stat_multiplier,
        })
    }
}

impl Into<Document> for Card {
    fn into(self) -> Document {
        let mut fields = HashMap::new();
        fields.insert("name".to_owned(), DocumentField::StringValue(self.name));
        fields.insert(
            "description".to_owned(),
            DocumentField::StringValue(self.description),
        );
        fields.insert(
            "image_uri".to_owned(),
            DocumentField::StringValue(self.image_uri),
        );
        fields.insert("stat_base".to_owned(), self.stat_base.into());
        fields.insert("stat_multiplier".to_owned(), self.stat_multiplier.into());

        Document::new(fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn can_convert_between_document_and_card() {
        let card = Card {
            id: Uuid::new_v4(),
            name: "test card".to_owned(),
            description: "test description".to_owned(),
            image_uri: "https://localhost/test_uri.png".to_owned(),
            stat_base: StatsI {
                physical: 24,
                mental: 12,
                tactical: 19,
            },
            stat_multiplier: StatsF {
                physical: 2.0,
                mental: 0.9,
                tactical: 0.5,
            },
        };

        let card_clone = card.clone();
        let mut doc: Document = card_clone.into();
        doc.name = format!("parent_path/{}", card.id.to_string());

        let card_from_doc: Card = doc.try_into().unwrap();

        assert_eq!(card, card_from_doc);
    }
}
