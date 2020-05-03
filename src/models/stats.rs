use crate::storage::firestore::{DocumentField, DocumentMapValue};
use std::{collections::HashMap, convert::TryFrom};

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

impl TryFrom<&DocumentField> for StatsI {
    type Error = String;

    fn try_from(value: &DocumentField) -> Result<Self, Self::Error> {
        if let DocumentField::MapValue(dmv) = value {
            let physical;
            let mental;
            let tactical;
            if let Some(fields) = &dmv.fields {
                if let Some(doc_field) = fields.get("physical") {
                    if let DocumentField::IntegerValue(ret_str) = doc_field {
                        if let Ok(ret) = ret_str.parse() {
                            physical = ret;
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
                            mental = ret;
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
                            tactical = ret;
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
                physical = Default::default();
                mental = Default::default();
                tactical = Default::default();
            }

            Ok(StatsI {
                physical,
                mental,
                tactical,
            })
        } else {
            Err(format!(
                "Expected DocumentMapValue to convert to StatsI, found {:?}",
                value
            ))
        }
    }
}

impl Into<DocumentField> for StatsI {
    fn into(self) -> DocumentField {
        let mut map = HashMap::new();
        map.insert(
            "physical".to_owned(),
            DocumentField::IntegerValue(self.physical.to_string()),
        );
        map.insert(
            "mental".to_owned(),
            DocumentField::IntegerValue(self.mental.to_string()),
        );
        map.insert(
            "tactical".to_owned(),
            DocumentField::IntegerValue(self.tactical.to_string()),
        );

        DocumentField::MapValue(DocumentMapValue { fields: Some(map) })
    }
}

impl TryFrom<&DocumentField> for StatsF {
    type Error = String;

    fn try_from(value: &DocumentField) -> Result<Self, Self::Error> {
        let physical;
        let mental;
        let tactical;
        if let DocumentField::MapValue(dmv) = value {
            if let Some(fields) = &dmv.fields {
                if let Some(doc_field) = fields.get("physical") {
                    if let DocumentField::DoubleValue(ret) = doc_field {
                        physical = *ret;
                    } else {
                        return Err(format!("Error parsing DoubleValue from {:?}", doc_field));
                    }
                } else {
                    return Err(format!("Missing field 'physical' in map"));
                }
                if let Some(doc_field) = fields.get("mental") {
                    if let DocumentField::DoubleValue(ret) = doc_field {
                        mental = *ret;
                    } else {
                        return Err(format!("Error parsing DoubleValue from {:?}", doc_field));
                    }
                } else {
                    return Err(format!("Missing field 'mental' in map"));
                }
                if let Some(doc_field) = fields.get("tactical") {
                    if let DocumentField::DoubleValue(ret) = doc_field {
                        tactical = *ret;
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
                physical = Default::default();
                mental = Default::default();
                tactical = Default::default();
            }

            Ok(StatsF {
                physical,
                mental,
                tactical,
            })
        } else {
            Err(format!(
                "Expected DocumentMapValue to convert to StatsI, found {:?}",
                value
            ))
        }
    }
}

impl Into<DocumentField> for StatsF {
    fn into(self) -> DocumentField {
        let mut map = HashMap::new();
        map.insert(
            "physical".to_owned(),
            DocumentField::DoubleValue(self.physical),
        );
        map.insert("mental".to_owned(), DocumentField::DoubleValue(self.mental));
        map.insert(
            "tactical".to_owned(),
            DocumentField::DoubleValue(self.tactical),
        );

        DocumentField::MapValue(DocumentMapValue { fields: Some(map) })
    }
}
