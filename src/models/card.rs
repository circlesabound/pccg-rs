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
        Document {
            name: "".to_owned(),
            fields,
            create_time: "".to_owned(),
            update_time: "".to_owned(),
        }
    }
}
