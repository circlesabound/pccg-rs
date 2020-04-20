use crate::storage::firestore::{Document, DocumentField};
use std::{collections::HashMap, convert::TryFrom};
use uuid::Uuid;

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Card {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub image_uri: String,
}

impl TryFrom<Document> for Card {
    type Error = &'static str;

    fn try_from(value: Document) -> Result<Self, Self::Error> {
        let id = value.name.split('/').next_back().unwrap();
        if let Some(DocumentField::StringValue(name)) = value.fields.get("name") {
            if let Some(DocumentField::StringValue(description)) = value.fields.get("description") {
                if let Some(DocumentField::StringValue(image_uri)) = value.fields.get("image_uri") {
                    return Ok(Card {
                        id: Uuid::parse_str(id).unwrap(),
                        name: name.to_string(),
                        description: description.to_string(),
                        image_uri: image_uri.to_string(),
                    });
                }
            }
        }

        Err("Could not convert Document to Card")
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
        };

        let card_clone = card.clone();
        let mut doc: Document = card_clone.into();
        doc.name = format!("parent_path/{}", card.id.to_string());

        let card_from_doc: Card = doc.try_into().unwrap();

        assert_eq!(card, card_from_doc);
    }
}
