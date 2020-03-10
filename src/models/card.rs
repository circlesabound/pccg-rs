#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct Card {
    name: String,
    description: String,
    image_uri: String,
}

impl Card {
    pub fn new(name: String, description: String, image_uri: String) -> Card {
        Card {
            name,
            description,
            image_uri
        }
    }
}
