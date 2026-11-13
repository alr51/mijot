//! Logique sans interface : coût des recettes, composition d'un menu qui tient
//! dans un budget, liste de courses agrégée.

use std::collections::HashMap;

use crate::catalogue::Catalogue;
use crate::donnees::{Categorie, Rayon, Unite};

pub const REPAS_MAX: usize = 14;
pub const PERSONNES_MAX: u32 = 12;

#[derive(Clone, Copy, Debug)]
pub struct Parametres {
    /// Budget de la semaine, en euros.
    pub budget: f64,
    pub personnes: u32,
    pub repas: usize,
    pub vege: bool,
}

#[derive(Debug, PartialEq)]
pub enum Echec {
    /// Même le menu le moins cher dépasse le budget.
    BudgetTropBas {
        minimum: f64,
    },
    PasAssezDeRecettes {
        disponibles: usize,
    },
}

/// Ligne de la liste de courses.
#[derive(Debug, Clone, PartialEq)]
pub struct Ligne {
    /// Position dans [`Catalogue::ingredients`].
    pub ingredient: usize,
    /// Quantité à acheter, dans l'unité de l'ingrédient (pièces arrondies au-dessus).
    pub quantite: f64,
    pub prix: f64,
}

fn prix_de(cat: &Catalogue, ingredient: usize, quantite: f64) -> f64 {
    let i = &cat.ingredients[ingredient];
    match i.unite {
        Unite::G | Unite::Ml => quantite / 1000. * i.prix,
        Unite::Piece => quantite * i.prix,
    }
}

/// Coût d'une recette pour une personne, au prorata des quantités utilisées.
pub fn cout_par_personne(cat: &Catalogue, recette: usize) -> f64 {
    cat.recettes[recette]
        .ingredients
        .iter()
        .filter_map(|(id, q)| cat.index_ingredient(*id).map(|i| prix_de(cat, i, *q)))
        .sum()
}

/// Liste de courses d'un menu : quantités cumulées, groupées par rayon.
///
/// Les pièces (œufs, pâte à tarte, salade…) sont arrondies à l'unité supérieure :
/// on n'achète pas un quart de pâte brisée. Le reste est compté au prorata.
pub fn liste_de_courses(
    cat: &Catalogue,
    menu: &[usize],
    personnes: u32,
) -> Vec<(Rayon, Vec<Ligne>)> {
    let mut quantites: HashMap<usize, f64> = HashMap::new();
    for &recette in menu {
        for (id, q) in &cat.recettes[recette].ingredients {
            if let Some(i) = cat.index_ingredient(*id) {
                *quantites.entry(i).or_default() += q * f64::from(personnes);
            }
        }
    }
    Rayon::TOUS
        .iter()
        .filter_map(|&rayon| {
            let mut lignes: Vec<Ligne> = quantites
                .iter()
                .filter(|(i, _)| cat.ingredients[**i].rayon == rayon)
                .map(|(&ingredient, &q)| {
                    let quantite = match cat.ingredients[ingredient].unite {
                        // Tolérance : 4 × 0,25 doit donner 1, pas 2.
                        Unite::Piece => (q - 1e-9).ceil().max(1.),
                        _ => q,
                    };
                    Ligne {
                        ingredient,
                        quantite,
                        prix: prix_de(cat, ingredient, quantite),
                    }
                })
                .collect();
            // Ordre du catalogue : stable d'une génération à l'autre.
            lignes.sort_by_key(|l| l.ingredient);
            (!lignes.is_empty()).then_some((rayon, lignes))
        })
        .collect()
}

/// Coût estimé d'un menu : le total de sa liste de courses.
pub fn cout_menu(cat: &Catalogue, menu: &[usize], personnes: u32) -> f64 {
    liste_de_courses(cat, menu, personnes)
        .iter()
        .flat_map(|(_, lignes)| lignes)
        .map(|l| l.prix)
        .sum()
}

fn eligibles(cat: &Catalogue, vege: bool) -> Vec<usize> {
    (0..cat.recettes.len())
        .filter(|&i| !vege || cat.recettes[i].vege)
        .collect()
}

/// Compose un menu de `p.repas` recettes distinctes dont le coût tient dans le budget.
///
/// Tirage au hasard (en évitant de répéter la même catégorie), puis, tant que le
/// budget est dépassé, la recette la plus chère est échangée contre une moins chère.
pub fn generer(
    cat: &Catalogue,
    p: &Parametres,
    rng: &mut fastrand::Rng,
) -> Result<Vec<usize>, Echec> {
    let mut candidats = eligibles(cat, p.vege);
    if candidats.len() < p.repas {
        return Err(Echec::PasAssezDeRecettes {
            disponibles: candidats.len(),
        });
    }
    if p.repas == 0 {
        return Ok(Vec::new());
    }

    // Borne basse : les recettes les moins chères.
    let mut par_prix = candidats.clone();
    par_prix.sort_by(|a, b| cout_par_personne(cat, *a).total_cmp(&cout_par_personne(cat, *b)));
    let moins_cher: Vec<usize> = par_prix[..p.repas].to_vec();
    let minimum = cout_menu(cat, &moins_cher, p.personnes);
    if minimum > p.budget {
        return Err(Echec::BudgetTropBas { minimum });
    }

    rng.shuffle(&mut candidats);
    // Au plus un repas sur trois de la même catégorie (deux au minimum).
    let plafond = p.repas.div_ceil(3).max(2);
    let mut par_categorie: HashMap<Categorie, usize> = HashMap::new();
    let mut menu: Vec<usize> = Vec::with_capacity(p.repas);
    for &r in &candidats {
        if menu.len() == p.repas {
            break;
        }
        let n = par_categorie.entry(cat.recettes[r].categorie).or_default();
        if *n < plafond {
            *n += 1;
            menu.push(r);
        }
    }
    // Catalogue trop peu varié pour le plafond : on complète sans lui.
    for &r in &candidats {
        if menu.len() == p.repas {
            break;
        }
        if !menu.contains(&r) {
            menu.push(r);
        }
    }

    while cout_menu(cat, &menu, p.personnes) > p.budget {
        let (position, plus_chere) = menu
            .iter()
            .copied()
            .enumerate()
            .max_by(|a, b| cout_par_personne(cat, a.1).total_cmp(&cout_par_personne(cat, b.1)))
            .expect("menu non vide");
        let seuil = cout_par_personne(cat, plus_chere);
        let moins_cheres: Vec<usize> = candidats
            .iter()
            .copied()
            .filter(|r| !menu.contains(r) && cout_par_personne(cat, *r) < seuil)
            .collect();
        if moins_cheres.is_empty() {
            // Plus rien à échanger : le menu le moins cher, dont on sait qu'il passe.
            let mut menu = moins_cher;
            rng.shuffle(&mut menu);
            return Ok(menu);
        }
        menu[position] = moins_cheres[rng.usize(..moins_cheres.len())];
    }
    Ok(menu)
}

/// Recettes pouvant remplacer celle en `position` sans dépasser le budget ni
/// doublonner un autre repas du menu.
pub fn remplacantes(
    cat: &Catalogue,
    menu: &[usize],
    position: usize,
    p: &Parametres,
) -> Vec<usize> {
    let mut essai = menu.to_vec();
    eligibles(cat, p.vege)
        .into_iter()
        .filter(|r| !menu.contains(r))
        .filter(|&r| {
            essai[position] = r;
            cout_menu(cat, &essai, p.personnes) <= p.budget
        })
        .collect()
}

/// Montant en euros, à la française : `12,50 €`.
pub fn euros(montant: f64) -> String {
    format!("{montant:.2} €").replace('.', ",")
}

/// Quantité lisible : `350 g`, `1,2 kg`, `25 cl`, `× 3`.
pub fn quantite_lisible(quantite: f64, unite: Unite) -> String {
    match unite {
        Unite::Piece => format!("× {}", quantite.round() as u64),
        Unite::G if quantite >= 1000. => format!("{:.1} kg", quantite / 1000.).replace('.', ","),
        Unite::G if quantite >= 20. => format!("{} g", (quantite / 10.).round() as u64 * 10),
        Unite::G => format!("{} g", quantite.round() as u64),
        Unite::Ml if quantite >= 1000. => format!("{:.1} l", quantite / 1000.).replace('.', ","),
        Unite::Ml if quantite >= 100. => format!("{} cl", (quantite / 10.).round() as u64),
        Unite::Ml => format!("{} ml", quantite.round() as u64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::donnees::{index_ingredient, index_recette};

    fn params(budget: f64, repas: usize, vege: bool) -> Parametres {
        Parametres {
            budget,
            personnes: 4,
            repas,
            vege,
        }
    }

    #[test]
    fn le_menu_tient_dans_le_budget() {
        // Catalogue de départ : positions identiques à celles de `donnees`.
        let cat = Catalogue::en_memoire().unwrap();
        for graine in 0..200 {
            let mut rng = fastrand::Rng::with_seed(graine);
            for budget in [45., 60., 90., 200.] {
                let p = params(budget, 7, false);
                let menu = generer(&cat, &p, &mut rng).expect("budget suffisant");
                assert_eq!(menu.len(), 7);
                let mut distinctes = menu.clone();
                distinctes.sort_unstable();
                distinctes.dedup();
                assert_eq!(distinctes.len(), 7, "recettes en double");
                assert!(
                    cout_menu(&cat, &menu, 4) <= budget,
                    "graine {graine}, budget {budget}"
                );
            }
        }
    }

    #[test]
    fn budget_juste_suffisant() {
        // Catalogue de départ : positions identiques à celles de `donnees`.
        let cat = Catalogue::en_memoire().unwrap();
        let Err(Echec::BudgetTropBas { minimum }) = generer(
            &cat,
            &params(1., 7, false),
            &mut fastrand::Rng::with_seed(1),
        ) else {
            panic!("un euro ne suffit pas");
        };
        for graine in 0..50 {
            let mut rng = fastrand::Rng::with_seed(graine);
            let menu =
                generer(&cat, &params(minimum, 7, false), &mut rng).expect("minimum accepté");
            assert!(cout_menu(&cat, &menu, 4) <= minimum + 1e-9);
        }
        assert!(matches!(
            generer(
                &cat,
                &params(minimum - 0.01, 7, false),
                &mut fastrand::Rng::with_seed(1)
            ),
            Err(Echec::BudgetTropBas { .. })
        ));
    }

    #[test]
    fn menu_vegetarien() {
        // Catalogue de départ : positions identiques à celles de `donnees`.
        let cat = Catalogue::en_memoire().unwrap();
        let mut rng = fastrand::Rng::with_seed(7);
        let menu = generer(&cat, &params(150., REPAS_MAX, true), &mut rng).unwrap();
        assert_eq!(menu.len(), REPAS_MAX);
        assert!(menu.iter().all(|&r| cat.recettes[r].vege));
    }

    #[test]
    fn varie_les_categories_quand_le_budget_le_permet() {
        // Catalogue de départ : positions identiques à celles de `donnees`.
        let cat = Catalogue::en_memoire().unwrap();
        for graine in 0..50 {
            let mut rng = fastrand::Rng::with_seed(graine);
            let menu = generer(&cat, &params(500., 7, false), &mut rng).unwrap();
            let mut compte: HashMap<Categorie, usize> = HashMap::new();
            for r in menu {
                *compte.entry(cat.recettes[r].categorie).or_default() += 1;
            }
            assert!(compte.values().all(|n| *n <= 3));
        }
    }

    #[test]
    fn liste_cumule_et_arrondit_les_pieces() {
        // Catalogue de départ : positions identiques à celles de `donnees`.
        let cat = Catalogue::en_memoire().unwrap();
        let carbonara = index_recette("carbonara").unwrap();
        let quiche = index_recette("quiche_lorraine").unwrap();
        let liste = liste_de_courses(&cat, &[carbonara, quiche], 4);
        let ligne = |id: &str| {
            let i = index_ingredient(id).unwrap();
            liste
                .iter()
                .flat_map(|(_, l)| l)
                .find(|l| l.ingredient == i)
                .unwrap_or_else(|| panic!("{id} absent"))
                .clone()
        };
        // 60 g par personne dans chaque recette.
        assert_eq!(ligne("lardons").quantite, 480.);
        assert!((ligne("lardons").prix - 4.32).abs() < 1e-9);
        assert_eq!(ligne("oeuf").quantite, 8.);
        // Un quart de pâte par personne : une pâte pour quatre, deux pour cinq.
        assert_eq!(ligne("pate_brisee").quantite, 1.);
        let pour_cinq = liste_de_courses(&cat, &[quiche], 5);
        let pate = index_ingredient("pate_brisee").unwrap();
        let ligne = pour_cinq
            .iter()
            .flat_map(|(_, l)| l)
            .find(|l| l.ingredient == pate)
            .unwrap();
        assert_eq!(ligne.quantite, 2.);
        // Rayons dans l'ordre du magasin, sans rayon vide.
        let rayons: Vec<Rayon> = liste.iter().map(|(r, _)| *r).collect();
        assert_eq!(
            rayons,
            vec![Rayon::ViandesPoissons, Rayon::Frais, Rayon::Epicerie]
        );
    }

    #[test]
    fn remplacantes_respectent_le_budget() {
        // Catalogue de départ : positions identiques à celles de `donnees`.
        let cat = Catalogue::en_memoire().unwrap();
        let mut rng = fastrand::Rng::with_seed(3);
        let p = params(55., 7, false);
        let menu = generer(&cat, &p, &mut rng).unwrap();
        for position in 0..menu.len() {
            for r in remplacantes(&cat, &menu, position, &p) {
                assert!(!menu.contains(&r));
                let mut essai = menu.clone();
                essai[position] = r;
                assert!(cout_menu(&cat, &essai, 4) <= 55.);
            }
        }
    }

    #[test]
    fn formats() {
        assert_eq!(euros(12.5), "12,50 €");
        assert_eq!(quantite_lisible(350., Unite::G), "350 g");
        assert_eq!(quantite_lisible(1250., Unite::G), "1,2 kg");
        assert_eq!(quantite_lisible(12., Unite::G), "12 g");
        assert_eq!(quantite_lisible(240., Unite::Ml), "24 cl");
        assert_eq!(quantite_lisible(1500., Unite::Ml), "1,5 l");
        assert_eq!(quantite_lisible(3., Unite::Piece), "× 3");
    }
}
