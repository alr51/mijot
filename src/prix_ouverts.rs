//! Suggestions de prix d'après Open Prices, la base ouverte de relevés de prix
//! d'Open Food Facts (<https://prices.openfoodfacts.org>, licence ODbL).
//!
//! C'est le seul endroit de l'application qui parle au réseau, et seulement sur
//! demande explicite. Ne partent que le terme cherché et la catégorie choisie.
//!
//! Un ingrédient est rapproché d'une **catégorie** de la taxonomie Open Food
//! Facts (« Pâtes alimentaires » = `en:pastas`). Deux sortes de relevés existent :
//!
//! - produits bruts (fruits, légumes…), relevés directement au kilo ou à la pièce ;
//! - produits emballés, relevés au paquet : le prix est ramené au kilo ou au
//!   litre grâce à la contenance connue du produit.
//!
//! Les appels sont bloquants : à lancer hors du fil de l'interface.

use std::fmt;
use std::time::Duration;

use serde_json::Value;

use crate::donnees::Unite;

const OPEN_PRICES: &str = "https://prices.openfoodfacts.org/api/v1";
const OPEN_FOOD_FACTS: &str = "https://world.openfoodfacts.org/api/v3";

/// Nom sous lequel l'application se présente. Open Food Facts recommande
/// `NomDeLApplication/version (contact)` : à personnaliser si tu le souhaites.
const AGENT: &str = "meal-planner-desktop/0.1";

/// Relevés demandés par requête : le maximum accepté par l'API.
const PAGE: &str = "100";

/// Catégories proposées pour un terme, après classement.
const CATEGORIES_MAX: usize = 12;

/// En dessous, pas de suggestion : trop peu de relevés pour une médiane crédible.
pub const RELEVES_MIN: usize = 3;

#[derive(Debug)]
pub enum Erreur {
    /// Pas de réponse : hors ligne, délai dépassé, service en panne.
    Reseau(String),
    /// Réponse reçue mais inattendue.
    Reponse(String),
}

impl fmt::Display for Erreur {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Erreur::Reseau(e) => write!(f, "Open Food Facts est injoignable ({e})."),
            Erreur::Reponse(e) => write!(f, "Réponse inattendue d'Open Food Facts ({e})."),
        }
    }
}

/// Un prix relevé en magasin, ramené à l'unité de l'ingrédient.
#[derive(Clone, Debug, PartialEq)]
pub struct Releve {
    /// Euros par kilo, par litre ou par pièce, selon l'unité de l'ingrédient.
    pub prix: f64,
    /// Nom du produit, ou « Vrac » pour un produit brut.
    pub libelle: String,
    /// Enseigne et ville, quand elles sont connues.
    pub magasin: String,
    /// Date du relevé, `AAAA-MM-JJ`.
    pub date: String,
    pub en_france: bool,
}

/// D'où viennent les relevés retenus pour une suggestion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    France,
    /// Trop peu de relevés français : tous les relevés en euros.
    Euro,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Suggestion {
    /// Prix médian, arrondi au centime : la valeur proposée.
    pub mediane: f64,
    /// Premier et troisième quartiles : la moitié des relevés est entre les deux.
    pub bas: f64,
    pub haut: f64,
    pub zone: Zone,
    /// Relevés retenus, du plus récent au plus ancien.
    pub releves: Vec<Releve>,
}

/// Terme à chercher pour un ingrédient : son nom sans les précisions entre
/// parenthèses (« Pois chiches (conserve) » devient « Pois chiches »).
pub fn terme_de_recherche(nom: &str) -> String {
    let mut terme = String::new();
    let mut profondeur = 0usize;
    for c in nom.chars() {
        match c {
            '(' => profondeur += 1,
            ')' => profondeur = profondeur.saturating_sub(1),
            _ if profondeur == 0 => terme.push(c),
            _ => {}
        }
    }
    terme.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn nombre(valeur: &Value) -> Option<f64> {
    match valeur {
        Value::Number(n) => n.as_f64(),
        // Certaines quantités arrivent en texte.
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
    .filter(|n| n.is_finite())
}

fn texte<'a>(valeur: &'a Value, cle: &str) -> &'a str {
    valeur.get(cle).and_then(Value::as_str).unwrap_or("").trim()
}

/// Lit une page de relevés et ramène chaque prix à l'unité de l'ingrédient.
/// Les relevés inexploitables (remise, autre devise, contenance inconnue ou
/// dans une autre grandeur) sont écartés.
fn lire_releves(page: &Value, unite: Unite) -> Vec<Releve> {
    let Some(items) = page.get("items").and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            if texte(item, "currency") != "EUR"
                || item.get("price_is_discounted").and_then(Value::as_bool) == Some(true)
            {
                return None;
            }
            let prix = item.get("price").and_then(nombre).filter(|p| *p > 0.)?;
            let produit = item.get("product").filter(|p| p.is_object());
            let (prix, libelle) = match texte(item, "type") {
                // Produit brut : le relevé dit s'il est au kilo, au litre ou à la pièce.
                "CATEGORY" => {
                    let attendu = match unite {
                        Unite::G => "KILOGRAM",
                        Unite::Ml => "LITER",
                        Unite::Piece => "UNIT",
                    };
                    if texte(item, "price_per") != attendu {
                        return None;
                    }
                    (prix, "Vrac".to_owned())
                }
                // Produit emballé : prix du paquet, à diviser par sa contenance
                // (en grammes ou millilitres). Rien à en tirer pour un ingrédient
                // compté à la pièce : le nombre de pièces du paquet n'est pas connu.
                "PRODUCT" => {
                    let produit = produit?;
                    let attendue = match unite {
                        Unite::G => "g",
                        Unite::Ml => "ml",
                        Unite::Piece => return None,
                    };
                    if texte(produit, "product_quantity_unit") != attendue {
                        return None;
                    }
                    let contenance = produit
                        .get("product_quantity")
                        .and_then(nombre)
                        .filter(|q| *q > 0.)?;
                    let nom = texte(produit, "product_name");
                    let marque = texte(produit, "brands");
                    let libelle = match (nom.is_empty(), marque.is_empty()) {
                        (false, false) => format!("{nom} ({marque})"),
                        (false, true) => nom.to_owned(),
                        (true, false) => marque.to_owned(),
                        (true, true) => "Produit sans nom".to_owned(),
                    };
                    (prix / contenance * 1000., libelle)
                }
                _ => return None,
            };
            let lieu = item.get("location").filter(|l| l.is_object());
            let enseigne = lieu
                .map(|l| match texte(l, "osm_brand") {
                    "" => texte(l, "osm_name"),
                    marque => marque,
                })
                .unwrap_or("");
            let ville = lieu.map(|l| texte(l, "osm_address_city")).unwrap_or("");
            let magasin = match (enseigne.is_empty(), ville.is_empty()) {
                (false, false) => format!("{enseigne}, {ville}"),
                (false, true) => enseigne.to_owned(),
                (true, false) => ville.to_owned(),
                (true, true) => "Magasin inconnu".to_owned(),
            };
            Some(Releve {
                prix,
                libelle,
                magasin,
                date: texte(item, "date").to_owned(),
                en_france: lieu.is_some_and(|l| texte(l, "osm_address_country_code") == "FR"),
            })
        })
        .collect()
}

/// Valeur au rang `part` (entre 0 et 1) d'une série triée, interpolée.
fn quantile(tries: &[f64], part: f64) -> f64 {
    let rang = part * (tries.len() - 1) as f64;
    let (bas, haut) = (rang.floor() as usize, rang.ceil() as usize);
    tries[bas] + (tries[haut] - tries[bas]) * (rang - bas as f64)
}

/// Résume des relevés en une suggestion : les relevés français s'il y en a
/// assez, sinon tous. `None` s'il y a trop peu de relevés pour conclure.
pub fn resumer(releves: &[Releve]) -> Option<Suggestion> {
    let francais: Vec<Releve> = releves.iter().filter(|r| r.en_france).cloned().collect();
    let (zone, mut retenus) = if francais.len() >= RELEVES_MIN {
        (Zone::France, francais)
    } else {
        (Zone::Euro, releves.to_vec())
    };
    if retenus.len() < RELEVES_MIN {
        return None;
    }
    let mut prix: Vec<f64> = retenus.iter().map(|r| r.prix).collect();
    prix.sort_by(f64::total_cmp);
    let centime = |p: f64| (p * 100.).round() / 100.;
    // Les dates ISO se comparent comme du texte.
    retenus.sort_by(|a, b| b.date.cmp(&a.date));
    Some(Suggestion {
        mediane: centime(quantile(&prix, 0.5)),
        bas: centime(quantile(&prix, 0.25)),
        haut: centime(quantile(&prix, 0.75)),
        zone,
        releves: retenus,
    })
}

/// Minuscules sans accents, pour comparer « Pâtes » et « pates ».
fn simplifier(texte: &str) -> String {
    texte
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'ç' => 'c',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            autre => autre,
        })
        .collect()
}

/// Classe les catégories proposées : celles qui commencent par le terme d'abord
/// (« Pâtes alimentaires » avant « Fromages à pâte persillée »), puis les plus
/// courtes, souvent les plus générales. La taxonomie renvoie tout ce qui
/// contient le terme, sans ordre utile.
fn classer_categories(terme: &str, mut noms: Vec<String>) -> Vec<String> {
    let terme = simplifier(terme.trim());
    noms.sort_by_cached_key(|nom| {
        let simple = simplifier(nom);
        (!simple.starts_with(&terme), simple.chars().count(), simple)
    });
    noms.dedup();
    noms
}

fn lire_categories(reponse: &Value) -> Vec<String> {
    reponse
        .get("suggestions")
        .and_then(Value::as_array)
        .map(|noms| {
            noms.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn lire_tag(reponse: &Value) -> Option<String> {
    let tag = reponse.get("canonical_tags")?.as_array()?.first()?;
    if tag.get("exists_in_taxonomy").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    tag.get("tag").and_then(Value::as_str).map(str::to_owned)
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .user_agent(AGENT)
        .build()
        .into()
}

fn appeler(agent: &ureq::Agent, url: &str, parametres: &[(&str, &str)]) -> Result<Value, Erreur> {
    let mut requete = agent.get(url);
    for (cle, valeur) in parametres {
        requete = requete.query(cle, valeur);
    }
    let mut reponse = requete.call().map_err(|e| match e {
        ureq::Error::StatusCode(code) => Erreur::Reponse(format!("code HTTP {code}")),
        autre => Erreur::Reseau(autre.to_string()),
    })?;
    reponse
        .body_mut()
        .read_json::<Value>()
        .map_err(|e| Erreur::Reponse(e.to_string()))
}

/// Place le terme lui-même en tête des catégories proposées, sans doublon.
fn avec_terme_en_tete(terme: &str, mut categories: Vec<String>) -> Vec<String> {
    let simple = simplifier(terme);
    categories.retain(|nom| simplifier(nom) != simple);
    categories.insert(0, terme.to_owned());
    categories
}

/// Noms de catégories Open Food Facts, en français, proches du terme.
///
/// Si le terme est lui-même le nom d'une catégorie (« Pâtes », « Oignons »),
/// il vient en premier : c'est le plus souvent le bon choix. Suivent les
/// catégories dont le nom contient le terme.
pub fn chercher_categories(terme: &str) -> Result<Vec<String>, Erreur> {
    let terme = terme.trim();
    let agent = agent();
    let reponse = appeler(
        &agent,
        &format!("{OPEN_FOOD_FACTS}/taxonomy_suggestions"),
        &[
            ("tagtype", "categories"),
            ("lc", "fr"),
            ("string", terme),
            ("limit", "25"),
        ],
    )?;
    let mut categories = classer_categories(terme, lire_categories(&reponse));
    if identifier(&agent, terme)?.is_some() {
        categories = avec_terme_en_tete(terme, categories);
    }
    categories.truncate(CATEGORIES_MAX);
    Ok(categories)
}

/// Identifiant canonique (`en:pastas`) d'une catégorie nommée en français,
/// ou `None` si la taxonomie ne la connaît pas.
pub fn identifier_categorie(nom: &str) -> Result<Option<String>, Erreur> {
    identifier(&agent(), nom)
}

fn identifier(agent: &ureq::Agent, nom: &str) -> Result<Option<String>, Erreur> {
    let reponse = appeler(
        agent,
        &format!("{OPEN_FOOD_FACTS}/taxonomy_canonicalize_tags"),
        &[
            ("tagtype", "categories"),
            ("lc", "fr"),
            ("local_tags_list", nom),
        ],
    )?;
    Ok(lire_tag(&reponse))
}

/// Relevés récents d'une catégorie, ramenés à l'unité de l'ingrédient.
///
/// Les relevés de produits bruts sont préférés s'ils suffisent : ils portent
/// exactement sur la catégorie, alors que les produits emballés d'une catégorie
/// en débordent souvent (les « tomates » emballées sont surtout des conserves).
pub fn releves(tag: &str, unite: Unite) -> Result<Vec<Releve>, Erreur> {
    let agent = agent();
    let communs = [
        ("currency", "EUR"),
        ("price_is_discounted", "false"),
        ("order_by", "-date"),
        ("size", PAGE),
    ];
    let chercher = |filtre: &str| -> Result<Vec<Releve>, Erreur> {
        let mut parametres = vec![(filtre, tag)];
        parametres.extend(communs);
        let page = appeler(&agent, &format!("{OPEN_PRICES}/prices"), &parametres)?;
        Ok(lire_releves(&page, unite))
    };
    let bruts = chercher("category_tag")?;
    if bruts.len() >= RELEVES_MIN || unite == Unite::Piece {
        return Ok(bruts);
    }
    chercher("product__categories_tags__contains")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn brut(prix: f64, par: &str, pays: &str, date: &str) -> Value {
        json!({
            "type": "CATEGORY", "price": prix, "price_per": par, "currency": "EUR",
            "price_is_discounted": false, "date": date, "product": null,
            "location": {"osm_brand": "Bio C' Bon", "osm_name": "Bio C' Bon Vincennes",
                         "osm_address_city": "Vincennes", "osm_address_country_code": pays},
        })
    }

    fn emballe(prix: f64, contenance: Value, unite: &str) -> Value {
        json!({
            "type": "PRODUCT", "price": prix, "price_per": null, "currency": "EUR",
            "price_is_discounted": false, "date": "2026-11-10",
            "product": {"product_name": "Fusilli", "brands": "Marque", "product_quantity": contenance,
                        "product_quantity_unit": unite},
            "location": {"osm_brand": null, "osm_name": "E. Leclerc", "osm_address_city": null,
                         "osm_address_country_code": "FR"},
        })
    }

    fn releve(prix: f64, en_france: bool, date: &str) -> Releve {
        Releve {
            prix,
            libelle: "Vrac".into(),
            magasin: "Magasin".into(),
            date: date.into(),
            en_france,
        }
    }

    #[test]
    fn termes_de_recherche() {
        assert_eq!(
            terme_de_recherche("Pois chiches (conserve)"),
            "Pois chiches"
        );
        assert_eq!(terme_de_recherche("Ail (gousses)"), "Ail");
        assert_eq!(terme_de_recherche("Pain de mie (tranches) "), "Pain de mie");
        assert_eq!(
            terme_de_recherche("Potiron ou butternut"),
            "Potiron ou butternut"
        );
        assert_eq!(terme_de_recherche("(vide)"), "");
    }

    #[test]
    fn produits_bruts_selon_l_unite() {
        let page = json!({"items": [
            brut(3.8, "KILOGRAM", "FR", "2026-11-05"),
            brut(0.6, "UNIT", "BG", "2026-11-01"),
        ]});
        let au_kilo = lire_releves(&page, Unite::G);
        assert_eq!(au_kilo.len(), 1);
        assert_eq!(
            au_kilo[0],
            Releve {
                prix: 3.8,
                libelle: "Vrac".into(),
                magasin: "Bio C' Bon, Vincennes".into(),
                date: "2026-11-05".into(),
                en_france: true,
            }
        );
        let a_la_piece = lire_releves(&page, Unite::Piece);
        assert_eq!(a_la_piece.len(), 1);
        assert_eq!(a_la_piece[0].prix, 0.6);
        assert!(!a_la_piece[0].en_france);
        assert!(lire_releves(&page, Unite::Ml).is_empty());
    }

    #[test]
    fn produits_emballes_ramenes_au_kilo_ou_au_litre() {
        let page = json!({"items": [
            emballe(1.18, json!(500), "g"),
            // Contenance en texte.
            emballe(2.19, json!("1000"), "g"),
            emballe(0.99, json!(1000), "ml"),
        ]});
        let au_kilo = lire_releves(&page, Unite::G);
        assert_eq!(
            au_kilo.iter().map(|r| r.prix).collect::<Vec<_>>(),
            vec![2.36, 2.19]
        );
        assert_eq!(au_kilo[0].libelle, "Fusilli (Marque)");
        assert_eq!(au_kilo[0].magasin, "E. Leclerc");
        let au_litre = lire_releves(&page, Unite::Ml);
        assert_eq!(
            au_litre.iter().map(|r| r.prix).collect::<Vec<_>>(),
            vec![0.99]
        );
        // À la pièce, un paquet ne dit pas combien il contient de pièces.
        assert!(lire_releves(&page, Unite::Piece).is_empty());
    }

    #[test]
    fn releves_inexploitables_ecartes() {
        let mut remise = brut(1., "KILOGRAM", "FR", "2026-01-01");
        remise["price_is_discounted"] = json!(true);
        let mut dollars = brut(1., "KILOGRAM", "FR", "2026-01-01");
        dollars["currency"] = json!("USD");
        let mut sans_lieu = brut(2., "KILOGRAM", "FR", "2026-01-01");
        sans_lieu["location"] = Value::Null;
        let page = json!({"items": [
            remise,
            dollars,
            brut(0., "KILOGRAM", "FR", "2026-01-01"),
            emballe(1., json!(0), "g"),
            emballe(1., Value::Null, "g"),
            emballe(1., json!(500), ""),
            json!({"type": "PRODUCT", "price": 1.0, "currency": "EUR", "product": null}),
            json!({"type": "AUTRE", "price": 1.0, "currency": "EUR"}),
            json!("n'importe quoi"),
            sans_lieu,
        ]});
        let lus = lire_releves(&page, Unite::G);
        // Seul le relevé sans lieu reste : exploitable, mais pas compté en France.
        assert_eq!(lus.len(), 1);
        assert_eq!(lus[0].magasin, "Magasin inconnu");
        assert!(!lus[0].en_france);
        assert!(lire_releves(&json!({"detail": "erreur"}), Unite::G).is_empty());
    }

    #[test]
    fn resume_des_releves() {
        // Assez de relevés français : eux seuls comptent.
        let releves = vec![
            releve(1., true, "2026-01-01"),
            releve(9., false, "2026-06-01"),
            releve(3., true, "2026-03-01"),
            releve(2., true, "2026-02-01"),
            releve(4., true, "2026-04-01"),
        ];
        let s = resumer(&releves).unwrap();
        assert_eq!(s.zone, Zone::France);
        assert_eq!((s.bas, s.mediane, s.haut), (1.75, 2.5, 3.25));
        // Du plus récent au plus ancien.
        assert_eq!(
            s.releves
                .iter()
                .map(|r| r.date.as_str())
                .collect::<Vec<_>>(),
            vec!["2026-04-01", "2026-03-01", "2026-02-01", "2026-01-01"]
        );

        // Trop peu de relevés français : tous les relevés en euros.
        let s = resumer(&releves[..3]).unwrap();
        assert_eq!(s.zone, Zone::Euro);
        assert_eq!(s.mediane, 3.);
        assert_eq!(s.releves.len(), 3);

        // Trop peu de relevés tout court.
        assert_eq!(resumer(&releves[..2]), None);
        assert_eq!(resumer(&[]), None);

        // Arrondi au centime.
        let s = resumer(&[
            releve(1.111, true, ""),
            releve(2.226, true, ""),
            releve(3.333, true, ""),
        ])
        .unwrap();
        assert_eq!(s.mediane, 2.23);
    }

    #[test]
    fn classement_des_categories() {
        let noms = [
            "Beurres de cacahuètes",
            "Fromages à pâte persillée",
            "Pâtes alimentaires de céréales",
            "Pates farcies",
            "Pâtes alimentaires",
            "Pates farcies",
        ]
        .map(String::from)
        .to_vec();
        assert_eq!(
            classer_categories(" Pâtes ", noms),
            vec![
                "Pates farcies",
                "Pâtes alimentaires",
                "Pâtes alimentaires de céréales",
                "Beurres de cacahuètes",
                "Fromages à pâte persillée",
            ]
        );
        assert!(classer_categories("x", Vec::new()).is_empty());

        // Terme reconnu comme catégorie : en tête, à la place de son doublon.
        assert_eq!(
            avec_terme_en_tete("Pâtes", vec!["Crozets".into(), "pates".into()]),
            vec!["Pâtes", "Crozets"]
        );
    }

    #[test]
    fn reponses_de_la_taxonomie() {
        let suggestions = json!({"status": "success", "suggestions": ["Lentilles corail", "Lentilles décortiquées"]});
        assert_eq!(
            lire_categories(&suggestions),
            vec!["Lentilles corail", "Lentilles décortiquées"]
        );
        assert!(lire_categories(&json!({"errors": []})).is_empty());

        let connue = json!({"canonical_tags": [{"exists_in_taxonomy": true, "tag": "en:pastas"}]});
        assert_eq!(lire_tag(&connue), Some("en:pastas".into()));
        let inconnue = json!({"canonical_tags": [{"exists_in_taxonomy": false, "tag": "fr:truc"}]});
        assert_eq!(lire_tag(&inconnue), None);
        assert_eq!(lire_tag(&json!({"canonical_tags": []})), None);
    }
}
