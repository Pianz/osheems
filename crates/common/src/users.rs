use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, PartialOrd)]
pub enum UserRole {
    Guest,      // Accès très limité (ex: affichage public)
    Viewer,     // Peut voir les données mais rien modifier
    User,       // Utilisateur quotidien (peut contrôler certains appareils)
    Admin,      // Administrateur du site (gère les utilisateurs locaux)
    Operator,   // Installateur/Intégrateur (config matérielle, maintenance)
    SuperAdmin, // Développeur (accès total, debug, système racine)
}

impl From<String> for UserRole {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "superadmin" => UserRole::SuperAdmin,
            "operator"   => UserRole::Operator,
            "admin"      => UserRole::Admin,
            "user"       => UserRole::User,
            "viewer"     => UserRole::Viewer,
            "guest"      => UserRole::Guest,
            _            => UserRole::Guest, // Default Security Level
        }
    }
}

impl ToString for UserRole {
    fn to_string(&self) -> String {
        match self {
            UserRole::SuperAdmin => "superadmin".to_string(),
            UserRole::Operator   => "operator".to_string(),
            UserRole::Admin      => "admin".to_string(),
            UserRole::User       => "user".to_string(),
            UserRole::Viewer     => "viewer".to_string(),
            UserRole::Guest      => "guest".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub entity_id: i64,
    pub username: String,
    pub role: UserRole,
    pub last_login: Option<String>,
}

impl User {
    /// Helper pour vérifier les permissions minimales
    pub fn has_permission(&self, required: UserRole) -> bool {
        // Grâce à PartialOrd, on peut comparer les rôles si l'ordre dans l'enum est respecté
        self.role >= required
    }
}
