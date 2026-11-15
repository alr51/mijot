//! État de l'application (réglages, menu, cases cochées), enregistré en JSON
//! dans le dossier de données local, à côté de la base du catalogue.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::catalogue::Catalogue;
use crate::plan::{PERSONNES_MAX, Parametres, REPAS_MAX, liste_de_courses};

/// Article ajouté à la main à la liste de courses.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Extra {
    pub nom: String,
    #[serde(default)]
    pub coche: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Etat {
    pub budget: f64,
    pub personnes: u32,
    pub repas: usize,
    pub vege: bool,
    /// Identifiants (base) des recettes du menu, dans l'ordre des repas.
    pub menu: Vec<i64>,
    /// Identifiants (base) des ingrédients déjà cochés dans la liste.
    pub coches: BTreeSet<i64>,
    pub extras: Vec<Extra>,
}

impl Default for Etat {
    fn default() -> Self {
        Self {
            budget: 60.,
            personnes: 2,
            repas: 7,
            vege: false,
            menu: Vec::new(),
            coches: BTreeSet::new(),
            extras: Vec::new(),
        }
    }
}

impl Etat {
    pub fn parametres(&self) -> Parametres {
        Parametres {
            budget: self.budget,
            personnes: self.personnes,
            repas: self.repas,
            vege: self.vege,
        }
    }

    /// Positions des recettes du menu dans le catalogue.
    pub fn menu_indices(&self, cat: &Catalogue) -> Vec<usize> {
        self.menu
            .iter()
            .filter_map(|id| cat.index_recette(*id))
            .collect()
    }

    pub fn definir_menu(&mut self, cat: &Catalogue, menu: &[usize]) {
        self.menu = menu.iter().map(|&r| cat.recettes[r].id).collect();
        self.elaguer(cat);
    }

    /// Remet l'état d'accord avec le catalogue : oublie les recettes supprimées
    /// et les coches des ingrédients qui ne sont plus dans la liste de courses.
    /// À appeler après toute modification du menu ou du catalogue.
    pub fn elaguer(&mut self, cat: &Catalogue) {
        self.menu.retain(|id| cat.index_recette(*id).is_some());
        let presents: BTreeSet<i64> =
            liste_de_courses(cat, &self.menu_indices(cat), self.personnes)
                .iter()
                .flat_map(|(_, lignes)| lignes)
                .map(|l| cat.ingredients[l.ingredient].id)
                .collect();
        self.coches.retain(|id| presents.contains(id));
    }

    /// Ramène un état lu sur disque dans les bornes.
    fn assainir(mut self, cat: &Catalogue) -> Self {
        if !self.budget.is_finite() || self.budget < 0. {
            self.budget = Self::default().budget;
        }
        self.personnes = self.personnes.clamp(1, PERSONNES_MAX);
        self.repas = self.repas.clamp(1, REPAS_MAX);
        // Un même repas deux fois (fichier retouché) : le planificateur ne le prévoit pas.
        let mut vus = BTreeSet::new();
        self.menu.retain(|id| vus.insert(*id));
        self.menu.truncate(REPAS_MAX);
        self.elaguer(cat);
        self
    }

    fn depuis_json(json: &str, cat: &Catalogue) -> Option<Self> {
        serde_json::from_str::<Etat>(json)
            .ok()
            .map(|etat| etat.assainir(cat))
    }

    /// État enregistré, ou état par défaut s'il est absent ou illisible (c'est
    /// aussi le cas d'un fichier de la première version, aux identifiants textuels).
    pub fn charger(cat: &Catalogue) -> Self {
        fichier()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .and_then(|json| Self::depuis_json(&json, cat))
            .unwrap_or_default()
    }

    /// Écriture atomique (fichier temporaire puis renommage).
    pub fn enregistrer(&self) -> std::io::Result<()> {
        let Some(fichier) = fichier() else {
            return Ok(());
        };
        if let Some(dossier) = fichier.parent() {
            std::fs::create_dir_all(dossier)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        let temporaire = fichier.with_extension("json.tmp");
        std::fs::write(&temporaire, json)?;
        std::fs::rename(temporaire, fichier)
    }
}

/// Dossier de données : celui de l'hôte s'il en fournit un (Android), sinon
/// `~/.local/share/mijot` sous Linux.
pub fn dossier() -> Option<PathBuf> {
    crate::hote::dossier().or_else(|| {
        directories::ProjectDirs::from("", "", "mijot").map(|d| d.data_dir().to_path_buf())
    })
}

fn fichier() -> Option<PathBuf> {
    dossier().map(|d| d.join("etat.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour_json() {
        let cat = Catalogue::en_memoire().unwrap();
        let mut etat = Etat::default();
        etat.definir_menu(&cat, &[0, 3, 5]);
        // Premier ingrédient de la première recette du menu : il est dans la liste.
        etat.coches.insert(cat.recettes[0].ingredients[0].0);
        etat.extras.push(Extra {
            nom: "Café".into(),
            coche: true,
        });
        let json = serde_json::to_string(&etat).unwrap();
        assert_eq!(Etat::depuis_json(&json, &cat), Some(etat));
    }

    #[test]
    fn etat_abime_ou_ancien() {
        let cat = Catalogue::en_memoire().unwrap();
        assert_eq!(Etat::depuis_json("pas du json", &cat), None);
        // Champs absents : valeurs par défaut.
        assert_eq!(Etat::depuis_json("{}", &cat), Some(Etat::default()));
        // Première version : identifiants textuels, repartir de zéro.
        assert_eq!(Etat::depuis_json(r#"{"menu":["carbonara"]}"#, &cat), None);

        let recette = &cat.recettes[0];
        let dedans = recette.ingredients[0].0;
        let dehors = cat
            .ingredients
            .iter()
            .map(|i| i.id)
            .find(|id| recette.ingredients.iter().all(|(autre, _)| autre != id))
            .unwrap();
        let json = format!(
            r#"{{"budget":-3,"personnes":0,"repas":99,"menu":[{0},9999,{0}],"coches":[{dedans},{dehors}]}}"#,
            recette.id
        );
        let etat = Etat::depuis_json(&json, &cat).unwrap();
        assert_eq!(etat.budget, 60.);
        assert_eq!(etat.personnes, 1);
        assert_eq!(etat.repas, REPAS_MAX);
        // Recette inconnue et doublon écartés.
        assert_eq!(etat.menu, vec![recette.id]);
        assert_eq!(etat.coches.into_iter().collect::<Vec<_>>(), vec![dedans]);
    }

    #[test]
    fn supprimer_une_recette_la_retire_du_menu() {
        let mut cat = Catalogue::en_memoire().unwrap();
        let mut etat = Etat::default();
        etat.definir_menu(&cat, &[0, 1]);
        let (premiere, seconde) = (cat.recettes[0].id, cat.recettes[1].id);
        // Un ingrédient propre à la première recette, coché.
        let propre = cat.recettes[0]
            .ingredients
            .iter()
            .map(|(id, _)| *id)
            .find(|id| {
                cat.recettes[1]
                    .ingredients
                    .iter()
                    .all(|(autre, _)| autre != id)
            })
            .unwrap();
        etat.coches.insert(propre);

        cat.supprimer_recette(premiere).unwrap();
        etat.elaguer(&cat);
        assert_eq!(etat.menu, vec![seconde]);
        assert!(etat.coches.is_empty());
    }
}
