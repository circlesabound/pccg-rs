use crate::storage::firestore::{Document, DocumentField};
use std::{collections::HashMap, convert::{TryInto, TryFrom}, sync::Arc};
use super::{card::StatsF, Card};
use uuid::Uuid;
use tokio::sync::Mutex;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Character {
    pub id: Uuid,
    pub prototype_id: Uuid,
    pub level: u32,
    pub experience: u32,
    #[serde(skip)]
    #[serde(default = "default_prototype_field")]
    prototype: Arc<Mutex<Option<Card>>>,
}

fn default_prototype_field() -> Arc<Mutex<Option<Card>>> {
    Arc::new(Mutex::new(None))
}

impl Character {
    pub fn new(id: Uuid, prototype_id: Uuid) -> Character {
        Character {
            id,
            prototype_id,
            level: 1,
            experience: 0,
            prototype: default_prototype_field(),
        }
    }

    pub async fn expand(&self, prototype: Card) {
        if prototype.id != self.prototype_id {
            panic!("id mismatch! self.prototype_id = {}, prototype.id = {}", self.prototype_id, prototype.id)
        }

        let mut lock = self.prototype.lock().await;
        *lock = Some(prototype);
    }
}

impl PartialEq for Character {
    fn eq(&self, other: &Character) -> bool {
        self.id == other.id &&
            self.prototype_id == other.prototype_id &&
            self.level == other.level &&
            self.experience == other.experience
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
            prototype: default_prototype_field(),
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

#[derive(serde::Serialize)]
pub struct CharacterEx {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub image_uri: String,
    pub level: u32,
    pub experience: u32,
    pub stats: StatsF,
}

impl CharacterEx {
    pub async fn new(character: Character, prototype: Card) -> CharacterEx {
        character.expand(prototype).await;
        character.try_into().unwrap()
    }
}

impl TryFrom<Character> for CharacterEx {
    type Error = String;

    fn try_from(value: Character) -> Result<Self, Self::Error> {
        if let Some(prototype) = Arc::try_unwrap(value.prototype).unwrap().into_inner() {
            let stats = StatsF {
                physical: prototype.stat_base.physical as f64 + value.level as f64 * prototype.stat_multiplier.physical,
                mental: prototype.stat_base.mental as f64 + value.level as f64 * prototype.stat_multiplier.mental,
                tactical: prototype.stat_base.tactical as f64 + value.level as f64 * prototype.stat_multiplier.tactical,
            };
            Ok(CharacterEx {
                id: value.id,
                name: prototype.name,
                description: prototype.description,
                image_uri: prototype.image_uri,
                level: value.level,
                experience: value.experience,
                stats,
            })
        } else {
            Err("Could not expand Character, no prototype".to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
