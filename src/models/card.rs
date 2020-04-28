use crate::storage::firestore::{Document, DocumentField, DocumentMapValue};
use std::{collections::HashMap, convert::TryFrom};
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

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StatsI {
    pub physical: i32,
    pub mental: i32,
    pub tactical: i32,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StatsF {
    pub physical: f64,
    pub mental: f64,
    pub tactical: f64,
}

impl TryFrom<Document> for Card {
    type Error = String;

    fn try_from(value: Document) -> Result<Self, Self::Error> {
        let id = value.extract_id()?;
        let name = value.extract_string("name")?;
        let description = value.extract_string("description")?;
        let image_uri = value.extract_string("image_uri")?;

        let stat_base_physical;
        let stat_base_mental;
        let stat_base_tactical;
        if let Some(DocumentField::MapValue(dmv)) = value.fields.get("stat_base") {
            if let Some(fields) = &dmv.fields {
                if let Some(doc_field) = fields.get("physical") {
                    if let DocumentField::IntegerValue(ret_str) = doc_field {
                        if let Ok(ret) = ret_str.parse() {
                            stat_base_physical = ret;
                        } else {
                            return Err(format!("Error casting to i32 from {}", ret_str));
                        }
                    } else {
                        return Err(format!("Error parsing IntegerValue from {:?}", doc_field));
                    }
                } else {
                    return Err(format!("Missing field 'physical' in map"));
                }
                if let Some(doc_field) = fields.get("mental") {
                    if let DocumentField::IntegerValue(ret_str) = doc_field {
                        if let Ok(ret) = ret_str.parse() {
                            stat_base_mental = ret;
                        } else {
                            return Err(format!("Error casting to i32 from {}", ret_str));
                        }
                    } else {
                        return Err(format!("Error parsing IntegerValue from {:?}", doc_field));
                    }
                } else {
                    return Err(format!("Missing field 'mental' in map"));
                }
                if let Some(doc_field) = fields.get("tactical") {
                    if let DocumentField::IntegerValue(ret_str) = doc_field {
                        if let Ok(ret) = ret_str.parse() {
                            stat_base_tactical = ret;
                        } else {
                            return Err(format!("Error casting to i32 from {}", ret_str));
                        }
                    } else {
                        return Err(format!("Error parsing IntegerValue from {:?}", doc_field));
                    }
                } else {
                    return Err(format!("Missing field 'tactical' in map"));
                }
            } else {
                warn!(
                    "Missing hashmap fields converting DocumentMapValue to StatsI, using defaults"
                );
                stat_base_physical = Default::default();
                stat_base_mental = Default::default();
                stat_base_tactical = Default::default();
            }
        } else {
            warn!("Missing hashmap fields converting DocumentMapValue to StatsI, using defaults");
            stat_base_physical = Default::default();
            stat_base_mental = Default::default();
            stat_base_tactical = Default::default();
        }

        let stat_multiplier_physical;
        let stat_multiplier_mental;
        let stat_multiplier_tactical;
        if let Some(DocumentField::MapValue(dmv)) = value.fields.get("stat_multiplier") {
            if let Some(fields) = &dmv.fields {
                if let Some(doc_field) = fields.get("physical") {
                    if let DocumentField::DoubleValue(ret) = doc_field {
                        stat_multiplier_physical = *ret;
                    } else {
                        return Err(format!("Error parsing DoubleValue from {:?}", doc_field));
                    }
                } else {
                    return Err(format!("Missing field 'physical' in map"));
                }
                if let Some(doc_field) = fields.get("mental") {
                    if let DocumentField::DoubleValue(ret) = doc_field {
                        stat_multiplier_mental = *ret;
                    } else {
                        return Err(format!("Error parsing DoubleValue from {:?}", doc_field));
                    }
                } else {
                    return Err(format!("Missing field 'mental' in map"));
                }
                if let Some(doc_field) = fields.get("tactical") {
                    if let DocumentField::DoubleValue(ret) = doc_field {
                        stat_multiplier_tactical = *ret;
                    } else {
                        return Err(format!("Error parsing DoubleValue from {:?}", doc_field));
                    }
                } else {
                    return Err(format!("Missing field 'tactical' in map"));
                }
            } else {
                warn!(
                    "Missing hashmap fields converting DocumentMapValue to StatsF, using defaults"
                );
                stat_multiplier_physical = Default::default();
                stat_multiplier_mental = Default::default();
                stat_multiplier_tactical = Default::default();
            }
        } else {
            warn!("Missing hashmap fields converting DocumentMapValue to StatsF, using defaults");
            stat_multiplier_physical = Default::default();
            stat_multiplier_mental = Default::default();
            stat_multiplier_tactical = Default::default();
        }

        Ok(Card {
            id,
            name,
            description,
            image_uri,
            stat_base: StatsI {
                physical: stat_base_physical,
                mental: stat_base_mental,
                tactical: stat_base_tactical,
            },
            stat_multiplier: StatsF {
                physical: stat_multiplier_physical,
                mental: stat_multiplier_mental,
                tactical: stat_multiplier_tactical,
            },
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

        let mut stat_base_map = HashMap::new();
        stat_base_map.insert(
            "physical".to_owned(),
            DocumentField::IntegerValue(self.stat_base.physical.to_string()),
        );
        stat_base_map.insert(
            "mental".to_owned(),
            DocumentField::IntegerValue(self.stat_base.mental.to_string()),
        );
        stat_base_map.insert(
            "tactical".to_owned(),
            DocumentField::IntegerValue(self.stat_base.tactical.to_string()),
        );
        fields.insert(
            "stat_base".to_owned(),
            DocumentField::MapValue(DocumentMapValue {
                fields: Some(stat_base_map),
            }),
        );

        let mut stat_multiplier_map = HashMap::new();
        stat_multiplier_map.insert(
            "physical".to_owned(),
            DocumentField::DoubleValue(self.stat_multiplier.physical),
        );
        stat_multiplier_map.insert(
            "mental".to_owned(),
            DocumentField::DoubleValue(self.stat_multiplier.mental),
        );
        stat_multiplier_map.insert(
            "tactical".to_owned(),
            DocumentField::DoubleValue(self.stat_multiplier.tactical),
        );
        fields.insert(
            "stat_multiplier".to_owned(),
            DocumentField::MapValue(DocumentMapValue {
                fields: Some(stat_multiplier_map),
            }),
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
        pretty_env_logger::init();
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
        warn!("{:?}", doc);
        doc.name = format!("parent_path/{}", card.id.to_string());

        let card_from_doc: Card = doc.try_into().unwrap();

        assert_eq!(card, card_from_doc);
    }
}
