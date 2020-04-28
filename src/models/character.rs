use crate::storage::firestore::{Document, DocumentField};
use std::{collections::HashMap, convert::TryFrom};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Character {
    pub id: Uuid,
    pub prototype_id: Uuid,
    pub level: u32,
    pub experience: u32,
}

impl Character {
    pub fn new(id: Uuid, prototype_id: Uuid) -> Character {
        Character {
            id,
            prototype_id,
            level: 1,
            experience: 0,
        }
    }
}

impl TryFrom<Document> for Character {
    type Error = String;

    fn try_from(value: Document) -> Result<Self, Self::Error> {
        let id = value.extract_id();
        if let Err(e) = id {
            return Err(format!("Could not convert Document to Character: {}", e));
        }
        let id = id.unwrap();

        let prototype_id;
        match Uuid::parse_str(&value.extract_string("prototype_id")?) {
            Ok(id) => prototype_id = id,
            Err(e) => return Err(format!(
                "Could not convert Document to Character: error parsing field 'prototype_id': {}",
                e
            )),
        }

        let level = value.extract_integer("level")?;
        let experience = value.extract_integer("experience")?;

        Ok(Character {
            id,
            prototype_id,
            level,
            experience,
        })
    }
}

impl Into<Document> for Character {
    fn into(self) -> Document {
        let mut fields = HashMap::new();
        fields.insert(
            "prototype_id".to_owned(),
            DocumentField::StringValue(self.prototype_id.to_string()),
        );
        fields.insert(
            "level".to_owned(),
            DocumentField::IntegerValue(self.level.to_string()),
        );
        fields.insert(
            "experience".to_owned(),
            DocumentField::IntegerValue(self.experience.to_string()),
        );
        Document::new(fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn can_convert_between_document_and_character() {
        let character = Character::new(Uuid::new_v4(), Uuid::new_v4());

        let character_clone = character.clone();
        let mut doc: Document = character_clone.into();
        doc.name = format!("parent_path/{}", character.id.to_string());

        let character_from_doc: Character = doc.try_into().unwrap();

        assert_eq!(character, character_from_doc);
    }
}
