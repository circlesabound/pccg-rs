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
        if let Some(DocumentField::StringValue(prototype_id_str)) = value.fields.get("prototype_id")
        {
            match Uuid::parse_str(&prototype_id_str) {
                Ok(uuid) => prototype_id = uuid,
                Err(e) => {
                    return Err(format!("Could not convert Document to Character: error parsing field 'prototype_id': {}", e));
                }
            }
        } else {
            return Err(
                "Could not convert Document to Character: missing field 'prototype_id'".to_owned(),
            );
        }

        let level;
        if let Some(DocumentField::IntegerValue(level_str)) = value.fields.get("level") {
            match level_str.parse() {
                Ok(l) => level = l,
                Err(e) => {
                    return Err(format!(
                        "Could not convert Document to Character: error parsing field 'level': {}",
                        e
                    ))
                }
            }
        } else {
            return Err(
                "Could not convert Document to Character: missing field 'level'".to_owned(),
            );
        }

        let experience;
        if let Some(DocumentField::IntegerValue(experience_str)) = value.fields.get("experience") {
            match experience_str.parse() {
                Ok(l) => experience = l,
                Err(e) => return Err(format!(
                    "Could not convert Document to Character: error parsing field 'experience': {}",
                    e
                )),
            }
        } else {
            return Err(
                "Could not convert Document to Character: missing field 'experience'".to_owned(),
            );
        }

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
