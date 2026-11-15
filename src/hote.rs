//! Ce que la plateforme hôte fournit à l'application quand elle n'est pas un
//! bureau : sur Android, le dossier de données et les marges des barres système.
//! Sans hôte installé (bureau), tout garde sa valeur par défaut.

use std::path::PathBuf;
use std::sync::OnceLock;

/// Bandes de l'écran occupées par le système (barre d'état, barre de
/// navigation), en pixels logiques.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Marges {
    pub haut: f32,
    pub bas: f32,
}

#[derive(Default)]
pub struct Hote {
    /// Dossier de données de l'application, s'il n'est pas celui du bureau.
    pub dossier: Option<PathBuf>,
    /// Marges courantes : relues à chaque rendu, elles changent avec la rotation.
    pub marges: Option<fn() -> Marges>,
}

static HOTE: OnceLock<Hote> = OnceLock::new();

/// À appeler une fois, avant de lancer l'application. Les appels suivants sont
/// ignorés.
pub fn installer(hote: Hote) {
    let _ = HOTE.set(hote);
}

pub(crate) fn dossier() -> Option<PathBuf> {
    HOTE.get().and_then(|hote| hote.dossier.clone())
}

pub(crate) fn marges() -> Marges {
    HOTE.get()
        .and_then(|hote| hote.marges)
        .map(|marges| marges())
        .unwrap_or_default()
}
