#[derive(
    Clone,
    Debug,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
)]
pub struct Card {
    id: uuid::Uuid,
    name: String,
    description: String,
    image_uri: String,
}

impl Card {
    pub fn new(id: uuid::Uuid, name: String, description: String, image_uri: String) -> Card {
        Card {
            id,
            name,
            description,
            image_uri
        }
    }
}
