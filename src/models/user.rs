use uuid::Uuid;

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct User {
    pub id: Uuid,
    pub cards: Vec<Uuid>,
}
