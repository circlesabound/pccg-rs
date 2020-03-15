use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Card {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub image_uri: String,
}

impl Card {
    pub fn new(id: Uuid, name: String, description: String, image_uri: String) -> Card {
        Card {
            id,
            name,
            description,
            image_uri,
        }
    }
}
