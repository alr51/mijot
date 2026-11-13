//! Catalogue modifiable : ingrédients et recettes, enregistrés en SQLite.
//!
//! La base est créée et remplie au premier lancement à partir du catalogue de
//! départ (`donnees.rs`). Le catalogue garde une copie en mémoire, relue après
//! chaque écriture : le reste de l'application ne parle jamais SQL.

use std::fmt;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension as _, params};

use crate::donnees::{self, Categorie, Rayon, Unite};

/// Version du schéma, tenue dans `PRAGMA user_version`.
const VERSION: i64 = 3;

/// Amène la base de la version `vers - 1` à la version `vers`.
fn migrer(base: &Connection, vers: i64) -> Result<(), Erreur> {
    match vers {
        // Catégorie Open Food Facts choisie pour suggérer un prix.
        2 => base.execute_batch(
            "ALTER TABLE ingredient ADD COLUMN off_tag TEXT;
             ALTER TABLE ingredient ADD COLUMN off_nom TEXT;",
        )?,
        // Deuxième lot de recettes du catalogue de départ.
        3 => ajouter_lot(base, donnees::LOT_2)?,
        _ => unreachable!("pas de migration vers la version {vers}"),
    }
    Ok(())
}

/// Schéma de la version 1 ; les versions suivantes sont dans [`migrer`].
const SCHEMA: &str = "
CREATE TABLE ingredient (
    id    INTEGER PRIMARY KEY AUTOINCREMENT,
    nom   TEXT NOT NULL,
    rayon TEXT NOT NULL,
    unite TEXT NOT NULL,
    prix  REAL NOT NULL CHECK (prix >= 0)
);
CREATE TABLE recette (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    nom       TEXT NOT NULL,
    categorie TEXT NOT NULL,
    vege      INTEGER NOT NULL,
    minutes   INTEGER NOT NULL
);
CREATE TABLE recette_ingredient (
    recette_id    INTEGER NOT NULL REFERENCES recette(id) ON DELETE CASCADE,
    position      INTEGER NOT NULL,
    ingredient_id INTEGER NOT NULL REFERENCES ingredient(id) ON DELETE RESTRICT,
    quantite      REAL NOT NULL CHECK (quantite > 0),
    PRIMARY KEY (recette_id, position)
);
CREATE TABLE etape (
    recette_id INTEGER NOT NULL REFERENCES recette(id) ON DELETE CASCADE,
    position   INTEGER NOT NULL,
    texte      TEXT NOT NULL,
    PRIMARY KEY (recette_id, position)
);
";

#[derive(Clone, Debug, PartialEq)]
pub struct Ingredient {
    pub id: i64,
    pub nom: String,
    pub rayon: Rayon,
    pub unite: Unite,
    /// Prix en euros : au kilo, au litre ou à la pièce selon l'unité.
    pub prix: f64,
    /// Catégorie Open Food Facts retenue pour les suggestions de prix.
    pub off: Option<CategorieOff>,
}

/// Catégorie de la taxonomie Open Food Facts.
#[derive(Clone, Debug, PartialEq)]
pub struct CategorieOff {
    /// Identifiant canonique : `en:pastas`.
    pub tag: String,
    /// Nom affiché : « Pâtes alimentaires ».
    pub nom: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Recette {
    /// 0 pour une recette pas encore enregistrée.
    pub id: i64,
    pub nom: String,
    pub categorie: Categorie,
    pub vege: bool,
    /// Temps total (préparation et cuisson), en minutes.
    pub minutes: u32,
    /// (identifiant d'ingrédient, quantité par personne dans l'unité de l'ingrédient).
    pub ingredients: Vec<(i64, f64)>,
    pub etapes: Vec<String>,
}

#[derive(Debug)]
pub enum Erreur {
    Base(rusqlite::Error),
    /// Saisie refusée ; le texte s'adresse à l'utilisateur.
    Invalide(String),
}

impl fmt::Display for Erreur {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Erreur::Base(e) => write!(f, "base de données : {e}"),
            Erreur::Invalide(message) => f.write_str(message),
        }
    }
}

impl From<rusqlite::Error> for Erreur {
    fn from(e: rusqlite::Error) -> Self {
        Erreur::Base(e)
    }
}

fn invalide<T>(message: impl Into<String>) -> Result<T, Erreur> {
    Err(Erreur::Invalide(message.into()))
}

pub struct Catalogue {
    base: Connection,
    /// Dans l'ordre de création.
    pub ingredients: Vec<Ingredient>,
    /// Dans l'ordre de création.
    pub recettes: Vec<Recette>,
}

impl Catalogue {
    pub fn ouvrir(chemin: &Path) -> Result<Self, Erreur> {
        Self::preparer(Connection::open(chemin)?)
    }

    /// Catalogue de départ, sans fichier : tests, ou secours si la base est illisible.
    pub fn en_memoire() -> Result<Self, Erreur> {
        Self::preparer(Connection::open_in_memory()?)
    }

    fn preparer(mut base: Connection) -> Result<Self, Erreur> {
        base.pragma_update(None, "foreign_keys", true)?;
        let mut version: i64 = base.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > VERSION {
            return invalide(format!(
                "la base vient d'une version plus récente de Mijot (schéma {version})"
            ));
        }
        if version < VERSION {
            let tx = base.transaction()?;
            if version == 0 {
                tx.execute_batch(SCHEMA)?;
                remplir(&tx)?;
                version = 1;
            }
            for vers in version + 1..=VERSION {
                migrer(&tx, vers)?;
            }
            tx.pragma_update(None, "user_version", VERSION)?;
            tx.commit()?;
        }
        let mut catalogue = Self {
            base,
            ingredients: Vec::new(),
            recettes: Vec::new(),
        };
        catalogue.recharger()?;
        Ok(catalogue)
    }

    fn recharger(&mut self) -> Result<(), Erreur> {
        let ingredients = self
            .base
            .prepare(
                "SELECT id, nom, rayon, unite, prix, off_tag, off_nom FROM ingredient ORDER BY id",
            )?
            .query_map([], |r| {
                Ok(Ingredient {
                    id: r.get(0)?,
                    nom: r.get(1)?,
                    // Code inconnu (base retouchée à la main) : valeur neutre plutôt qu'un refus.
                    rayon: Rayon::depuis_code(&r.get::<_, String>(2)?).unwrap_or(Rayon::Epicerie),
                    unite: Unite::depuis_code(&r.get::<_, String>(3)?).unwrap_or(Unite::G),
                    prix: r.get(4)?,
                    off: match (
                        r.get::<_, Option<String>>(5)?,
                        r.get::<_, Option<String>>(6)?,
                    ) {
                        (Some(tag), Some(nom)) => Some(CategorieOff { tag, nom }),
                        _ => None,
                    },
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut recettes = self
            .base
            .prepare("SELECT id, nom, categorie, vege, minutes FROM recette ORDER BY id")?
            .query_map([], |r| {
                Ok(Recette {
                    id: r.get(0)?,
                    nom: r.get(1)?,
                    categorie: Categorie::depuis_code(&r.get::<_, String>(2)?)
                        .unwrap_or(Categorie::Express),
                    vege: r.get(3)?,
                    minutes: r.get(4)?,
                    ingredients: Vec::new(),
                    etapes: Vec::new(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // Les deux requêtes suivent l'ordre des recettes : un seul parcours suffit.
        let position = |recettes: &[Recette], id: i64| recettes.iter().position(|r| r.id == id);
        let mut lignes = self.base.prepare(
            "SELECT recette_id, ingredient_id, quantite FROM recette_ingredient
             ORDER BY recette_id, position",
        )?;
        for ligne in lignes.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, f64>(2)?,
            ))
        })? {
            let (recette, ingredient, quantite) = ligne?;
            if let Some(i) = position(&recettes, recette) {
                recettes[i].ingredients.push((ingredient, quantite));
            }
        }
        drop(lignes);
        let mut etapes = self
            .base
            .prepare("SELECT recette_id, texte FROM etape ORDER BY recette_id, position")?;
        for etape in etapes.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))? {
            let (recette, texte) = etape?;
            if let Some(i) = position(&recettes, recette) {
                recettes[i].etapes.push(texte);
            }
        }
        drop(etapes);

        self.ingredients = ingredients;
        self.recettes = recettes;
        Ok(())
    }

    /// Position d'un ingrédient dans [`Self::ingredients`].
    pub fn index_ingredient(&self, id: i64) -> Option<usize> {
        self.ingredients.iter().position(|i| i.id == id)
    }

    /// Position d'une recette dans [`Self::recettes`].
    pub fn index_recette(&self, id: i64) -> Option<usize> {
        self.recettes.iter().position(|r| r.id == id)
    }

    /// Nombre de recettes qui utilisent l'ingrédient.
    pub fn utilisations(&self, ingredient: i64) -> usize {
        self.recettes
            .iter()
            .filter(|r| r.ingredients.iter().any(|(id, _)| *id == ingredient))
            .count()
    }

    pub fn definir_prix(&mut self, ingredient: i64, prix: f64) -> Result<(), Erreur> {
        verifier_prix(prix)?;
        let modifies = self.base.execute(
            "UPDATE ingredient SET prix = ?1 WHERE id = ?2",
            params![prix, ingredient],
        )?;
        if modifies == 0 {
            return invalide("Cet ingrédient n'existe plus.");
        }
        self.recharger()
    }

    /// Retient (ou oublie) la catégorie Open Food Facts d'un ingrédient.
    pub fn definir_categorie_off(
        &mut self,
        ingredient: i64,
        categorie: Option<&CategorieOff>,
    ) -> Result<(), Erreur> {
        self.base.execute(
            "UPDATE ingredient SET off_tag = ?1, off_nom = ?2 WHERE id = ?3",
            params![
                categorie.map(|c| c.tag.as_str()),
                categorie.map(|c| c.nom.as_str()),
                ingredient
            ],
        )?;
        self.recharger()
    }

    pub fn ajouter_ingredient(
        &mut self,
        nom: &str,
        rayon: Rayon,
        unite: Unite,
        prix: f64,
    ) -> Result<i64, Erreur> {
        let nom = nom.trim();
        if nom.is_empty() {
            return invalide("Donne un nom à l'ingrédient.");
        }
        if self
            .ingredients
            .iter()
            .any(|i| i.nom.to_lowercase() == nom.to_lowercase())
        {
            return invalide(format!("L'ingrédient « {nom} » existe déjà."));
        }
        verifier_prix(prix)?;
        self.base.execute(
            "INSERT INTO ingredient (nom, rayon, unite, prix) VALUES (?1, ?2, ?3, ?4)",
            params![nom, rayon.code(), unite.code(), prix],
        )?;
        let id = self.base.last_insert_rowid();
        self.recharger()?;
        Ok(id)
    }

    /// Refusé tant qu'une recette utilise l'ingrédient.
    pub fn supprimer_ingredient(&mut self, ingredient: i64) -> Result<(), Erreur> {
        let utilisations = self.utilisations(ingredient);
        if utilisations > 0 {
            return invalide(format!(
                "Cet ingrédient est utilisé par {utilisations} recette(s)."
            ));
        }
        self.base
            .execute("DELETE FROM ingredient WHERE id = ?1", params![ingredient])?;
        self.recharger()
    }

    /// Crée la recette (`id` nul) ou remplace celle qui porte cet identifiant.
    /// Renvoie l'identifiant enregistré.
    pub fn enregistrer_recette(&mut self, recette: &Recette) -> Result<i64, Erreur> {
        let nom = recette.nom.trim();
        if nom.is_empty() {
            return invalide("Donne un nom à la recette.");
        }
        if recette.ingredients.is_empty() {
            return invalide("Ajoute au moins un ingrédient.");
        }
        for (position, (ingredient, quantite)) in recette.ingredients.iter().enumerate() {
            let Some(i) = self.index_ingredient(*ingredient) else {
                return invalide("Un des ingrédients n'existe plus.");
            };
            let nom = &self.ingredients[i].nom;
            if !quantite.is_finite() || *quantite <= 0. {
                return invalide(format!("Indique une quantité positive pour « {nom} »."));
            }
            if recette.ingredients[..position]
                .iter()
                .any(|(autre, _)| autre == ingredient)
            {
                return invalide(format!("« {nom} » figure deux fois dans la recette."));
            }
        }
        if recette.id != 0 && self.index_recette(recette.id).is_none() {
            return invalide("Cette recette n'existe plus.");
        }
        let etapes: Vec<&str> = recette
            .etapes
            .iter()
            .map(|e| e.trim())
            .filter(|e| !e.is_empty())
            .collect();

        let tx = self.base.transaction()?;
        let id = if recette.id == 0 {
            tx.execute(
                "INSERT INTO recette (nom, categorie, vege, minutes) VALUES (?1, ?2, ?3, ?4)",
                params![nom, recette.categorie.code(), recette.vege, recette.minutes],
            )?;
            tx.last_insert_rowid()
        } else {
            tx.execute(
                "UPDATE recette SET nom = ?1, categorie = ?2, vege = ?3, minutes = ?4 WHERE id = ?5",
                params![
                    nom,
                    recette.categorie.code(),
                    recette.vege,
                    recette.minutes,
                    recette.id
                ],
            )?;
            tx.execute(
                "DELETE FROM recette_ingredient WHERE recette_id = ?1",
                params![recette.id],
            )?;
            tx.execute(
                "DELETE FROM etape WHERE recette_id = ?1",
                params![recette.id],
            )?;
            recette.id
        };
        for (position, (ingredient, quantite)) in recette.ingredients.iter().enumerate() {
            tx.execute(
                "INSERT INTO recette_ingredient (recette_id, position, ingredient_id, quantite)
                 VALUES (?1, ?2, ?3, ?4)",
                params![id, position as i64, ingredient, quantite],
            )?;
        }
        for (position, texte) in etapes.iter().enumerate() {
            tx.execute(
                "INSERT INTO etape (recette_id, position, texte) VALUES (?1, ?2, ?3)",
                params![id, position as i64, texte],
            )?;
        }
        tx.commit()?;
        self.recharger()?;
        Ok(id)
    }

    pub fn supprimer_recette(&mut self, recette: i64) -> Result<(), Erreur> {
        self.base
            .execute("DELETE FROM recette WHERE id = ?1", params![recette])?;
        self.recharger()
    }
}

fn verifier_prix(prix: f64) -> Result<(), Erreur> {
    if prix.is_finite() && prix >= 0. {
        Ok(())
    } else {
        invalide("Le prix doit être un nombre positif.")
    }
}

/// Copie le catalogue de départ dans une base vide.
fn remplir(base: &Connection) -> Result<(), Erreur> {
    for i in donnees::INGREDIENTS {
        inserer_ingredient(base, i)?;
    }
    for r in donnees::RECETTES {
        inserer_recette(base, r)?;
    }
    Ok(())
}

/// Ajoute à une base existante les recettes d'un lot du catalogue de départ.
///
/// Une recette dont le nom existe déjà est laissée telle quelle (elle a pu
/// être modifiée, ou créée par l'utilisateur). Prix et ingrédients existants
/// ne sont pas touchés ; un ingrédient supprimé entre-temps est recréé.
fn ajouter_lot(base: &Connection, lot: &[&str]) -> Result<(), Erreur> {
    for r in donnees::RECETTES.iter().filter(|r| lot.contains(&r.id)) {
        let existe: bool = base.query_row(
            "SELECT EXISTS (SELECT 1 FROM recette WHERE nom = ?1 COLLATE NOCASE)",
            params![r.nom],
            |ligne| ligne.get(0),
        )?;
        if !existe {
            inserer_recette(base, r)?;
        }
    }
    Ok(())
}

fn inserer_ingredient(base: &Connection, i: &donnees::Ingredient) -> Result<i64, Erreur> {
    base.execute(
        "INSERT INTO ingredient (nom, rayon, unite, prix) VALUES (?1, ?2, ?3, ?4)",
        params![i.nom, i.rayon.code(), i.unite.code(), i.prix],
    )?;
    Ok(base.last_insert_rowid())
}

/// Insère une recette du catalogue de départ. Ses ingrédients sont retrouvés
/// par leur nom, et créés s'ils manquent.
fn inserer_recette(base: &Connection, r: &donnees::Recette) -> Result<(), Erreur> {
    base.execute(
        "INSERT INTO recette (nom, categorie, vege, minutes) VALUES (?1, ?2, ?3, ?4)",
        params![r.nom, r.categorie.code(), r.vege, r.minutes],
    )?;
    let recette = base.last_insert_rowid();
    for (position, (code, quantite)) in r.ingredients.iter().enumerate() {
        let Some(i) = donnees::index_ingredient(code) else {
            continue;
        };
        let depart = &donnees::INGREDIENTS[i];
        let connu = base
            .query_row(
                "SELECT id FROM ingredient WHERE nom = ?1 COLLATE NOCASE ORDER BY id LIMIT 1",
                params![depart.nom],
                |ligne| ligne.get::<_, i64>(0),
            )
            .optional()?;
        let ingredient = match connu {
            Some(id) => id,
            None => inserer_ingredient(base, depart)?,
        };
        base.execute(
            "INSERT INTO recette_ingredient (recette_id, position, ingredient_id, quantite)
             VALUES (?1, ?2, ?3, ?4)",
            params![recette, position as i64, ingredient, quantite],
        )?;
    }
    for (position, texte) in r.etapes.iter().enumerate() {
        base.execute(
            "INSERT INTO etape (recette_id, position, texte) VALUES (?1, ?2, ?3)",
            params![recette, position as i64, texte],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recette(nom: &str, ingredients: Vec<(i64, f64)>) -> Recette {
        Recette {
            id: 0,
            nom: nom.into(),
            categorie: Categorie::Express,
            vege: true,
            minutes: 10,
            ingredients,
            etapes: vec!["Mélanger.".into(), "  ".into(), " Servir. ".into()],
        }
    }

    fn est_invalide<T: fmt::Debug>(resultat: Result<T, Erreur>) -> bool {
        matches!(resultat, Err(Erreur::Invalide(_)))
    }

    #[test]
    fn catalogue_de_depart() {
        let cat = Catalogue::en_memoire().unwrap();
        assert_eq!(cat.ingredients.len(), donnees::INGREDIENTS.len());
        assert_eq!(cat.recettes.len(), donnees::RECETTES.len());
        // Même ordre et mêmes contenus que le catalogue de départ.
        for (lue, depart) in cat.recettes.iter().zip(donnees::RECETTES) {
            assert_eq!(lue.nom, depart.nom);
            assert_eq!(lue.categorie, depart.categorie);
            assert_eq!(lue.vege, depart.vege);
            assert_eq!(lue.minutes, depart.minutes);
            assert_eq!(lue.etapes, depart.etapes);
            assert_eq!(lue.ingredients.len(), depart.ingredients.len());
            for ((id, quantite), (code, attendue)) in lue.ingredients.iter().zip(depart.ingredients)
            {
                let i = &cat.ingredients[cat.index_ingredient(*id).unwrap()];
                let d = &donnees::INGREDIENTS[donnees::index_ingredient(code).unwrap()];
                assert_eq!(i.nom, d.nom);
                assert_eq!((i.rayon, i.unite, i.prix), (d.rayon, d.unite, d.prix));
                assert_eq!(quantite, attendue);
            }
        }
    }

    #[test]
    fn les_modifications_survivent_a_la_reouverture() {
        let dossier = std::env::temp_dir().join(format!("mijot-test-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let chemin = dossier.join("catalogue.db");
        let _ = std::fs::remove_file(&chemin);

        let (pates, sel, creee) = {
            let mut cat = Catalogue::ouvrir(&chemin).unwrap();
            let pates = cat
                .ingredients
                .iter()
                .find(|i| i.nom == "Pâtes")
                .unwrap()
                .id;
            cat.definir_prix(pates, 2.45).unwrap();
            let sel = cat
                .ajouter_ingredient(" Gros sel ", Rayon::Epicerie, Unite::G, 0.9)
                .unwrap();
            let creee = cat
                .enregistrer_recette(&recette("Pâtes au sel", vec![(pates, 120.), (sel, 2.)]))
                .unwrap();
            cat.supprimer_recette(cat.recettes[0].id).unwrap();
            (pates, sel, creee)
        };

        let cat = Catalogue::ouvrir(&chemin).unwrap();
        // Pas de second remplissage à la réouverture.
        assert_eq!(cat.recettes.len(), donnees::RECETTES.len());
        assert_eq!(cat.ingredients.len(), donnees::INGREDIENTS.len() + 1);
        assert_eq!(
            cat.ingredients[cat.index_ingredient(pates).unwrap()].prix,
            2.45
        );
        assert_eq!(
            cat.ingredients[cat.index_ingredient(sel).unwrap()].nom,
            "Gros sel"
        );
        let lue = &cat.recettes[cat.index_recette(creee).unwrap()];
        assert_eq!(lue.nom, "Pâtes au sel");
        assert_eq!(lue.ingredients, vec![(pates, 120.), (sel, 2.)]);
        // Étapes vides écartées, espaces retirés.
        assert_eq!(lue.etapes, vec!["Mélanger.", "Servir."]);
        assert!(
            cat.recettes
                .iter()
                .all(|r| r.nom != donnees::RECETTES[0].nom)
        );

        drop(cat);
        std::fs::remove_dir_all(dossier).unwrap();
    }

    #[test]
    fn une_base_de_la_version_1_est_migree_sans_perte() {
        let dossier = std::env::temp_dir().join(format!("mijot-migration-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let chemin = dossier.join("v1.db");
        let _ = std::fs::remove_file(&chemin);
        {
            // Base telle que la version 1 l'écrivait, avec un prix modifié.
            let mut base = Connection::open(&chemin).unwrap();
            let tx = base.transaction().unwrap();
            tx.execute_batch(SCHEMA).unwrap();
            remplir(&tx).unwrap();
            tx.execute("UPDATE ingredient SET prix = 7.77 WHERE id = 1", [])
                .unwrap();
            tx.pragma_update(None, "user_version", 1).unwrap();
            tx.commit().unwrap();
        }
        let categorie = CategorieOff {
            tag: "en:onions".into(),
            nom: "Oignons".into(),
        };
        {
            let mut cat = Catalogue::ouvrir(&chemin).unwrap();
            assert_eq!(cat.ingredients.len(), donnees::INGREDIENTS.len());
            assert_eq!(cat.recettes.len(), donnees::RECETTES.len());
            assert_eq!(cat.ingredients[0].prix, 7.77);
            assert_eq!(cat.ingredients[0].off, None);
            cat.definir_categorie_off(1, Some(&categorie)).unwrap();
        }
        // Réouverture : pas de seconde migration, la catégorie est conservée puis oubliée.
        let mut cat = Catalogue::ouvrir(&chemin).unwrap();
        assert_eq!(cat.ingredients[0].off, Some(categorie));
        cat.definir_categorie_off(1, None).unwrap();
        assert_eq!(cat.ingredients[0].off, None);
        drop(cat);
        std::fs::remove_dir_all(dossier).unwrap();
    }

    #[test]
    fn le_lot_2_rejoint_une_base_existante() {
        let dossier = std::env::temp_dir().join(format!("mijot-lot2-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let chemin = dossier.join("v2.db");
        let _ = std::fs::remove_file(&chemin);
        let lot: Vec<&donnees::Recette> = donnees::RECETTES
            .iter()
            .filter(|r| donnees::LOT_2.contains(&r.id))
            .collect();
        let premier_lot = donnees::RECETTES.len() - lot.len();
        // Recette du lot que l'utilisateur aurait déjà créée, à sa façon.
        let deja = lot[0].nom;
        {
            // Base de la version 2, sans le lot, et qui a vécu : un prix modifié,
            // une recette supprimée avec un de ses ingrédients, une recette maison.
            let mut base = Connection::open(&chemin).unwrap();
            base.pragma_update(None, "foreign_keys", true).unwrap();
            let tx = base.transaction().unwrap();
            tx.execute_batch(SCHEMA).unwrap();
            remplir(&tx).unwrap();
            migrer(&tx, 2).unwrap();
            for r in &lot {
                tx.execute("DELETE FROM recette WHERE nom = ?1", params![r.nom])
                    .unwrap();
            }
            tx.execute(
                "UPDATE ingredient SET prix = 7.77 WHERE nom = 'Courgettes'",
                [],
            )
            .unwrap();
            // La pizza est la seule recette du premier lot à utiliser le coulis.
            tx.execute(
                "DELETE FROM recette WHERE nom = 'Pizza maison jambon-champignons'",
                [],
            )
            .unwrap();
            tx.execute("DELETE FROM ingredient WHERE nom = 'Coulis de tomate'", [])
                .unwrap();
            tx.execute(
                "INSERT INTO recette (nom, categorie, vege, minutes) VALUES (?1, 'soupe', 1, 5)",
                params![deja.to_uppercase()],
            )
            .unwrap();
            tx.pragma_update(None, "user_version", 2).unwrap();
            tx.commit().unwrap();
        }

        let cat = Catalogue::ouvrir(&chemin).unwrap();
        // Le premier lot sans la pizza (une recette supprimée ne revient pas), la
        // recette maison, et le lot moins la recette déjà présente sous ce nom.
        assert_eq!(cat.recettes.len(), (premier_lot - 1) + 1 + (lot.len() - 1));
        assert!(cat.recettes.iter().all(|r| !r.nom.starts_with("Pizza")));
        let maison = cat
            .recettes
            .iter()
            .find(|r| r.nom == deja.to_uppercase())
            .unwrap();
        assert_eq!(maison.minutes, 5);
        assert!(cat.recettes.iter().all(|r| r.nom != deja));
        for depart in &lot[1..] {
            let lue = cat.recettes.iter().find(|r| r.nom == depart.nom).unwrap();
            assert_eq!(lue.etapes, depart.etapes);
            assert_eq!(lue.ingredients.len(), depart.ingredients.len());
            for ((id, quantite), (code, attendue)) in lue.ingredients.iter().zip(depart.ingredients)
            {
                let i = &cat.ingredients[cat.index_ingredient(*id).unwrap()];
                let d = &donnees::INGREDIENTS[donnees::index_ingredient(code).unwrap()];
                assert_eq!(i.nom, d.nom);
                assert_eq!(quantite, attendue);
            }
        }
        // Le prix modifié est conservé, l'ingrédient supprimé est recréé une seule fois.
        let prix = |nom: &str| -> Vec<f64> {
            cat.ingredients
                .iter()
                .filter(|i| i.nom == nom)
                .map(|i| i.prix)
                .collect()
        };
        assert_eq!(prix("Courgettes"), vec![7.77]);
        assert_eq!(prix("Coulis de tomate").len(), 1);
        assert_eq!(cat.ingredients.len(), donnees::INGREDIENTS.len());

        // Réouverture : rien n'est ajouté une seconde fois.
        let recettes = cat.recettes.len();
        drop(cat);
        assert_eq!(Catalogue::ouvrir(&chemin).unwrap().recettes.len(), recettes);
        std::fs::remove_dir_all(dossier).unwrap();
    }

    #[test]
    fn modifier_une_recette_remplace_ses_lignes() {
        let mut cat = Catalogue::en_memoire().unwrap();
        let (a, b, c) = (
            cat.ingredients[0].id,
            cat.ingredients[1].id,
            cat.ingredients[2].id,
        );
        let id = cat
            .enregistrer_recette(&recette("Essai", vec![(a, 10.), (b, 1.)]))
            .unwrap();
        let mut modifiee = cat.recettes[cat.index_recette(id).unwrap()].clone();
        modifiee.nom = "Essai bis".into();
        modifiee.vege = false;
        modifiee.minutes = 25;
        modifiee.categorie = Categorie::Soupe;
        modifiee.ingredients = vec![(c, 50.)];
        modifiee.etapes = vec!["Tout revoir.".into()];
        assert_eq!(cat.enregistrer_recette(&modifiee).unwrap(), id);
        assert_eq!(cat.recettes[cat.index_recette(id).unwrap()], modifiee);
        assert_eq!(cat.recettes.len(), donnees::RECETTES.len() + 1);
    }

    #[test]
    fn saisies_refusees() {
        let mut cat = Catalogue::en_memoire().unwrap();
        let a = cat.ingredients[0].id;
        assert!(est_invalide(
            cat.enregistrer_recette(&recette("  ", vec![(a, 1.)]))
        ));
        assert!(est_invalide(
            cat.enregistrer_recette(&recette("Vide", vec![]))
        ));
        assert!(est_invalide(
            cat.enregistrer_recette(&recette("Zéro", vec![(a, 0.)]))
        ));
        assert!(est_invalide(cat.enregistrer_recette(&recette(
            "Double",
            vec![(a, 1.), (a, 2.)]
        ))));
        assert!(est_invalide(
            cat.enregistrer_recette(&recette("Fantôme", vec![(9999, 1.)]))
        ));
        let mut disparue = recette("Disparue", vec![(a, 1.)]);
        disparue.id = 9999;
        assert!(est_invalide(cat.enregistrer_recette(&disparue)));

        assert!(est_invalide(cat.definir_prix(a, -1.)));
        assert!(est_invalide(cat.definir_prix(a, f64::NAN)));
        assert!(est_invalide(cat.definir_prix(9999, 1.)));
        assert!(est_invalide(cat.ajouter_ingredient(
            "",
            Rayon::Frais,
            Unite::G,
            1.
        )));
        // Doublon, à la casse près.
        let existant = cat.ingredients[0].nom.to_uppercase();
        assert!(est_invalide(cat.ajouter_ingredient(
            &existant,
            Rayon::Frais,
            Unite::G,
            1.
        )));
        // Rien n'a été écrit.
        assert_eq!(cat.recettes.len(), donnees::RECETTES.len());
        assert_eq!(cat.ingredients.len(), donnees::INGREDIENTS.len());
    }

    #[test]
    fn un_ingredient_utilise_ne_se_supprime_pas() {
        let mut cat = Catalogue::en_memoire().unwrap();
        let utilise = cat.recettes[0].ingredients[0].0;
        assert!(cat.utilisations(utilise) > 0);
        assert!(est_invalide(cat.supprimer_ingredient(utilise)));
        assert!(cat.index_ingredient(utilise).is_some());

        let libre = cat
            .ajouter_ingredient("Safran", Rayon::Epicerie, Unite::G, 3000.)
            .unwrap();
        assert_eq!(cat.utilisations(libre), 0);
        cat.supprimer_ingredient(libre).unwrap();
        assert!(cat.index_ingredient(libre).is_none());

        // Supprimer une recette libère ses ingrédients, sans laisser de lignes orphelines.
        let seule = cat
            .ajouter_ingredient("Truffe", Rayon::Epicerie, Unite::G, 900.)
            .unwrap();
        let id = cat
            .enregistrer_recette(&recette("Pâtes à la truffe", vec![(seule, 5.)]))
            .unwrap();
        assert!(est_invalide(cat.supprimer_ingredient(seule)));
        cat.supprimer_recette(id).unwrap();
        cat.supprimer_ingredient(seule).unwrap();
    }
}
