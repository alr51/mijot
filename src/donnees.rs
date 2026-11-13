//! Catalogue de départ : ingrédients (avec prix moyens estimés) et recettes.
//!
//! Il ne sert qu'à remplir la base SQLite à sa création (voir `catalogue.rs`) ;
//! ensuite, prix et recettes vivent dans la base et se modifient depuis l'application.
//!
//! Les quantités des recettes sont données **par personne**. Les prix sont des
//! moyennes de grande surface en France : ils servent à comparer les menus et à
//! tenir un budget, pas à prédire un ticket de caisse au centime.

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Rayon {
    FruitsLegumes,
    ViandesPoissons,
    Frais,
    Epicerie,
    Boulangerie,
}

impl Rayon {
    pub const TOUS: [Rayon; 5] = [
        Rayon::FruitsLegumes,
        Rayon::ViandesPoissons,
        Rayon::Frais,
        Rayon::Epicerie,
        Rayon::Boulangerie,
    ];

    pub fn nom(self) -> &'static str {
        match self {
            Rayon::FruitsLegumes => "Fruits et légumes",
            Rayon::ViandesPoissons => "Viandes et poissons",
            Rayon::Frais => "Frais et crèmerie",
            Rayon::Epicerie => "Épicerie",
            Rayon::Boulangerie => "Boulangerie",
        }
    }

    /// Code enregistré en base.
    pub fn code(self) -> &'static str {
        match self {
            Rayon::FruitsLegumes => "fruits_legumes",
            Rayon::ViandesPoissons => "viandes_poissons",
            Rayon::Frais => "frais",
            Rayon::Epicerie => "epicerie",
            Rayon::Boulangerie => "boulangerie",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|r| r.code() == code)
    }
}

/// Unité de quantité d'un ingrédient ; fixe aussi l'unité du prix.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Unite {
    /// Quantité en grammes, prix au kilo.
    G,
    /// Quantité en millilitres, prix au litre.
    Ml,
    /// Quantité en pièces, prix à la pièce. Arrondie à l'unité supérieure à l'achat.
    Piece,
}

impl Unite {
    pub const TOUTES: [Unite; 3] = [Unite::G, Unite::Ml, Unite::Piece];

    /// Code enregistré en base.
    pub fn code(self) -> &'static str {
        match self {
            Unite::G => "g",
            Unite::Ml => "ml",
            Unite::Piece => "piece",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|u| u.code() == code)
    }

    /// Façon de vendre l'ingrédient, pour un formulaire.
    pub fn nom(self) -> &'static str {
        match self {
            Unite::G => "Au poids (g)",
            Unite::Ml => "Au volume (ml)",
            Unite::Piece => "À la pièce",
        }
    }

    /// Unité du prix : `€ / kg`.
    pub fn prix_par(self) -> &'static str {
        match self {
            Unite::G => "€ / kg",
            Unite::Ml => "€ / l",
            Unite::Piece => "€ / pièce",
        }
    }

    /// Unité des quantités d'une recette.
    pub fn abrege(self) -> &'static str {
        match self {
            Unite::G => "g",
            Unite::Ml => "ml",
            Unite::Piece => "pièce",
        }
    }
}

pub struct Ingredient {
    pub id: &'static str,
    pub nom: &'static str,
    pub rayon: Rayon,
    pub unite: Unite,
    /// Prix en euros : au kilo, au litre ou à la pièce selon l'unité.
    pub prix: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Categorie {
    Pates,
    Riz,
    Four,
    Mijote,
    Soupe,
    Poisson,
    Express,
}

impl Categorie {
    pub const TOUTES: [Categorie; 7] = [
        Categorie::Pates,
        Categorie::Riz,
        Categorie::Four,
        Categorie::Mijote,
        Categorie::Soupe,
        Categorie::Poisson,
        Categorie::Express,
    ];

    /// Code enregistré en base.
    pub fn code(self) -> &'static str {
        match self {
            Categorie::Pates => "pates",
            Categorie::Riz => "riz",
            Categorie::Four => "four",
            Categorie::Mijote => "mijote",
            Categorie::Soupe => "soupe",
            Categorie::Poisson => "poisson",
            Categorie::Express => "express",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|c| c.code() == code)
    }

    pub fn nom(self) -> &'static str {
        match self {
            Categorie::Pates => "Pâtes",
            Categorie::Riz => "Riz et céréales",
            Categorie::Four => "Au four",
            Categorie::Mijote => "Mijoté",
            Categorie::Soupe => "Soupe",
            Categorie::Poisson => "Poisson",
            Categorie::Express => "Express",
        }
    }
}

pub struct Recette {
    /// Nom court, stable : il ne sert qu'à désigner une recette dans les tests.
    #[cfg_attr(not(test), allow(dead_code))]
    pub id: &'static str,
    pub nom: &'static str,
    pub categorie: Categorie,
    pub vege: bool,
    /// Temps total (préparation et cuisson), en minutes.
    pub minutes: u32,
    /// (identifiant d'ingrédient, quantité par personne dans l'unité de l'ingrédient).
    pub ingredients: &'static [(&'static str, f64)],
    pub etapes: &'static [&'static str],
}

const fn ing(
    id: &'static str,
    nom: &'static str,
    rayon: Rayon,
    unite: Unite,
    prix: f64,
) -> Ingredient {
    Ingredient {
        id,
        nom,
        rayon,
        unite,
        prix,
    }
}

use Rayon::{
    Boulangerie as BO, Epicerie as EP, Frais as FR, FruitsLegumes as FL, ViandesPoissons as VP,
};
use Unite::{G, Ml, Piece};

pub static INGREDIENTS: &[Ingredient] = &[
    // Fruits et légumes
    ing("oignon", "Oignons", FL, G, 2.2),
    ing("ail", "Ail (gousses)", FL, Piece, 0.10),
    ing("carotte", "Carottes", FL, G, 1.8),
    ing("pomme_de_terre", "Pommes de terre", FL, G, 1.6),
    ing("tomate", "Tomates", FL, G, 3.5),
    ing("courgette", "Courgettes", FL, G, 2.8),
    ing("aubergine", "Aubergines", FL, G, 3.2),
    ing("poivron", "Poivrons", FL, G, 3.8),
    ing("champignons", "Champignons de Paris", FL, G, 6.5),
    ing("epinards", "Épinards frais", FL, G, 6.0),
    ing("poireau", "Poireaux", FL, G, 3.0),
    ing("brocoli", "Brocoli", FL, G, 3.5),
    ing("chou_fleur", "Chou-fleur", FL, G, 3.0),
    ing("potiron", "Potiron ou butternut", FL, G, 2.5),
    ing("patate_douce", "Patates douces", FL, G, 3.0),
    ing("salade", "Salade verte", FL, Piece, 1.30),
    ing("citron", "Citrons", FL, Piece, 0.60),
    ing("concombre", "Concombre", FL, Piece, 1.20),
    // Viandes et poissons
    ing("poulet", "Filets de poulet", VP, G, 12.0),
    ing("cuisse_poulet", "Cuisses de poulet", VP, G, 6.5),
    ing("dinde", "Escalopes de dinde", VP, G, 11.0),
    ing("boeuf_hache", "Bœuf haché", VP, G, 11.0),
    ing("lardons", "Lardons", VP, G, 9.0),
    ing("jambon", "Jambon blanc (tranches)", VP, Piece, 0.45),
    ing("saucisse", "Saucisses", VP, G, 10.0),
    ing("chorizo", "Chorizo", VP, G, 14.0),
    ing("saumon", "Pavés de saumon", VP, G, 22.0),
    ing("colin", "Filets de colin", VP, G, 13.0),
    // Frais et crèmerie
    ing("oeuf", "Œufs", FR, Piece, 0.30),
    ing("lait", "Lait", FR, Ml, 1.1),
    ing("creme", "Crème fraîche", FR, Ml, 4.0),
    ing("beurre", "Beurre", FR, G, 9.5),
    ing("emmental", "Emmental râpé", FR, G, 9.0),
    ing("parmesan", "Parmesan", FR, G, 20.0),
    ing("mozzarella", "Mozzarella", FR, G, 8.0),
    ing("feta", "Feta", FR, G, 11.0),
    ing("chevre", "Bûche de chèvre", FR, G, 12.0),
    ing("yaourt", "Yaourts nature", FR, Piece, 0.25),
    ing("pate_brisee", "Pâte brisée", FR, Piece, 1.20),
    ing("pate_feuilletee", "Pâte feuilletée", FR, Piece, 1.30),
    ing("pate_pizza", "Pâte à pizza", FR, Piece, 1.50),
    ing("gnocchi", "Gnocchis", FR, G, 4.5),
    ing("tofu", "Tofu ferme", FR, G, 9.0),
    // Épicerie
    ing("pates", "Pâtes", EP, G, 1.8),
    ing("riz", "Riz", EP, G, 2.2),
    ing("riz_arborio", "Riz à risotto", EP, G, 3.5),
    ing("semoule", "Semoule", EP, G, 2.2),
    ing("nouilles", "Nouilles de blé", EP, G, 5.0),
    ing("lentilles_vertes", "Lentilles vertes", EP, G, 3.5),
    ing("lentilles_corail", "Lentilles corail", EP, G, 4.5),
    ing("pois_chiches", "Pois chiches (conserve)", EP, G, 3.5),
    ing("haricots_rouges", "Haricots rouges (conserve)", EP, G, 3.2),
    ing("mais", "Maïs (conserve)", EP, G, 4.0),
    ing("thon", "Thon (conserve)", EP, G, 11.0),
    ing("tomates_concassees", "Tomates concassées", EP, G, 2.2),
    ing("coulis_tomate", "Coulis de tomate", EP, G, 2.5),
    ing("lait_coco", "Lait de coco", EP, Ml, 4.5),
    ing("farine", "Farine", EP, G, 1.1),
    ing("huile_olive", "Huile d'olive", EP, Ml, 9.0),
    ing("sauce_soja", "Sauce soja", EP, Ml, 8.0),
    ing("moutarde", "Moutarde", EP, G, 5.0),
    ing("bouillon", "Bouillon (cubes)", EP, Piece, 0.15),
    ing("tortilla", "Tortillas de blé", EP, Piece, 0.35),
    ing("curry", "Curry en poudre", EP, G, 30.0),
    ing("cumin", "Cumin", EP, G, 30.0),
    ing("paprika", "Paprika", EP, G, 30.0),
    ing("herbes", "Herbes de Provence", EP, G, 30.0),
    // Boulangerie
    ing("pain", "Baguette", BO, Piece, 1.10),
    ing("pain_mie", "Pain de mie (tranches)", BO, Piece, 0.10),
];

pub static RECETTES: &[Recette] = &[
    Recette {
        id: "carbonara",
        nom: "Pâtes à la carbonara",
        categorie: Categorie::Pates,
        vege: false,
        minutes: 20,
        ingredients: &[
            ("pates", 100.),
            ("lardons", 60.),
            ("oeuf", 1.),
            ("parmesan", 20.),
        ],
        etapes: &[
            "Cuire les pâtes dans une grande casserole d'eau salée.",
            "Faire dorer les lardons à la poêle, sans matière grasse.",
            "Battre les œufs avec le parmesan râpé et du poivre.",
            "Égoutter les pâtes en gardant un peu d'eau de cuisson, les mêler aux lardons hors du feu, puis verser les œufs en remuant. Détendre avec l'eau de cuisson.",
        ],
    },
    Recette {
        id: "bolognaise",
        nom: "Pâtes à la bolognaise",
        categorie: Categorie::Pates,
        vege: false,
        minutes: 40,
        ingredients: &[
            ("pates", 100.),
            ("boeuf_hache", 90.),
            ("tomates_concassees", 130.),
            ("oignon", 40.),
            ("carotte", 40.),
            ("ail", 0.5),
        ],
        etapes: &[
            "Faire revenir l'oignon, l'ail et la carotte coupés en petits dés.",
            "Ajouter le bœuf haché et le faire colorer en l'émiettant.",
            "Verser les tomates concassées, saler, poivrer et laisser mijoter 25 minutes.",
            "Cuire les pâtes et les servir nappées de sauce.",
        ],
    },
    Recette {
        id: "pates_courgette_chevre",
        nom: "Pâtes aux courgettes et au chèvre",
        categorie: Categorie::Pates,
        vege: true,
        minutes: 20,
        ingredients: &[
            ("pates", 100.),
            ("courgette", 150.),
            ("chevre", 40.),
            ("ail", 0.5),
            ("huile_olive", 10.),
        ],
        etapes: &[
            "Cuire les pâtes.",
            "Faire sauter les courgettes en demi-rondelles avec l'ail dans l'huile d'olive, une dizaine de minutes.",
            "Mêler les pâtes aux courgettes avec un peu d'eau de cuisson, puis émietter le chèvre par-dessus.",
        ],
    },
    Recette {
        id: "pates_tomate",
        nom: "Pâtes à la sauce tomate et à l'ail",
        categorie: Categorie::Pates,
        vege: true,
        minutes: 20,
        ingredients: &[
            ("pates", 110.),
            ("tomates_concassees", 150.),
            ("ail", 1.),
            ("huile_olive", 10.),
            ("parmesan", 15.),
        ],
        etapes: &[
            "Faire blondir l'ail émincé dans l'huile d'olive.",
            "Ajouter les tomates concassées, saler et laisser réduire 15 minutes.",
            "Cuire les pâtes, les mêler à la sauce et servir avec le parmesan.",
        ],
    },
    Recette {
        id: "gratin_pates_jambon",
        nom: "Gratin de pâtes au jambon",
        categorie: Categorie::Four,
        vege: false,
        minutes: 30,
        ingredients: &[
            ("pates", 90.),
            ("jambon", 1.5),
            ("creme", 60.),
            ("emmental", 35.),
        ],
        etapes: &[
            "Cuire les pâtes en les gardant un peu fermes.",
            "Les mélanger avec la crème, le jambon coupé en lanières et la moitié du fromage.",
            "Verser dans un plat, couvrir du reste de fromage et gratiner 15 minutes à 200 °C.",
        ],
    },
    Recette {
        id: "pates_thon",
        nom: "Pâtes au thon et à la tomate",
        categorie: Categorie::Pates,
        vege: false,
        minutes: 20,
        ingredients: &[
            ("pates", 100.),
            ("thon", 60.),
            ("tomates_concassees", 120.),
            ("oignon", 30.),
        ],
        etapes: &[
            "Faire revenir l'oignon émincé, puis ajouter les tomates et laisser réduire 10 minutes.",
            "Ajouter le thon égoutté et émietté, juste pour le réchauffer.",
            "Cuire les pâtes et les mêler à la sauce.",
        ],
    },
    Recette {
        id: "risotto_champignons",
        nom: "Risotto aux champignons",
        categorie: Categorie::Riz,
        vege: true,
        minutes: 35,
        ingredients: &[
            ("riz_arborio", 80.),
            ("champignons", 120.),
            ("oignon", 30.),
            ("bouillon", 0.5),
            ("parmesan", 20.),
            ("beurre", 10.),
        ],
        etapes: &[
            "Préparer le bouillon et le garder chaud. Faire dorer les champignons émincés, les réserver.",
            "Faire suer l'oignon dans le beurre, ajouter le riz et le nacrer 2 minutes.",
            "Mouiller louche par louche avec le bouillon chaud en remuant, environ 18 minutes.",
            "Ajouter les champignons et le parmesan, couvrir 2 minutes avant de servir.",
        ],
    },
    Recette {
        id: "riz_saute_poulet",
        nom: "Riz sauté au poulet et aux légumes",
        categorie: Categorie::Riz,
        vege: false,
        minutes: 25,
        ingredients: &[
            ("riz", 80.),
            ("poulet", 90.),
            ("carotte", 50.),
            ("poivron", 50.),
            ("oeuf", 0.5),
            ("sauce_soja", 10.),
            ("huile_olive", 8.),
        ],
        etapes: &[
            "Cuire le riz, l'égoutter et le laisser refroidir (idéalement cuit la veille).",
            "Faire sauter le poulet en dés, puis les légumes coupés fin.",
            "Pousser le tout sur le côté, brouiller les œufs dans la poêle.",
            "Ajouter le riz et la sauce soja, faire sauter à feu vif 3 minutes.",
        ],
    },
    Recette {
        id: "curry_poulet_coco",
        nom: "Curry de poulet au lait de coco",
        categorie: Categorie::Mijote,
        vege: false,
        minutes: 30,
        ingredients: &[
            ("poulet", 120.),
            ("lait_coco", 100.),
            ("riz", 80.),
            ("oignon", 40.),
            ("tomates_concassees", 50.),
            ("curry", 3.),
        ],
        etapes: &[
            "Faire revenir l'oignon émincé, ajouter le poulet en morceaux et le curry.",
            "Verser les tomates et le lait de coco, laisser mijoter 15 minutes.",
            "Cuire le riz pendant ce temps et servir avec le curry.",
        ],
    },
    Recette {
        id: "dahl",
        nom: "Dahl de lentilles corail",
        categorie: Categorie::Mijote,
        vege: true,
        minutes: 30,
        ingredients: &[
            ("lentilles_corail", 80.),
            ("lait_coco", 80.),
            ("tomates_concassees", 80.),
            ("oignon", 40.),
            ("ail", 0.5),
            ("curry", 3.),
            ("riz", 60.),
        ],
        etapes: &[
            "Faire revenir l'oignon et l'ail avec le curry.",
            "Ajouter les lentilles rincées, les tomates, le lait de coco et deux fois le volume des lentilles en eau.",
            "Laisser mijoter 20 minutes en remuant, jusqu'à ce que les lentilles se défassent. Saler en fin de cuisson.",
            "Servir avec le riz.",
        ],
    },
    Recette {
        id: "curry_pois_chiches",
        nom: "Curry de pois chiches aux épinards",
        categorie: Categorie::Mijote,
        vege: true,
        minutes: 25,
        ingredients: &[
            ("pois_chiches", 120.),
            ("epinards", 80.),
            ("tomates_concassees", 100.),
            ("lait_coco", 60.),
            ("oignon", 40.),
            ("curry", 3.),
            ("riz", 70.),
        ],
        etapes: &[
            "Faire revenir l'oignon avec le curry.",
            "Ajouter les tomates, le lait de coco et les pois chiches égouttés, mijoter 15 minutes.",
            "Ajouter les épinards et les laisser tomber 2 minutes.",
            "Servir avec le riz.",
        ],
    },
    Recette {
        id: "chili_sin_carne",
        nom: "Chili sin carne",
        categorie: Categorie::Mijote,
        vege: true,
        minutes: 35,
        ingredients: &[
            ("haricots_rouges", 120.),
            ("mais", 40.),
            ("tomates_concassees", 150.),
            ("poivron", 60.),
            ("oignon", 40.),
            ("riz", 70.),
            ("cumin", 2.),
        ],
        etapes: &[
            "Faire revenir l'oignon et le poivron en dés avec le cumin.",
            "Ajouter les tomates, les haricots et le maïs égouttés, mijoter 25 minutes.",
            "Servir avec le riz.",
        ],
    },
    Recette {
        id: "chili_con_carne",
        nom: "Chili con carne",
        categorie: Categorie::Mijote,
        vege: false,
        minutes: 40,
        ingredients: &[
            ("boeuf_hache", 90.),
            ("haricots_rouges", 100.),
            ("tomates_concassees", 130.),
            ("oignon", 40.),
            ("poivron", 40.),
            ("riz", 70.),
            ("cumin", 2.),
        ],
        etapes: &[
            "Faire revenir l'oignon et le poivron, puis le bœuf haché avec le cumin.",
            "Ajouter les tomates et les haricots égouttés, mijoter 30 minutes.",
            "Servir avec le riz.",
        ],
    },
    Recette {
        id: "tortilla",
        nom: "Tortilla de pommes de terre et salade",
        categorie: Categorie::Express,
        vege: true,
        minutes: 30,
        ingredients: &[
            ("oeuf", 2.),
            ("pomme_de_terre", 200.),
            ("oignon", 40.),
            ("huile_olive", 15.),
            ("salade", 0.25),
        ],
        etapes: &[
            "Faire fondre à la poêle les pommes de terre en fines rondelles et l'oignon, à feu doux, 15 minutes.",
            "Les mêler aux œufs battus et salés, puis reverser dans la poêle.",
            "Cuire 5 minutes, retourner à l'aide d'une assiette et cuire encore 3 minutes.",
            "Servir avec la salade.",
        ],
    },
    Recette {
        id: "quiche_poireaux",
        nom: "Quiche aux poireaux",
        categorie: Categorie::Four,
        vege: true,
        minutes: 45,
        ingredients: &[
            ("pate_brisee", 0.25),
            ("poireau", 150.),
            ("oeuf", 1.),
            ("creme", 60.),
            ("emmental", 25.),
        ],
        etapes: &[
            "Faire fondre les poireaux émincés à la poêle, une dizaine de minutes.",
            "Foncer un moule avec la pâte et la piquer à la fourchette.",
            "Battre les œufs avec la crème, ajouter les poireaux et le fromage, verser sur la pâte.",
            "Cuire 30 minutes à 180 °C.",
        ],
    },
    Recette {
        id: "quiche_lorraine",
        nom: "Quiche lorraine",
        categorie: Categorie::Four,
        vege: false,
        minutes: 45,
        ingredients: &[
            ("pate_brisee", 0.25),
            ("lardons", 60.),
            ("oeuf", 1.),
            ("creme", 60.),
            ("lait", 40.),
            ("emmental", 20.),
        ],
        etapes: &[
            "Foncer un moule avec la pâte et la piquer à la fourchette.",
            "Faire dorer les lardons et les répartir sur la pâte.",
            "Battre les œufs avec la crème et le lait, verser, parsemer de fromage.",
            "Cuire 30 minutes à 180 °C.",
        ],
    },
    Recette {
        id: "gratin_dauphinois",
        nom: "Gratin dauphinois et salade",
        categorie: Categorie::Four,
        vege: true,
        minutes: 70,
        ingredients: &[
            ("pomme_de_terre", 300.),
            ("creme", 80.),
            ("lait", 80.),
            ("ail", 0.5),
            ("salade", 0.25),
        ],
        etapes: &[
            "Frotter le plat avec l'ail. Couper les pommes de terre en fines rondelles, sans les rincer.",
            "Les ranger dans le plat en salant entre les couches, puis verser le lait et la crème mélangés.",
            "Cuire 1 heure à 160 °C, jusqu'à ce que le dessus soit doré.",
            "Servir avec la salade.",
        ],
    },
    Recette {
        id: "hachis_parmentier",
        nom: "Hachis parmentier",
        categorie: Categorie::Four,
        vege: false,
        minutes: 50,
        ingredients: &[
            ("pomme_de_terre", 280.),
            ("boeuf_hache", 100.),
            ("oignon", 40.),
            ("lait", 50.),
            ("beurre", 10.),
            ("emmental", 20.),
        ],
        etapes: &[
            "Cuire les pommes de terre à l'eau, les écraser en purée avec le lait et le beurre.",
            "Faire revenir l'oignon puis le bœuf haché, saler et poivrer.",
            "Étaler la viande dans un plat, couvrir de purée et de fromage.",
            "Gratiner 20 minutes à 200 °C.",
        ],
    },
    Recette {
        id: "poulet_roti",
        nom: "Cuisses de poulet rôties et pommes de terre",
        categorie: Categorie::Four,
        vege: false,
        minutes: 55,
        ingredients: &[
            ("cuisse_poulet", 220.),
            ("pomme_de_terre", 250.),
            ("ail", 1.),
            ("huile_olive", 10.),
            ("herbes", 1.),
        ],
        etapes: &[
            "Couper les pommes de terre en quartiers et les mettre dans un plat avec l'ail en chemise.",
            "Poser les cuisses de poulet dessus, arroser d'huile, saler et parsemer d'herbes.",
            "Rôtir 45 minutes à 200 °C en arrosant à mi-cuisson.",
        ],
    },
    Recette {
        id: "saucisses_lentilles",
        nom: "Saucisses aux lentilles",
        categorie: Categorie::Mijote,
        vege: false,
        minutes: 40,
        ingredients: &[
            ("saucisse", 120.),
            ("lentilles_vertes", 80.),
            ("carotte", 60.),
            ("oignon", 40.),
            ("bouillon", 0.5),
        ],
        etapes: &[
            "Faire dorer les saucisses, les réserver.",
            "Faire revenir l'oignon et les carottes en rondelles, ajouter les lentilles et couvrir de bouillon.",
            "Cuire 25 minutes à couvert, remettre les saucisses 5 minutes avant la fin.",
        ],
    },
    Recette {
        id: "veloute_potiron",
        nom: "Velouté de potiron",
        categorie: Categorie::Soupe,
        vege: true,
        minutes: 35,
        ingredients: &[
            ("potiron", 300.),
            ("pomme_de_terre", 80.),
            ("oignon", 40.),
            ("creme", 30.),
            ("bouillon", 0.5),
            ("pain", 0.25),
        ],
        etapes: &[
            "Faire suer l'oignon, ajouter le potiron et les pommes de terre en cubes.",
            "Couvrir de bouillon et cuire 25 minutes.",
            "Mixer avec la crème, rectifier l'assaisonnement et servir avec le pain.",
        ],
    },
    Recette {
        id: "soupe_poireaux",
        nom: "Soupe poireaux-pommes de terre",
        categorie: Categorie::Soupe,
        vege: true,
        minutes: 35,
        ingredients: &[
            ("poireau", 150.),
            ("pomme_de_terre", 150.),
            ("bouillon", 0.5),
            ("creme", 20.),
            ("pain", 0.25),
        ],
        etapes: &[
            "Faire fondre les poireaux émincés, ajouter les pommes de terre en cubes.",
            "Couvrir de bouillon et cuire 25 minutes.",
            "Mixer plus ou moins finement, ajouter la crème et servir avec le pain.",
        ],
    },
    Recette {
        id: "ratatouille",
        nom: "Ratatouille et semoule",
        categorie: Categorie::Mijote,
        vege: true,
        minutes: 45,
        ingredients: &[
            ("courgette", 120.),
            ("aubergine", 120.),
            ("poivron", 80.),
            ("tomates_concassees", 100.),
            ("oignon", 40.),
            ("ail", 0.5),
            ("huile_olive", 15.),
            ("semoule", 70.),
        ],
        etapes: &[
            "Faire revenir l'oignon et l'ail dans l'huile d'olive, puis les légumes coupés en dés.",
            "Ajouter les tomates, couvrir et laisser mijoter 30 minutes.",
            "Verser sur la semoule son volume d'eau bouillante salée, couvrir 5 minutes et égrener.",
        ],
    },
    Recette {
        id: "couscous_poulet",
        nom: "Couscous au poulet et aux légumes",
        categorie: Categorie::Mijote,
        vege: false,
        minutes: 50,
        ingredients: &[
            ("cuisse_poulet", 180.),
            ("semoule", 80.),
            ("carotte", 80.),
            ("courgette", 80.),
            ("pois_chiches", 50.),
            ("tomates_concassees", 60.),
            ("bouillon", 0.5),
            ("cumin", 2.),
        ],
        etapes: &[
            "Faire dorer le poulet avec le cumin.",
            "Ajouter les carottes, les tomates et couvrir de bouillon ; cuire 20 minutes.",
            "Ajouter les courgettes et les pois chiches, cuire encore 15 minutes.",
            "Préparer la semoule et servir avec les légumes et le bouillon.",
        ],
    },
    Recette {
        id: "shakshuka",
        nom: "Shakshuka",
        categorie: Categorie::Express,
        vege: true,
        minutes: 25,
        ingredients: &[
            ("oeuf", 2.),
            ("tomates_concassees", 200.),
            ("poivron", 80.),
            ("oignon", 40.),
            ("ail", 0.5),
            ("cumin", 2.),
            ("pain", 0.33),
        ],
        etapes: &[
            "Faire revenir l'oignon, le poivron et l'ail avec le cumin.",
            "Ajouter les tomates et laisser réduire 10 minutes.",
            "Creuser des puits, y casser les œufs, couvrir et cuire 5 minutes.",
            "Servir dans la poêle avec le pain.",
        ],
    },
    Recette {
        id: "croque_monsieur",
        nom: "Croque-monsieur et salade",
        categorie: Categorie::Express,
        vege: false,
        minutes: 15,
        ingredients: &[
            ("pain_mie", 4.),
            ("jambon", 2.),
            ("emmental", 50.),
            ("beurre", 10.),
            ("salade", 0.25),
        ],
        etapes: &[
            "Beurrer les tranches de pain de mie, garnir de jambon et de fromage.",
            "Refermer, parsemer de fromage et dorer 8 minutes au four à 210 °C (ou à la poêle).",
            "Servir avec la salade.",
        ],
    },
    Recette {
        id: "pizza_jambon",
        nom: "Pizza maison jambon-champignons",
        categorie: Categorie::Four,
        vege: false,
        minutes: 25,
        ingredients: &[
            ("pate_pizza", 0.5),
            ("coulis_tomate", 60.),
            ("mozzarella", 60.),
            ("jambon", 1.),
            ("champignons", 50.),
        ],
        etapes: &[
            "Étaler la pâte et la napper de coulis de tomate.",
            "Répartir le jambon, les champignons émincés et la mozzarella.",
            "Cuire 12 à 15 minutes dans un four très chaud (240 °C).",
        ],
    },
    Recette {
        id: "tarte_tomate",
        nom: "Tarte à la tomate, moutarde et chèvre",
        categorie: Categorie::Four,
        vege: true,
        minutes: 35,
        ingredients: &[
            ("pate_feuilletee", 0.25),
            ("tomate", 150.),
            ("moutarde", 8.),
            ("chevre", 40.),
            ("salade", 0.25),
        ],
        etapes: &[
            "Étaler la pâte, la piquer et la badigeonner de moutarde.",
            "Ranger les tomates en rondelles, poser le chèvre par-dessus.",
            "Cuire 25 minutes à 200 °C et servir avec la salade.",
        ],
    },
    Recette {
        id: "saumon_brocoli",
        nom: "Saumon au four, riz et brocoli",
        categorie: Categorie::Poisson,
        vege: false,
        minutes: 25,
        ingredients: &[
            ("saumon", 130.),
            ("riz", 70.),
            ("brocoli", 150.),
            ("citron", 0.25),
        ],
        etapes: &[
            "Cuire le saumon 12 minutes à 200 °C avec un filet de citron.",
            "Cuire le riz, et le brocoli en fleurettes 6 minutes à l'eau ou à la vapeur.",
            "Servir avec un quartier de citron.",
        ],
    },
    Recette {
        id: "colin_citron",
        nom: "Colin à la crème citronnée et pommes vapeur",
        categorie: Categorie::Poisson,
        vege: false,
        minutes: 30,
        ingredients: &[
            ("colin", 130.),
            ("pomme_de_terre", 250.),
            ("creme", 40.),
            ("citron", 0.25),
        ],
        etapes: &[
            "Cuire les pommes de terre à la vapeur ou à l'eau, 20 minutes.",
            "Cuire le colin à la poêle, 3 à 4 minutes par face.",
            "Déglacer avec le jus de citron, ajouter la crème et napper le poisson.",
        ],
    },
    Recette {
        id: "salade_riz_thon",
        nom: "Salade de riz au thon",
        categorie: Categorie::Riz,
        vege: false,
        minutes: 20,
        ingredients: &[
            ("riz", 70.),
            ("thon", 60.),
            ("mais", 50.),
            ("tomate", 100.),
            ("oeuf", 1.),
            ("huile_olive", 10.),
        ],
        etapes: &[
            "Cuire le riz et les œufs durs (10 minutes), les refroidir sous l'eau.",
            "Mélanger le riz, le thon émietté, le maïs et les tomates en dés.",
            "Assaisonner à l'huile d'olive et ajouter les œufs en quartiers.",
        ],
    },
    Recette {
        id: "salade_lentilles_feta",
        nom: "Salade de lentilles à la feta",
        categorie: Categorie::Express,
        vege: true,
        minutes: 30,
        ingredients: &[
            ("lentilles_vertes", 70.),
            ("feta", 50.),
            ("tomate", 100.),
            ("concombre", 0.25),
            ("huile_olive", 10.),
            ("citron", 0.25),
        ],
        etapes: &[
            "Cuire les lentilles 20 minutes à l'eau non salée, les égoutter et les laisser tiédir.",
            "Ajouter les tomates et le concombre en dés, puis la feta émiettée.",
            "Assaisonner d'huile d'olive et de jus de citron.",
        ],
    },
    Recette {
        id: "fajitas_poulet",
        nom: "Fajitas au poulet",
        categorie: Categorie::Express,
        vege: false,
        minutes: 25,
        ingredients: &[
            ("tortilla", 2.),
            ("poulet", 110.),
            ("poivron", 80.),
            ("oignon", 40.),
            ("creme", 30.),
            ("emmental", 20.),
            ("paprika", 2.),
        ],
        etapes: &[
            "Faire sauter le poulet en lanières avec le paprika.",
            "Ajouter les poivrons et l'oignon émincés, cuire 8 minutes.",
            "Réchauffer les tortillas et les garnir avec la poêlée, la crème et le fromage.",
        ],
    },
    Recette {
        id: "gnocchis_epinards",
        nom: "Gnocchis aux épinards et à la crème",
        categorie: Categorie::Express,
        vege: true,
        minutes: 15,
        ingredients: &[
            ("gnocchi", 180.),
            ("epinards", 100.),
            ("creme", 50.),
            ("parmesan", 15.),
            ("ail", 0.5),
        ],
        etapes: &[
            "Faire dorer les gnocchis à la poêle, 5 minutes.",
            "Ajouter l'ail et les épinards, les laisser tomber.",
            "Verser la crème, laisser épaissir 2 minutes et servir avec le parmesan.",
        ],
    },
    Recette {
        id: "nouilles_tofu",
        nom: "Nouilles sautées au tofu et aux légumes",
        categorie: Categorie::Express,
        vege: true,
        minutes: 20,
        ingredients: &[
            ("nouilles", 80.),
            ("tofu", 80.),
            ("carotte", 60.),
            ("brocoli", 80.),
            ("sauce_soja", 15.),
            ("huile_olive", 8.),
            ("ail", 0.5),
        ],
        etapes: &[
            "Cuire les nouilles selon le paquet.",
            "Faire dorer le tofu en dés, le réserver. Faire sauter les légumes coupés fin avec l'ail.",
            "Ajouter les nouilles, le tofu et la sauce soja, faire sauter 2 minutes à feu vif.",
        ],
    },
    Recette {
        id: "dinde_champignons",
        nom: "Escalope de dinde à la crème et aux champignons",
        categorie: Categorie::Riz,
        vege: false,
        minutes: 25,
        ingredients: &[
            ("dinde", 120.),
            ("champignons", 100.),
            ("creme", 50.),
            ("riz", 70.),
            ("moutarde", 5.),
        ],
        etapes: &[
            "Dorer les escalopes 4 minutes par face, les réserver.",
            "Faire sauter les champignons, ajouter la crème et la moutarde.",
            "Remettre la dinde 2 minutes dans la sauce et servir avec le riz.",
        ],
    },
    Recette {
        id: "gratin_chou_fleur",
        nom: "Gratin de chou-fleur",
        categorie: Categorie::Four,
        vege: true,
        minutes: 40,
        ingredients: &[
            ("chou_fleur", 250.),
            ("lait", 100.),
            ("farine", 10.),
            ("beurre", 10.),
            ("emmental", 35.),
        ],
        etapes: &[
            "Cuire le chou-fleur en fleurettes 10 minutes à l'eau bouillante.",
            "Préparer une béchamel : beurre fondu, farine, puis lait versé peu à peu en fouettant.",
            "Napper le chou-fleur, couvrir de fromage et gratiner 20 minutes à 200 °C.",
        ],
    },
    Recette {
        id: "riz_chorizo",
        nom: "Riz au chorizo et aux poivrons",
        categorie: Categorie::Riz,
        vege: false,
        minutes: 30,
        ingredients: &[
            ("riz", 80.),
            ("chorizo", 50.),
            ("poivron", 80.),
            ("tomates_concassees", 60.),
            ("oignon", 40.),
            ("bouillon", 0.5),
        ],
        etapes: &[
            "Faire revenir le chorizo en rondelles, puis l'oignon et le poivron dans son gras.",
            "Ajouter le riz, les tomates et deux fois le volume du riz en bouillon.",
            "Cuire à couvert 18 minutes sans remuer.",
        ],
    },
    Recette {
        id: "patates_douces_pois_chiches",
        nom: "Patates douces rôties aux pois chiches",
        categorie: Categorie::Four,
        vege: true,
        minutes: 40,
        ingredients: &[
            ("patate_douce", 250.),
            ("pois_chiches", 100.),
            ("yaourt", 1.),
            ("paprika", 2.),
            ("huile_olive", 10.),
            ("citron", 0.25),
        ],
        etapes: &[
            "Mélanger les patates douces en cubes et les pois chiches égouttés avec l'huile et le paprika.",
            "Rôtir 30 minutes à 200 °C en remuant à mi-cuisson.",
            "Servir avec le yaourt relevé de jus de citron.",
        ],
    },
    Recette {
        id: "omelette_champignons",
        nom: "Omelette aux champignons et salade",
        categorie: Categorie::Express,
        vege: true,
        minutes: 15,
        ingredients: &[
            ("oeuf", 3.),
            ("champignons", 80.),
            ("emmental", 25.),
            ("beurre", 8.),
            ("salade", 0.25),
            ("pain", 0.25),
        ],
        etapes: &[
            "Faire sauter les champignons émincés dans le beurre.",
            "Verser les œufs battus et salés, parsemer de fromage.",
            "Cuire à feu moyen en ramenant les bords, plier et servir avec la salade et le pain.",
        ],
    },
    // Deuxième lot (voir `LOT_2`) : ajouté aux bases existantes par migration.
    Recette {
        id: "soupe_lentilles_corail",
        nom: "Soupe de lentilles corail aux carottes",
        categorie: Categorie::Soupe,
        vege: true,
        minutes: 30,
        ingredients: &[
            ("lentilles_corail", 70.),
            ("carotte", 100.),
            ("oignon", 40.),
            ("lait_coco", 40.),
            ("bouillon", 0.5),
            ("cumin", 2.),
            ("pain", 0.25),
        ],
        etapes: &[
            "Faire revenir l'oignon émincé avec le cumin, puis ajouter les carottes en rondelles.",
            "Ajouter les lentilles rincées, couvrir largement de bouillon et cuire 20 minutes.",
            "Mixer avec le lait de coco, rectifier l'assaisonnement et servir avec le pain.",
        ],
    },
    Recette {
        id: "gratin_courgettes",
        nom: "Gratin de courgettes",
        categorie: Categorie::Four,
        vege: true,
        minutes: 40,
        ingredients: &[
            ("courgette", 250.),
            ("creme", 50.),
            ("oeuf", 0.5),
            ("emmental", 30.),
        ],
        etapes: &[
            "Couper les courgettes en rondelles et les faire revenir 10 minutes à la poêle pour leur faire rendre leur eau.",
            "Battre les œufs avec la crème, saler et poivrer.",
            "Ranger les courgettes dans un plat, verser l'appareil, couvrir de fromage et cuire 25 minutes à 190 °C.",
        ],
    },
    Recette {
        id: "pates_brocoli_lardons",
        nom: "Pâtes au brocoli et aux lardons",
        categorie: Categorie::Pates,
        vege: false,
        minutes: 20,
        ingredients: &[
            ("pates", 100.),
            ("brocoli", 150.),
            ("lardons", 40.),
            ("creme", 30.),
            ("parmesan", 10.),
        ],
        etapes: &[
            "Cuire les pâtes ; ajouter le brocoli en petites fleurettes dans la casserole 4 minutes avant la fin.",
            "Faire dorer les lardons à la poêle, puis ajouter la crème.",
            "Égoutter les pâtes et le brocoli, les mêler aux lardons et servir avec le parmesan.",
        ],
    },
    Recette {
        id: "pommes_de_terre_lardons",
        nom: "Pommes de terre gratinées aux lardons",
        categorie: Categorie::Four,
        vege: false,
        minutes: 45,
        ingredients: &[
            ("pomme_de_terre", 300.),
            ("lardons", 60.),
            ("oignon", 50.),
            ("creme", 30.),
            ("emmental", 30.),
        ],
        etapes: &[
            "Cuire les pommes de terre à l'eau 15 minutes, puis les couper en rondelles épaisses.",
            "Faire revenir les lardons et l'oignon émincé.",
            "Mélanger le tout dans un plat avec la crème, couvrir de fromage et gratiner 20 minutes à 200 °C.",
        ],
    },
    Recette {
        id: "gnocchis_tomate_mozzarella",
        nom: "Gnocchis gratinés tomate-mozzarella",
        categorie: Categorie::Four,
        vege: true,
        minutes: 20,
        ingredients: &[
            ("gnocchi", 180.),
            ("coulis_tomate", 100.),
            ("mozzarella", 60.),
            ("herbes", 1.),
        ],
        etapes: &[
            "Faire dorer les gnocchis à la poêle 5 minutes.",
            "Les mélanger au coulis de tomate et aux herbes dans un plat.",
            "Répartir la mozzarella en morceaux et gratiner 10 minutes à 210 °C.",
        ],
    },
    Recette {
        id: "pois_chiches_chorizo",
        nom: "Pois chiches au chorizo et semoule",
        categorie: Categorie::Mijote,
        vege: false,
        minutes: 25,
        ingredients: &[
            ("pois_chiches", 130.),
            ("chorizo", 40.),
            ("tomates_concassees", 120.),
            ("oignon", 40.),
            ("paprika", 2.),
            ("semoule", 70.),
        ],
        etapes: &[
            "Faire revenir le chorizo en rondelles, puis l'oignon émincé dans son gras.",
            "Ajouter les pois chiches égouttés, les tomates et le paprika, laisser mijoter 15 minutes.",
            "Verser sur la semoule son volume d'eau bouillante salée, couvrir 5 minutes, égrener et servir.",
        ],
    },
    Recette {
        id: "curry_chou_fleur",
        nom: "Curry de chou-fleur et patate douce",
        categorie: Categorie::Mijote,
        vege: true,
        minutes: 35,
        ingredients: &[
            ("chou_fleur", 150.),
            ("patate_douce", 120.),
            ("lait_coco", 80.),
            ("tomates_concassees", 60.),
            ("oignon", 40.),
            ("curry", 3.),
            ("riz", 70.),
        ],
        etapes: &[
            "Faire revenir l'oignon émincé avec le curry.",
            "Ajouter la patate douce en cubes et le chou-fleur en fleurettes, puis les tomates et le lait de coco.",
            "Couvrir et laisser mijoter 20 minutes, jusqu'à ce que les légumes soient tendres.",
            "Servir avec le riz.",
        ],
    },
    Recette {
        id: "taboule_pois_chiches",
        nom: "Taboulé aux pois chiches et à la feta",
        categorie: Categorie::Express,
        vege: true,
        minutes: 15,
        ingredients: &[
            ("semoule", 70.),
            ("pois_chiches", 60.),
            ("tomate", 100.),
            ("concombre", 0.25),
            ("feta", 40.),
            ("citron", 0.5),
            ("huile_olive", 10.),
        ],
        etapes: &[
            "Verser sur la semoule son volume d'eau bouillante salée, couvrir 5 minutes puis égrener.",
            "Ajouter les tomates et le concombre en petits dés, les pois chiches égouttés et la feta émiettée.",
            "Assaisonner de jus de citron et d'huile d'olive ; servir frais.",
        ],
    },
    Recette {
        id: "poulet_moutarde",
        nom: "Poulet à la moutarde et riz",
        categorie: Categorie::Riz,
        vege: false,
        minutes: 30,
        ingredients: &[
            ("poulet", 120.),
            ("moutarde", 10.),
            ("creme", 50.),
            ("oignon", 30.),
            ("riz", 75.),
        ],
        etapes: &[
            "Faire dorer le poulet en morceaux, le réserver.",
            "Faire fondre l'oignon émincé dans la même poêle, ajouter la moutarde et la crème.",
            "Remettre le poulet et laisser mijoter 10 minutes ; servir avec le riz.",
        ],
    },
    Recette {
        id: "wraps_thon",
        nom: "Wraps au thon et crudités",
        categorie: Categorie::Express,
        vege: false,
        minutes: 10,
        ingredients: &[
            ("tortilla", 2.),
            ("thon", 60.),
            ("salade", 0.25),
            ("tomate", 80.),
            ("mais", 30.),
            ("yaourt", 0.5),
        ],
        etapes: &[
            "Mélanger le thon émietté avec le yaourt, saler et poivrer.",
            "Garnir les tortillas de salade, de tomates en dés, de maïs et de thon.",
            "Rouler serré et couper en deux.",
        ],
    },
    Recette {
        id: "aubergines_parmigiana",
        nom: "Aubergines gratinées à la parmigiana",
        categorie: Categorie::Four,
        vege: true,
        minutes: 45,
        ingredients: &[
            ("aubergine", 250.),
            ("tomates_concassees", 150.),
            ("mozzarella", 60.),
            ("parmesan", 15.),
            ("ail", 0.5),
            ("huile_olive", 10.),
            ("pain", 0.25),
        ],
        etapes: &[
            "Couper les aubergines en tranches, les huiler et les dorer 15 minutes au four à 200 °C.",
            "Faire réduire les tomates avec l'ail une dizaine de minutes.",
            "Alterner dans un plat les aubergines, la sauce et la mozzarella ; finir par le parmesan.",
            "Gratiner 20 minutes à 190 °C et servir avec le pain.",
        ],
    },
    Recette {
        id: "colin_papillote",
        nom: "Colin en papillote aux légumes",
        categorie: Categorie::Poisson,
        vege: false,
        minutes: 30,
        ingredients: &[
            ("colin", 130.),
            ("courgette", 100.),
            ("carotte", 60.),
            ("citron", 0.25),
            ("huile_olive", 8.),
            ("semoule", 70.),
        ],
        etapes: &[
            "Tailler les courgettes et les carottes en fins bâtonnets.",
            "Les répartir sur des feuilles de papier cuisson, poser le colin, arroser d'huile et de jus de citron, puis refermer.",
            "Cuire 18 minutes à 200 °C.",
            "Préparer la semoule et servir avec les papillotes.",
        ],
    },
];

/// Recettes ajoutées au catalogue de départ après la première version. Une
/// base créée avant ne les a pas reçues : une migration les lui ajoute.
pub static LOT_2: &[&str] = &[
    "soupe_lentilles_corail",
    "gratin_courgettes",
    "pates_brocoli_lardons",
    "pommes_de_terre_lardons",
    "gnocchis_tomate_mozzarella",
    "pois_chiches_chorizo",
    "curry_chou_fleur",
    "taboule_pois_chiches",
    "poulet_moutarde",
    "wraps_thon",
    "aubergines_parmigiana",
    "colin_papillote",
];

/// Position d'un ingrédient dans [`INGREDIENTS`].
pub fn index_ingredient(id: &str) -> Option<usize> {
    INGREDIENTS.iter().position(|i| i.id == id)
}

/// Position d'une recette dans [`RECETTES`].
#[cfg(test)]
pub fn index_recette(id: &str) -> Option<usize> {
    RECETTES.iter().position(|r| r.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn identifiants_uniques() {
        let ingredients: HashSet<_> = INGREDIENTS.iter().map(|i| i.id).collect();
        assert_eq!(ingredients.len(), INGREDIENTS.len());
        let recettes: HashSet<_> = RECETTES.iter().map(|r| r.id).collect();
        assert_eq!(recettes.len(), RECETTES.len());
    }

    #[test]
    fn recettes_coherentes() {
        for r in RECETTES {
            assert!(!r.etapes.is_empty(), "{} sans étapes", r.id);
            assert!(!r.ingredients.is_empty(), "{} sans ingrédients", r.id);
            for (id, quantite) in r.ingredients {
                let i = index_ingredient(id)
                    .unwrap_or_else(|| panic!("{} : ingrédient inconnu « {id} »", r.id));
                assert!(*quantite > 0., "{} : quantité nulle pour {id}", r.id);
                if r.vege {
                    assert_ne!(
                        INGREDIENTS[i].rayon,
                        Rayon::ViandesPoissons,
                        "{} est végétarienne mais contient {id}",
                        r.id
                    );
                    assert_ne!(
                        *id, "thon",
                        "{} est végétarienne mais contient du thon",
                        r.id
                    );
                }
            }
        }
    }

    #[test]
    fn lot_2_dans_le_catalogue() {
        for id in LOT_2 {
            assert!(index_recette(id).is_some(), "{id} absente du catalogue");
        }
        // Le lot est la fin du catalogue : l'ordre d'une base neuve et celui
        // d'une base migrée sont les mêmes.
        let debut = RECETTES.len() - LOT_2.len();
        let fin: Vec<&str> = RECETTES[debut..].iter().map(|r| r.id).collect();
        assert_eq!(fin, LOT_2);
    }

    #[test]
    fn assez_de_recettes_vegetariennes_pour_deux_semaines_de_repas() {
        assert!(RECETTES.iter().filter(|r| r.vege).count() >= 14);
    }
}
