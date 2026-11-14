//! Nutrition : valeurs moyennes des ingrédients du catalogue de départ, apports
//! d'une recette par personne et Nutri-Score estimé.
//!
//! Les valeurs sont des moyennes arrondies (ordre de grandeur des tables
//! Ciqual), pour 100 g ou 100 ml d'ingrédient cru. Elles sont retrouvées par le
//! **nom** de l'ingrédient : un ingrédient créé dans l'application n'en a pas, et
//! les recettes qui l'utilisent sont signalées comme incomplètes.
//!
//! Le score suit l'algorithme Nutri-Score de 2023 pour les aliments généraux,
//! appliqué à 100 g de plat cuisiné. C'est une estimation : le sel ajouté « à
//! vue », l'eau de cuisson et les pertes à la cuisson ne sont pas mesurés.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::catalogue::Catalogue;
use crate::donnees::{self, Categorie, Unite};

/// Apports, en kilocalories et en grammes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Apports {
    pub kcal: f64,
    pub glucides: f64,
    pub sucres: f64,
    pub lipides: f64,
    pub satures: f64,
    pub proteines: f64,
    pub fibres: f64,
    pub sel: f64,
}

impl Apports {
    fn ajouter(&mut self, autre: &Apports, facteur: f64) {
        self.kcal += autre.kcal * facteur;
        self.glucides += autre.glucides * facteur;
        self.sucres += autre.sucres * facteur;
        self.lipides += autre.lipides * facteur;
        self.satures += autre.satures * facteur;
        self.proteines += autre.proteines * facteur;
        self.fibres += autre.fibres * facteur;
        self.sel += autre.sel * facteur;
    }
}

/// Valeurs de référence d'un ingrédient.
#[derive(Clone, Copy)]
struct Reference {
    /// Pour 100 g ou 100 ml d'ingrédient cru.
    pour_100: Apports,
    /// Compte parmi les fruits, légumes et légumineuses du Nutri-Score.
    vegetal: bool,
    /// Poids d'une pièce, en grammes, pour les ingrédients vendus à la pièce.
    piece: f64,
    /// Poids dans l'assiette pour 1 g cru : les pâtes et le riz gonflent,
    /// les moules perdent leur coquille.
    rendement: f64,
}

impl Reference {
    const fn vegetal(mut self) -> Self {
        self.vegetal = true;
        self
    }

    const fn piece(mut self, grammes: f64) -> Self {
        self.piece = grammes;
        self
    }

    const fn rendement(mut self, rendement: f64) -> Self {
        self.rendement = rendement;
        self
    }
}

/// kcal, glucides, dont sucres, lipides, dont saturés, protéines, fibres, sel.
#[allow(clippy::too_many_arguments)]
const fn v(
    kcal: f64,
    glucides: f64,
    sucres: f64,
    lipides: f64,
    satures: f64,
    proteines: f64,
    fibres: f64,
    sel: f64,
) -> Reference {
    Reference {
        pour_100: Apports {
            kcal,
            glucides,
            sucres,
            lipides,
            satures,
            proteines,
            fibres,
            sel,
        },
        vegetal: false,
        piece: 100.,
        rendement: 1.,
    }
}

/// Épices sèches : utilisées par pincées, une valeur commune suffit.
const EPICE: Reference = v(320., 30., 3., 13., 2., 13., 30., 0.1);
/// Pâtes sèches et assimilés.
const PATES: Reference = v(355., 71., 3., 1.7, 0.3, 12.5, 3., 0.01).rendement(2.3);

/// (identifiant du catalogue de départ, valeurs).
static REFERENCES: &[(&str, Reference)] = &[
    // Fruits et légumes
    (
        "oignon",
        v(40., 7.4, 5.6, 0.2, 0.03, 1.1, 1.7, 0.01).vegetal(),
    ),
    (
        "ail",
        v(131., 21.5, 1.4, 0.3, 0.07, 6., 4.7, 0.02)
            .vegetal()
            .piece(5.),
    ),
    (
        "carotte",
        v(36., 6.6, 5., 0.2, 0.04, 0.8, 2.7, 0.15).vegetal(),
    ),
    // Les tubercules ne comptent pas parmi les légumes du Nutri-Score.
    ("pomme_de_terre", v(80., 16.7, 0.8, 0.1, 0.03, 2., 2., 0.01)),
    (
        "tomate",
        v(19., 2.5, 2.5, 0.3, 0.05, 0.9, 1.2, 0.01).vegetal(),
    ),
    (
        "courgette",
        v(16., 1.8, 1.7, 0.3, 0.06, 1.3, 1.1, 0.01).vegetal(),
    ),
    (
        "aubergine",
        v(21., 2.6, 2.4, 0.2, 0.04, 1., 2.8, 0.01).vegetal(),
    ),
    (
        "poivron",
        v(27., 4.6, 4.2, 0.3, 0.05, 0.9, 1.9, 0.01).vegetal(),
    ),
    (
        "champignons",
        v(22., 1.9, 1.4, 0.4, 0.06, 2.4, 1.4, 0.02).vegetal(),
    ),
    (
        "epinards",
        v(23., 0.8, 0.4, 0.4, 0.07, 2.8, 2.7, 0.18).vegetal(),
    ),
    (
        "poireau",
        v(29., 3.9, 2.8, 0.3, 0.04, 1.5, 2.6, 0.03).vegetal(),
    ),
    (
        "brocoli",
        v(30., 2., 1.5, 0.6, 0.07, 3., 2.8, 0.05).vegetal(),
    ),
    (
        "chou_fleur",
        v(25., 2.4, 2., 0.3, 0.06, 2., 2.2, 0.04).vegetal(),
    ),
    (
        "potiron",
        v(30., 5.5, 2.5, 0.1, 0.03, 1., 1.5, 0.01).vegetal(),
    ),
    ("patate_douce", v(86., 17.7, 5.5, 0.1, 0.03, 1.6, 3., 0.1)),
    (
        "salade",
        v(14., 1.4, 1.2, 0.2, 0.03, 1.3, 1.5, 0.02)
            .vegetal()
            .piece(250.),
    ),
    (
        "citron",
        v(22., 2.5, 2.2, 0.3, 0.04, 0.5, 1., 0.)
            .vegetal()
            .piece(60.),
    ),
    (
        "concombre",
        v(13., 2., 1.7, 0.1, 0.03, 0.6, 0.6, 0.)
            .vegetal()
            .piece(300.),
    ),
    (
        "chou_blanc",
        v(28., 4., 3.5, 0.2, 0.03, 1.3, 2.4, 0.03).vegetal(),
    ),
    (
        "betterave",
        v(44., 7.9, 7., 0.1, 0.02, 1.7, 2., 0.15).vegetal(),
    ),
    ("navet", v(22., 3.1, 3., 0.2, 0.02, 0.9, 2., 0.1).vegetal()),
    (
        "celeri",
        v(16., 1.5, 1.3, 0.2, 0.04, 0.7, 1.6, 0.2).vegetal(),
    ),
    (
        "gingembre",
        v(80., 15.8, 1.7, 0.8, 0.2, 1.8, 2., 0.03).vegetal(),
    ),
    (
        "piment",
        v(40., 7., 5., 0.4, 0.04, 1.9, 1.5, 0.02)
            .vegetal()
            .piece(10.),
    ),
    (
        "citron_vert",
        v(22., 3., 1.7, 0.2, 0.02, 0.7, 1., 0.).vegetal().piece(50.),
    ),
    (
        "coriandre",
        v(23., 0.9, 0.9, 0.5, 0.01, 2.1, 2.8, 0.1)
            .vegetal()
            .piece(30.),
    ),
    (
        "persil",
        v(36., 3., 0.9, 0.8, 0.13, 3., 3.3, 0.14)
            .vegetal()
            .piece(30.),
    ),
    (
        "aneth",
        v(43., 4.9, 0., 1.1, 0.06, 3.5, 2.1, 0.15)
            .vegetal()
            .piece(20.),
    ),
    (
        "menthe",
        v(50., 5.3, 0., 0.9, 0.2, 3.8, 6.8, 0.08)
            .vegetal()
            .piece(20.),
    ),
    (
        "sauge",
        v(60., 5., 0., 1.5, 0.5, 2., 5., 0.01).vegetal().piece(15.),
    ),
    (
        "citronnelle",
        v(99., 25., 0., 0.5, 0.1, 1.8, 0., 0.02)
            .vegetal()
            .piece(15.),
    ),
    ("plantain", v(122., 29.6, 15., 0.4, 0.14, 1.3, 2.3, 0.01)),
    (
        "epi_mais",
        v(86., 16.3, 3.2, 1.2, 0.2, 3.2, 2.7, 0.04).piece(150.),
    ),
    (
        "avocat",
        v(160., 1.8, 0.7, 14.7, 2.1, 2., 6.7, 0.02)
            .vegetal()
            .piece(150.),
    ),
    (
        "germes_soja",
        v(30., 4., 2., 0.2, 0.05, 3., 1.8, 0.01).vegetal(),
    ),
    // Viandes et poissons
    ("poulet", v(110., 0., 0., 1.5, 0.4, 23.5, 0., 0.15)),
    // Avec os : valeurs ramenées au poids acheté.
    (
        "cuisse_poulet",
        v(140., 0., 0., 8.5, 2.3, 15., 0., 0.15).rendement(0.75),
    ),
    ("dinde", v(105., 0., 0., 1.3, 0.4, 23.5, 0., 0.15)),
    ("boeuf_hache", v(220., 0., 0., 15., 6.5, 19.5, 0., 0.18)),
    ("lardons", v(280., 0.6, 0.5, 24., 8.8, 15.5, 0., 2.6)),
    (
        "jambon",
        v(115., 0.8, 0.8, 3.5, 1.2, 20., 0., 1.9).piece(40.),
    ),
    ("saucisse", v(290., 1., 0.5, 25., 9.5, 15., 0., 1.8)),
    ("chorizo", v(455., 2., 1.5, 38., 14., 24., 0., 3.8)),
    ("saumon", v(200., 0., 0., 13., 2.5, 20., 0., 0.1)),
    ("colin", v(80., 0., 0., 1., 0.2, 17.5, 0., 0.25)),
    ("boeuf_braiser", v(180., 0., 0., 10., 4., 21., 0., 0.15)),
    ("rumsteck", v(125., 0., 0., 3.5, 1.4, 22.5, 0., 0.15)),
    ("entrecote", v(200., 0., 0., 13., 5.5, 20.5, 0., 0.15)),
    ("filet_boeuf", v(140., 0., 0., 5., 2.1, 22.5, 0., 0.14)),
    ("veau", v(150., 0., 0., 6.5, 2.6, 21., 0., 0.2)),
    (
        "jarret_veau",
        v(120., 0., 0., 4., 1.5, 20., 0., 0.2).rendement(0.8),
    ),
    ("escalope_veau", v(110., 0., 0., 2., 0.8, 22.5, 0., 0.2)),
    ("agneau", v(220., 0., 0., 16., 7.5, 18., 0., 0.17)),
    ("magret", v(290., 0., 0., 25., 7.5, 17., 0., 0.2)),
    (
        "confit_canard",
        v(290., 0., 0., 22., 6.5, 23., 0., 1.5).piece(200.),
    ),
    ("porc_hache", v(250., 0., 0., 20., 7.5, 17., 0., 0.15)),
    ("echine_porc", v(230., 0., 0., 17., 6.5, 18., 0., 0.15)),
    ("poitrine_porc", v(380., 0., 0., 35., 13., 14., 0., 0.15)),
    ("cote_porc", v(190., 0., 0., 12., 4.5, 20., 0., 0.15)),
    ("saucisse_fumee", v(320., 1., 0.5, 28., 10.5, 14., 0., 2.3)),
    (
        "jambon_cru",
        v(240., 0.5, 0.3, 13., 4.7, 28., 0., 5.3).piece(15.),
    ),
    ("crevettes", v(95., 0.5, 0., 1.2, 0.3, 20.5, 0., 0.9)),
    // Vendues en coquille : la chair en fait à peu près le tiers.
    (
        "moules",
        v(26., 1.1, 0., 0.7, 0.15, 3.6, 0., 0.2).rendement(0.3),
    ),
    ("morue", v(105., 0., 0., 0.8, 0.2, 24., 0., 2.)),
    ("cabillaud", v(80., 0., 0., 0.7, 0.15, 18., 0., 0.2)),
    ("thon_frais", v(140., 0., 0., 5., 1.3, 24., 0., 0.1)),
    // Frais et crèmerie
    (
        "oeuf",
        v(140., 0.7, 0.3, 9.8, 2.6, 12.5, 0., 0.35).piece(55.),
    ),
    ("lait", v(46., 4.8, 4.8, 1.5, 1., 3.2, 0., 0.1)),
    ("creme", v(290., 2.6, 2.6, 30., 20., 2.3, 0., 0.07)),
    ("beurre", v(745., 0.7, 0.7, 82., 55., 0.7, 0., 0.03)),
    ("emmental", v(370., 0., 0., 28.5, 18.5, 28., 0., 0.6)),
    ("parmesan", v(395., 0., 0., 28., 19., 35., 0., 1.6)),
    ("mozzarella", v(250., 1.5, 1., 19., 13., 18., 0., 0.5)),
    ("feta", v(265., 1., 1., 21.5, 14.5, 16., 0., 2.5)),
    ("chevre", v(290., 1., 1., 23.5, 16.5, 18., 0., 1.4)),
    (
        "yaourt",
        v(60., 5., 5., 2.8, 1.8, 3.8, 0., 0.13).piece(125.),
    ),
    (
        "pate_brisee",
        v(440., 45., 3., 26., 13., 6., 1.8, 0.9).piece(230.),
    ),
    (
        "pate_feuilletee",
        v(390., 37., 1.5, 24., 13., 5.5, 1.5, 0.9).piece(230.),
    ),
    (
        "pate_pizza",
        v(250., 45., 3., 4.5, 0.8, 7., 2., 1.3).piece(260.),
    ),
    ("gnocchi", v(160., 33., 0.5, 0.8, 0.3, 4., 1.8, 1.)),
    ("tofu", v(125., 1.5, 0.5, 7.5, 1.2, 13., 1., 0.02)),
    (
        "petits_pois",
        v(70., 8.5, 4.5, 0.6, 0.1, 5.5, 5.5, 0.01).vegetal(),
    ),
    (
        "choucroute",
        v(20., 1.5, 1.5, 0.2, 0.03, 1.2, 2.5, 1.).vegetal(),
    ),
    ("ricotta", v(150., 3., 3., 11., 7., 9.5, 0., 0.25)),
    ("cheddar", v(405., 0.5, 0.3, 33.5, 21., 25., 0., 1.8)),
    (
        "feuilles_brick",
        v(290., 60., 1.5, 1.5, 0.3, 8., 2., 1.5).piece(17.),
    ),
    // Épicerie
    ("pates", PATES),
    ("lasagnes", PATES),
    ("cannellonis", PATES),
    (
        "riz",
        v(350., 78., 0.2, 0.8, 0.2, 7., 1.2, 0.01).rendement(2.8),
    ),
    (
        "riz_arborio",
        v(350., 78., 0.2, 0.8, 0.2, 7., 1.2, 0.01).rendement(2.8),
    ),
    (
        "semoule",
        v(355., 72., 2., 1.5, 0.3, 12., 4., 0.01).rendement(2.5),
    ),
    (
        "nouilles",
        v(350., 70., 2., 2., 0.5, 12., 3., 0.5).rendement(2.4),
    ),
    (
        "lentilles_vertes",
        v(330., 48., 1.5, 1.5, 0.2, 25., 12., 0.02)
            .vegetal()
            .rendement(2.5),
    ),
    (
        "lentilles_corail",
        v(340., 50., 1.5, 1.8, 0.3, 25., 11., 0.02)
            .vegetal()
            .rendement(2.5),
    ),
    (
        "pois_chiches",
        v(130., 16., 0.5, 2.5, 0.3, 7., 6., 0.5).vegetal(),
    ),
    (
        "haricots_rouges",
        v(105., 13., 0.5, 0.6, 0.1, 8., 6.5, 0.5).vegetal(),
    ),
    (
        "haricots_blancs",
        v(100., 13., 0.5, 0.5, 0.1, 7., 6., 0.5).vegetal(),
    ),
    (
        "haricots_noirs",
        v(110., 14., 0.5, 0.5, 0.1, 7.5, 7., 0.5).vegetal(),
    ),
    ("mais", v(85., 15., 5., 1.5, 0.3, 2.9, 3., 0.5)),
    ("thon", v(115., 0., 0., 1., 0.3, 26.5, 0., 1.)),
    (
        "tomates_concassees",
        v(24., 3.5, 3.5, 0.2, 0.03, 1.2, 1.2, 0.15).vegetal(),
    ),
    (
        "coulis_tomate",
        v(32., 5., 4.5, 0.3, 0.05, 1.5, 1.5, 0.5).vegetal(),
    ),
    ("lait_coco", v(185., 2.8, 2.2, 18., 16., 1.7, 0., 0.03)),
    ("farine", v(350., 73., 1., 1.1, 0.2, 10.5, 3., 0.01)),
    ("huile_olive", v(825., 0., 0., 92., 13.5, 0., 0., 0.)),
    ("huile_sesame", v(825., 0., 0., 92., 14., 0., 0., 0.)),
    ("sauce_soja", v(60., 6., 2., 0., 0., 8., 0., 14.5)),
    ("moutarde", v(150., 3.5, 2.5, 11., 0.7, 7., 2., 5.5)),
    (
        "bouillon",
        v(240., 20., 8., 14., 8., 9., 1., 48.).piece(10.),
    ),
    (
        "tortilla",
        v(310., 52., 3., 7.5, 3.2, 8.5, 2.5, 1.3).piece(45.),
    ),
    ("curry", EPICE),
    ("cumin", EPICE),
    ("paprika", EPICE),
    ("herbes", EPICE),
    ("ras_el_hanout", EPICE),
    ("curcuma", EPICE),
    ("garam_masala", EPICE),
    ("cannelle", EPICE),
    ("cardamome", EPICE),
    ("sumac", EPICE),
    ("berbere", EPICE),
    ("piment_jamaique", EPICE),
    ("safran", EPICE.piece(0.1)),
    ("anis_etoile", EPICE.piece(1.)),
    ("beurre_cacahuete", v(620., 13., 7., 51., 9.5, 25., 7., 0.9)),
    ("cacahuetes", v(620., 9., 5., 50., 8.5, 26., 8., 0.8)),
    (
        "polenta",
        v(355., 77., 1., 1.5, 0.2, 8., 3., 0.01).rendement(4.5),
    ),
    (
        "vermicelles_riz",
        v(350., 80., 0.5, 0.6, 0.2, 6., 1.5, 0.05).rendement(2.7),
    ),
    (
        "nouilles_riz",
        v(350., 80., 0.5, 0.6, 0.2, 6., 1.5, 0.05).rendement(2.7),
    ),
    (
        "nouilles_patate_douce",
        v(340., 85., 0., 0.2, 0., 0.2, 0.5, 0.02).rendement(3.),
    ),
    ("pate_curry_rouge", v(130., 14., 7., 6., 1., 3.5, 6., 5.)),
    (
        "pate_curry_massaman",
        v(180., 15., 8., 11., 3., 3.5, 5., 5.5),
    ),
    ("vinaigre", v(10., 0.6, 0.6, 0., 0., 0.1, 0., 0.01)),
    ("vinaigre_riz", v(10., 0.5, 0.5, 0., 0., 0., 0., 0.01)),
    // Une bonne part de l'alcool s'évapore à la cuisson.
    ("vin_rouge", v(60., 0.3, 0.3, 0., 0., 0.1, 0., 0.)),
    ("vin_blanc", v(60., 1., 1., 0., 0., 0.1, 0., 0.)),
    ("biere", v(40., 3., 0.2, 0., 0., 0.4, 0., 0.01)),
    ("miel", v(325., 80., 80., 0., 0., 0.4, 0., 0.01)),
    ("sucre", v(400., 100., 100., 0., 0., 0., 0., 0.)),
    (
        "galettes_riz",
        v(340., 82., 1., 0.5, 0.1, 5.5, 1., 1.5).piece(9.),
    ),
    (
        "farine_mais",
        v(360., 78., 1., 1.8, 0.3, 7., 3.5, 0.01).rendement(2.2),
    ),
    (
        "ananas",
        v(65., 15., 14.5, 0.1, 0., 0.4, 1., 0.01).vegetal(),
    ),
    (
        "cornichons",
        v(15., 1.3, 1., 0.2, 0.05, 0.7, 1.2, 1.8).vegetal(),
    ),
    ("chapelure", v(370., 73., 5., 3.5, 0.8, 11., 4., 1.2)),
    ("levure", v(325., 14., 0., 7.5, 1., 40., 27., 0.1).piece(8.)),
    ("chocolat", v(570., 34., 28., 41., 24.5, 9., 11., 0.02)),
    ("noix", v(700., 6.5, 3., 67., 6.5, 15., 6., 0.01)),
    ("amandes", v(620., 7.5, 4.5, 53., 4.2, 21., 10., 0.01)),
    (
        "raisins_secs",
        v(310., 70., 66., 0.9, 0.2, 3., 5., 0.05).vegetal(),
    ),
    (
        "pruneaux",
        v(240., 55., 38., 0.4, 0.1, 2.2, 7., 0.01).vegetal(),
    ),
    ("melasse_grenade", v(320., 75., 70., 0., 0., 0.5, 0., 0.05)),
    ("nuoc_mam", v(45., 3.5, 3.5, 0., 0., 8., 0., 22.)),
    ("miso", v(200., 25., 6., 6., 1., 12., 5., 11.)),
    ("gochujang", v(220., 45., 25., 1.5, 0.3, 5., 3., 6.5)),
    // Boulangerie
    (
        "pain",
        v(275., 56., 2.5, 1.3, 0.3, 9., 3.5, 1.4).piece(250.),
    ),
    (
        "pain_mie",
        v(270., 49., 6., 4.5, 0.5, 8., 3., 1.2).piece(28.),
    ),
    ("pain_epices", v(330., 70., 40., 2.5, 0.5, 3.5, 2., 0.5)),
    (
        "pain_pita",
        v(265., 53., 2., 1.2, 0.2, 9., 2.5, 1.).piece(60.),
    ),
    (
        "pain_burger",
        v(280., 48., 6., 5., 1., 9., 2.5, 1.1).piece(50.),
    ),
];

/// Références retrouvées par le nom de l'ingrédient, en minuscules.
static PAR_NOM: LazyLock<HashMap<String, Reference>> = LazyLock::new(|| {
    REFERENCES
        .iter()
        .filter_map(|(id, reference)| {
            let i = donnees::index_ingredient(id)?;
            Some((donnees::INGREDIENTS[i].nom.to_lowercase(), *reference))
        })
        .collect()
});

/// Eau de cuisson d'une soupe, par personne : elle n'est pas dans les ingrédients.
const EAU_SOUPE: f64 = 250.;

/// Classe du Nutri-Score, de la meilleure à la moins bonne.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Score {
    A,
    B,
    C,
    D,
    E,
}

impl Score {
    pub const TOUS: [Score; 5] = [Score::A, Score::B, Score::C, Score::D, Score::E];

    pub fn lettre(self) -> &'static str {
        match self {
            Score::A => "A",
            Score::B => "B",
            Score::C => "C",
            Score::D => "D",
            Score::E => "E",
        }
    }
}

/// Nutrition d'une recette.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bilan {
    /// Apports d'une portion.
    pub par_personne: Apports,
    pub score: Score,
    /// Ingrédients sans valeurs connues : le bilan les ignore.
    pub manquants: usize,
}

/// Nombre de seuils dépassés : la façon dont le Nutri-Score attribue ses points.
fn points(valeur: f64, seuils: &[f64]) -> i32 {
    seuils.iter().filter(|seuil| valeur > **seuil).count() as i32
}

/// Seuils réguliers : `pas`, `2 × pas`… jusqu'à `nombre × pas`.
fn points_reguliers(valeur: f64, pas: f64, nombre: i32) -> i32 {
    if valeur <= 0. {
        0
    } else {
        // Un seuil n'est franchi que s'il est strictement dépassé.
        (((valeur - 1e-9) / pas).floor() as i32).clamp(0, nombre)
    }
}

/// Nutri-Score (algorithme 2023, aliments généraux) pour 100 g d'aliment.
/// `vegetaux` : part de fruits, légumes et légumineuses, en pour cent.
pub fn score(pour_100: &Apports, vegetaux: f64) -> Score {
    let energie = points_reguliers(pour_100.kcal * 4.184, 335., 10);
    let satures = points_reguliers(pour_100.satures, 1., 10);
    let sucres = points(
        pour_100.sucres,
        &[
            3.4, 6.8, 10., 14., 17., 20., 24., 27., 31., 34., 37., 41., 44., 48., 51.,
        ],
    );
    let sel = points_reguliers(pour_100.sel, 0.2, 20);
    let negatifs = energie + satures + sucres + sel;

    let proteines = points(pour_100.proteines, &[2.4, 4.8, 7.2, 9.6, 12., 14., 17.]);
    let fibres = points(pour_100.fibres, &[3., 4.1, 5.2, 6.3, 7.4]);
    let vegetaux = match vegetaux {
        v if v > 80. => 5,
        v if v > 60. => 2,
        v if v > 40. => 1,
        _ => 0,
    };
    // Au-delà de 11 points négatifs, les protéines ne rattrapent plus le score.
    let total = if negatifs >= 11 {
        negatifs - fibres - vegetaux
    } else {
        negatifs - proteines - fibres - vegetaux
    };
    match total {
        i32::MIN..=0 => Score::A,
        1..=2 => Score::B,
        3..=10 => Score::C,
        11..=18 => Score::D,
        _ => Score::E,
    }
}

/// Nutrition d'une recette, ou `None` si aucun de ses ingrédients n'est connu.
pub fn bilan(cat: &Catalogue, recette: usize) -> Option<Bilan> {
    let r = &cat.recettes[recette];
    let mut par_personne = Apports::default();
    // Poids dans l'assiette, et part de ce poids venant des végétaux.
    let (mut poids, mut vegetaux) = (0., 0.);
    let (mut connus, mut manquants) = (0, 0);
    for (id, quantite) in &r.ingredients {
        let Some(i) = cat.index_ingredient(*id) else {
            continue;
        };
        let i = &cat.ingredients[i];
        let Some(reference) = PAR_NOM.get(&i.nom.to_lowercase()) else {
            manquants += 1;
            continue;
        };
        connus += 1;
        let grammes = match i.unite {
            Unite::G | Unite::Ml => *quantite,
            Unite::Piece => quantite * reference.piece,
        };
        par_personne.ajouter(&reference.pour_100, grammes / 100.);
        let servi = grammes * reference.rendement;
        poids += servi;
        if reference.vegetal {
            vegetaux += servi;
        }
    }
    if connus == 0 || poids <= 0. {
        return None;
    }
    if r.categorie == Categorie::Soupe {
        poids += EAU_SOUPE;
    }
    let mut pour_100 = Apports::default();
    pour_100.ajouter(&par_personne, 100. / poids);
    Some(Bilan {
        par_personne,
        score: score(&pour_100, vegetaux / poids * 100.),
        manquants,
    })
}

/// Quantité en grammes, à la française : `12 g`, `2,4 g`.
pub fn grammes(quantite: f64) -> String {
    if quantite < 9.95 {
        format!("{quantite:.1} g").replace('.', ",")
    } else {
        format!("{quantite:.0} g")
    }
}

/// Énergie arrondie : `680 kcal`.
pub fn kcal(energie: f64) -> String {
    format!("{energie:.0} kcal")
}

/// Apports moyens par assiette d'un menu, sur les recettes dont la nutrition
/// est connue.
pub fn moyenne_menu(cat: &Catalogue, menu: &[usize]) -> Option<Apports> {
    let bilans: Vec<Bilan> = menu.iter().filter_map(|&r| bilan(cat, r)).collect();
    if bilans.is_empty() {
        return None;
    }
    let mut moyenne = Apports::default();
    for b in &bilans {
        moyenne.ajouter(&b.par_personne, 1. / bilans.len() as f64);
    }
    Some(moyenne)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tous_les_ingredients_de_depart_ont_leurs_valeurs() {
        for i in donnees::INGREDIENTS {
            assert!(
                REFERENCES.iter().any(|(id, _)| *id == i.id),
                "pas de valeurs nutritionnelles pour « {} »",
                i.id
            );
        }
        for (id, _) in REFERENCES {
            assert!(
                donnees::index_ingredient(id).is_some(),
                "« {id} » n'est pas un ingrédient du catalogue"
            );
        }
        assert_eq!(REFERENCES.len(), donnees::INGREDIENTS.len(), "doublon");
    }

    #[test]
    fn valeurs_plausibles() {
        for (id, r) in REFERENCES {
            let a = &r.pour_100;
            assert!(a.sucres <= a.glucides + 1e-9, "{id} : sucres > glucides");
            assert!(a.satures <= a.lipides + 1e-9, "{id} : saturés > lipides");
            assert!(
                a.glucides + a.lipides + a.proteines + a.fibres <= 101.,
                "{id} : plus de 100 g de nutriments pour 100 g"
            );
            // L'énergie doit suivre les macronutriments (4, 9, 4 et 2 kcal/g),
            // sauf pour les boissons alcoolisées.
            if !["vin_rouge", "vin_blanc", "biere"].contains(id) {
                let attendu = 4. * a.glucides + 9. * a.lipides + 4. * a.proteines + 2. * a.fibres;
                assert!(
                    (a.kcal - attendu).abs() <= 0.25 * attendu.max(40.),
                    "{id} : {} kcal annoncées, {attendu:.0} d'après les macronutriments",
                    a.kcal
                );
            }
        }
    }

    #[test]
    fn formats() {
        assert_eq!(grammes(2.44), "2,4 g");
        assert_eq!(grammes(0.), "0,0 g");
        assert_eq!(grammes(9.96), "10 g");
        assert_eq!(grammes(71.6), "72 g");
        assert_eq!(kcal(679.4), "679 kcal");
    }

    #[test]
    fn points_aux_seuils() {
        assert_eq!(points_reguliers(0., 1., 10), 0);
        assert_eq!(points_reguliers(1., 1., 10), 0);
        assert_eq!(points_reguliers(1.1, 1., 10), 1);
        assert_eq!(points_reguliers(50., 1., 10), 10);
        assert_eq!(points(2.4, &[2.4, 4.8]), 0);
        assert_eq!(points(5., &[2.4, 4.8]), 2);
    }

    #[test]
    fn scores_de_reference() {
        let aliment = |kcal, sucres, satures, proteines, fibres, sel| Apports {
            kcal,
            sucres,
            satures,
            proteines,
            fibres,
            sel,
            ..Apports::default()
        };
        // Légumes nature.
        assert_eq!(
            score(&aliment(30., 3., 0.1, 1.5, 2.5, 0.05), 100.),
            Score::A
        );
        // Plat de pâtes à la sauce tomate.
        assert_eq!(score(&aliment(130., 3., 0.5, 5., 2., 0.6), 30.), Score::B);
        // Gratin au fromage.
        assert_eq!(score(&aliment(150., 2., 3., 7., 1., 0.7), 10.), Score::C);
        // Pizza au fromage : trop de points négatifs pour compter ses protéines.
        assert_eq!(score(&aliment(250., 3., 4.5, 11., 2., 1.2), 10.), Score::D);
        // Charcuterie grasse et salée.
        assert_eq!(score(&aliment(455., 1.5, 14., 24., 0., 3.8), 0.), Score::E);
        // Pâte à tartiner.
        assert_eq!(score(&aliment(540., 56., 10.6, 6., 3.5, 0.1), 0.), Score::E);
    }

    #[test]
    fn bilan_des_recettes_de_depart() {
        let cat = Catalogue::en_memoire().unwrap();
        let mut classes = [0; 5];
        for r in 0..cat.recettes.len() {
            let b = bilan(&cat, r).expect("recette de départ sans nutrition");
            let nom = &cat.recettes[r].nom;
            assert_eq!(b.manquants, 0, "{nom}");
            assert!(
                (150. ..=1500.).contains(&b.par_personne.kcal),
                "{nom} : {:.0} kcal par personne",
                b.par_personne.kcal
            );
            classes[b.score as usize] += 1;
        }
        // Un plat cuisiné maison est rarement classé E, et tout n'est pas A.
        assert!(
            classes[0] > 0 && classes[1] > 0 && classes[2] > 0,
            "{classes:?}"
        );
        assert!(classes[4] * 10 < cat.recettes.len(), "{classes:?}");
    }

    #[test]
    fn un_ingredient_inconnu_est_signale() {
        let mut cat = Catalogue::en_memoire().unwrap();
        let inconnu = cat
            .ajouter_ingredient("Topinambours", donnees::Rayon::FruitsLegumes, Unite::G, 4.)
            .unwrap();
        let mut recette = cat.recettes[0].clone();
        recette.ingredients.push((inconnu, 100.));
        cat.enregistrer_recette(&recette).unwrap();
        let b = bilan(&cat, 0).unwrap();
        assert_eq!(b.manquants, 1);
    }
}
