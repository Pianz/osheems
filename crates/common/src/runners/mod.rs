pub mod mqtt;

// Tu pourras ajouter ici d'autres runners au fur et à mesure :
// pub mod modbus;
// pub mod zigbee;

/// Structure de données pour transporter les commandes réseau
/// de manière générique entre le Core et les Runners.
pub struct NetworkAction {
    pub topic: String,
    pub payload: String,
}
