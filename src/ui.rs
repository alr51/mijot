//! Fenêtre principale : état, actions, barre latérale. Chaque page a son
//! sous-module ; `style` porte la palette et les briques visuelles communes.

mod courses;
mod menu;
mod prix;
mod recettes;
mod style;
mod suggestion;

pub use style::appliquer_palette;

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _, Size, Theme, ThemeMode,
    alert::Alert,
    h_flex,
    input::{InputEvent, InputState},
    searchable_list::SearchableListItem,
    select::{SearchableVec, SelectState},
    tag::Tag,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::catalogue::Catalogue;
use crate::donnees::Unite;
use crate::etat::{Etat, Extra};
use crate::plan::{
    Echec, PERSONNES_MAX, REPAS_MAX, euros, generer, liste_de_courses, quantite_lisible,
    remplacantes,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Onglet {
    Menu,
    Courses,
    Recettes,
    Prix,
}

const ONGLETS: [Onglet; 4] = [
    Onglet::Menu,
    Onglet::Courses,
    Onglet::Recettes,
    Onglet::Prix,
];

#[derive(Clone, Copy)]
enum Compteur {
    Personnes,
    Repas,
}

/// Largeur de la barre latérale, en pixels.
const LARGEUR_NAVIGATION: f32 = 232.;

/// Marge intérieure horizontale de la feuille du contenu, en pixels.
const MARGE_FEUILLE: f32 = 36.;

const JOURS: [&str; 7] = [
    "Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche",
];

/// Nom du repas `i` d'un menu de `total` repas : un par jour jusqu'à sept,
/// midi et soir au-delà.
fn libelle_repas(i: usize, total: usize) -> String {
    if total <= 7 {
        JOURS[i % 7].to_owned()
    } else {
        let moment = if i.is_multiple_of(2) { "midi" } else { "soir" };
        format!("{} {moment}", JOURS[(i / 2) % 7])
    }
}

/// Nombre positif saisi : `60`, `60,50` ou `60.5`, avec ou sans `€`.
fn lire_nombre(texte: &str) -> Option<f64> {
    let texte = texte.trim().trim_end_matches('€').trim().replace(',', ".");
    texte
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite() && *n >= 0.)
}

/// Nombre à présenter dans un champ : `60`, `2,45`, `0,5`.
fn ecrire_nombre(nombre: f64) -> String {
    let texte = format!("{nombre:.2}");
    texte
        .trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', ",")
}

/// Quantité d'une recette : contrairement à la liste de courses, les pièces ne
/// sont pas arrondies (une demi-pâte à pizza reste une demi-pâte).
fn quantite_recette(quantite: f64, unite: Unite) -> String {
    match unite {
        Unite::Piece if (quantite - quantite.round()).abs() > 0.01 => {
            format!("× {quantite:.1}").replace('.', ",")
        }
        _ => quantite_lisible(quantite, unite),
    }
}

/// Option d'une liste déroulante : un libellé et la valeur qu'il désigne.
#[derive(Clone)]
struct Choix<V> {
    titre: SharedString,
    valeur: V,
}

impl<V: Clone + PartialEq + 'static> SearchableListItem for Choix<V> {
    type Value = V;

    fn title(&self) -> SharedString {
        self.titre.clone()
    }

    fn value(&self) -> &V {
        &self.valeur
    }
}

type Liste<V> = SearchableVec<Choix<V>>;

fn liste<V: Clone + PartialEq + 'static>(choix: impl IntoIterator<Item = (String, V)>) -> Liste<V> {
    SearchableVec::new(
        choix
            .into_iter()
            .map(|(titre, valeur)| Choix {
                titre: titre.into(),
                valeur,
            })
            .collect::<Vec<_>>(),
    )
}

/// Liste déroulante, éventuellement avec une valeur déjà choisie.
fn selecteur<V: Clone + PartialEq + 'static, T: 'static>(
    choix: Liste<V>,
    initiale: Option<&V>,
    recherche: bool,
    window: &mut Window,
    cx: &mut Context<T>,
) -> Entity<SelectState<Liste<V>>> {
    cx.new(|cx| {
        let mut etat = SelectState::new(choix, None, window, cx).searchable(recherche);
        if let Some(valeur) = initiale {
            etat.set_selected_value(valeur, window, cx);
        }
        etat
    })
}

pub struct Mijot {
    cat: Catalogue,
    etat: Etat,
    onglet: Onglet,
    budget: Entity<InputState>,
    filtre: Entity<InputState>,
    extra: Entity<InputState>,
    /// Identifiant de la recette ouverte dans le volet de droite.
    detail: Option<i64>,
    /// Recette en cours de création ou de modification, dans le même volet.
    editeur: Option<recettes::Editeur>,
    prix: prix::Prix,
    /// Suggestion de prix ouverte dans le volet de l'onglet des prix.
    suggestion: Option<suggestion::Volet>,
    /// Dernier message : (succès, texte).
    avis: Option<(bool, String)>,
    rng: fastrand::Rng,
    _abonnements: Vec<Subscription>,
}

impl Mijot {
    /// `alerte` : problème rencontré à l'ouverture de la base, à afficher d'entrée.
    pub fn new(
        cat: Catalogue,
        etat: Etat,
        alerte: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let budget = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Budget")
                .default_value(ecrire_nombre(etat.budget))
        });
        let filtre = cx.new(|cx| InputState::new(window, cx).placeholder("Chercher une recette…"));
        let extra = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Ajouter un article (café, lessive…)")
        });
        let abonnements = vec![
            cx.subscribe(&budget, |this, input, event: &InputEvent, cx| match event {
                InputEvent::Change => {
                    if let Some(b) = lire_nombre(&input.read(cx).value())
                        && b != this.etat.budget
                    {
                        this.etat.budget = b;
                        this.sauver(cx);
                    }
                }
                InputEvent::PressEnter { .. } => this.generer(cx),
                _ => {}
            }),
            cx.subscribe(&filtre, |_, _, _: &InputEvent, cx| cx.notify()),
            cx.subscribe_in(&extra, window, |this, _, event: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { .. } = event {
                    this.ajouter_extra(window, cx);
                }
            }),
        ];
        let prix = prix::Prix::new(&cat, window, cx);
        let mut mijot = Self {
            cat,
            etat,
            onglet: Onglet::Menu,
            budget,
            filtre,
            extra,
            detail: None,
            editeur: None,
            prix,
            suggestion: None,
            avis: None,
            rng: fastrand::Rng::new(),
            _abonnements: abonnements,
        };
        // Premier lancement : un menu tout de suite plutôt qu'une page vide.
        if mijot.etat.menu.is_empty() {
            mijot.generer(cx);
        }
        if let Some(alerte) = alerte {
            mijot.avis = Some((false, alerte));
        }
        mijot
    }

    // --- Actions -------------------------------------------------------------

    fn sauver(&mut self, cx: &mut Context<Self>) {
        if let Err(e) = self.etat.enregistrer() {
            self.avis = Some((false, format!("Enregistrement impossible : {e}")));
        }
        cx.notify();
    }

    fn generer(&mut self, cx: &mut Context<Self>) {
        self.avis = None;
        match generer(&self.cat, &self.etat.parametres(), &mut self.rng) {
            Ok(menu) => {
                self.etat.definir_menu(&self.cat, &menu);
                // Nouvelle semaine, nouvelle liste : plus rien n'est coché.
                self.etat.coches.clear();
            }
            Err(Echec::BudgetTropBas { minimum }) => {
                self.avis = Some((
                    false,
                    format!(
                        "Budget trop serré : le menu le moins cher pour {} repas à {} revient à environ {}. Augmente le budget ou réduis le nombre de repas.",
                        self.etat.repas,
                        self.etat.personnes,
                        euros(minimum)
                    ),
                ));
            }
            Err(Echec::PasAssezDeRecettes { disponibles }) => {
                self.avis = Some((
                    false,
                    format!("Seulement {disponibles} recettes correspondent à ces critères."),
                ));
            }
        }
        self.sauver(cx);
    }

    fn changer(&mut self, position: usize, cx: &mut Context<Self>) {
        self.avis = None;
        let mut menu = self.etat.menu_indices(&self.cat);
        if position >= menu.len() {
            return;
        }
        let possibles = remplacantes(&self.cat, &menu, position, &self.etat.parametres());
        if possibles.is_empty() {
            self.avis = Some((
                false,
                "Aucune autre recette ne tient dans le budget à cette place.".into(),
            ));
        } else {
            let ouverte = self.detail == Some(self.cat.recettes[menu[position]].id);
            menu[position] = possibles[self.rng.usize(..possibles.len())];
            if ouverte {
                self.detail = Some(self.cat.recettes[menu[position]].id);
            }
            self.etat.definir_menu(&self.cat, &menu);
        }
        self.sauver(cx);
    }

    fn retirer(&mut self, position: usize, cx: &mut Context<Self>) {
        self.avis = None;
        let mut menu = self.etat.menu_indices(&self.cat);
        if position < menu.len() {
            menu.remove(position);
            self.etat.definir_menu(&self.cat, &menu);
            self.sauver(cx);
        }
    }

    fn ajouter_au_menu(&mut self, recette: usize, cx: &mut Context<Self>) {
        self.avis = None;
        let mut menu = self.etat.menu_indices(&self.cat);
        if menu.contains(&recette) || menu.len() >= REPAS_MAX {
            return;
        }
        menu.push(recette);
        self.etat.definir_menu(&self.cat, &menu);
        self.avis = Some((
            true,
            format!("« {} » ajoutée au menu.", self.cat.recettes[recette].nom),
        ));
        self.sauver(cx);
    }

    fn ajuster(&mut self, compteur: Compteur, delta: i64, cx: &mut Context<Self>) {
        self.avis = None;
        match compteur {
            Compteur::Personnes => {
                let n = i64::from(self.etat.personnes) + delta;
                self.etat.personnes = n.clamp(1, i64::from(PERSONNES_MAX)) as u32;
            }
            Compteur::Repas => {
                let n = self.etat.repas as i64 + delta;
                self.etat.repas = n.clamp(1, REPAS_MAX as i64) as usize;
            }
        }
        self.sauver(cx);
    }

    fn cocher(&mut self, ingredient: i64, coche: bool, cx: &mut Context<Self>) {
        if coche {
            self.etat.coches.insert(ingredient);
        } else {
            self.etat.coches.remove(&ingredient);
        }
        self.sauver(cx);
    }

    fn ajouter_extra(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let nom = self.extra.read(cx).value().trim().to_owned();
        if nom.is_empty() {
            return;
        }
        self.etat.extras.push(Extra { nom, coche: false });
        self.extra.update(cx, |s, cx| s.set_value("", window, cx));
        self.sauver(cx);
    }

    fn tout_decocher(&mut self, cx: &mut Context<Self>) {
        self.avis = None;
        self.etat.coches.clear();
        for extra in &mut self.etat.extras {
            extra.coche = false;
        }
        self.sauver(cx);
    }

    /// Liste en texte, cases à cocher comprises, prête à coller dans une note.
    fn texte_liste(&self) -> String {
        let mut texte = String::from("Liste de courses\n");
        let menu = self.etat.menu_indices(&self.cat);
        for (rayon, lignes) in liste_de_courses(&self.cat, &menu, self.etat.personnes) {
            texte.push_str(&format!("\n{}\n", rayon.nom()));
            for l in lignes {
                let i = &self.cat.ingredients[l.ingredient];
                let case = if self.etat.coches.contains(&i.id) {
                    "x"
                } else {
                    " "
                };
                texte.push_str(&format!(
                    "- [{case}] {} : {}\n",
                    i.nom,
                    quantite_lisible(l.quantite, i.unite)
                ));
            }
        }
        if !self.etat.extras.is_empty() {
            texte.push_str("\nAutres articles\n");
            for e in &self.etat.extras {
                let case = if e.coche { "x" } else { " " };
                texte.push_str(&format!("- [{case}] {}\n", e.nom));
            }
        }
        texte
    }

    fn copier(&mut self, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(self.texte_liste()));
        self.avis = Some((true, "Liste copiée dans le presse-papiers.".into()));
        cx.notify();
    }

    // --- Éléments communs ----------------------------------------------------

    /// Articles de la liste de courses pas encore cochés.
    fn articles_restants(&self) -> usize {
        let menu = self.etat.menu_indices(&self.cat);
        liste_de_courses(&self.cat, &menu, self.etat.personnes)
            .iter()
            .flat_map(|(_, lignes)| lignes)
            .filter(|l| {
                !self
                    .etat
                    .coches
                    .contains(&self.cat.ingredients[l.ingredient].id)
            })
            .count()
            + self.etat.extras.iter().filter(|e| !e.coche).count()
    }

    /// Ligne de la barre latérale : pictogramme, libellé, pastille de compte.
    fn entree_navigation(
        id: impl Into<ElementId>,
        icone: IconName,
        libelle: &'static str,
        compte: Option<usize>,
        active: bool,
        cx: &App,
    ) -> Stateful<Div> {
        let theme = cx.theme();
        let (primaire, survol, rayon, surface, bordure, discret) = (
            theme.primary,
            theme.foreground.opacity(0.05),
            theme.radius,
            theme.group_box,
            theme.border,
            theme.muted_foreground,
        );
        h_flex()
            .id(id)
            .gap_3()
            .px_3()
            .py_2()
            .rounded(rayon)
            .text_sm()
            .cursor_pointer()
            .border_1()
            .border_color(gpui_kit::transparent_black())
            .when(active, |ligne| {
                ligne
                    .bg(surface)
                    .border_color(bordure)
                    .shadow(style::ombre(cx))
                    .font_weight(FontWeight::SEMIBOLD)
            })
            .when(!active, |ligne| {
                ligne
                    .text_color(discret)
                    .hover(move |ligne| ligne.bg(survol))
            })
            .child(
                Icon::new(icone)
                    .small()
                    .when(active, |icone| icone.text_color(primaire)),
            )
            .child(div().flex_1().min_w_0().child(libelle))
            .children(compte.filter(|n| *n > 0).map(|n| {
                div()
                    .px_2()
                    .rounded_full()
                    .bg(primaire.opacity(0.14))
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(primaire)
                    .child(n.to_string())
            }))
    }

    /// Barre latérale : nom de l'application, pages, bascule clair/sombre.
    fn navigation(&self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();
        let (discret, primaire, sur_primaire, sombre) = (
            theme.muted_foreground,
            theme.primary,
            theme.primary_foreground,
            theme.is_dark(),
        );
        // Le logo reprend l'accent, en dégradé vers un orange plus chaud.
        let logo = linear_gradient(
            145.,
            linear_color_stop(hsla(32. / 360., 0.95, 0.58, 1.), 0.),
            linear_color_stop(primaire, 1.),
        );
        let restants = self.articles_restants();
        let mut barre = v_flex()
            .w(px(LARGEUR_NAVIGATION))
            .flex_shrink_0()
            .gap_1()
            .p_3()
            .child(
                h_flex()
                    .gap_3()
                    .px_2()
                    .pt_2()
                    .pb_4()
                    .child(
                        div()
                            .flex()
                            .flex_shrink_0()
                            .items_center()
                            .justify_center()
                            .size(px(38.))
                            .rounded(px(12.))
                            .bg(logo)
                            .shadow(style::ombre(cx))
                            .child(
                                Icon::new(IconName::ChefHat)
                                    .with_size(Size::Size(px(22.)))
                                    .text_color(sur_primaire),
                            ),
                    )
                    .child(
                        v_flex()
                            .child(div().text_lg().font_weight(FontWeight::BOLD).child("Mijot"))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(discret)
                                    .child("Menus et courses"),
                            ),
                    ),
            );
        for (position, onglet) in ONGLETS.into_iter().enumerate() {
            let (icone, libelle, compte) = match onglet {
                Onglet::Menu => (IconName::CalendarDays, "Cette semaine", None),
                Onglet::Courses => (IconName::ShoppingCart, "Liste de courses", Some(restants)),
                Onglet::Recettes => (IconName::BookOpen, "Recettes", None),
                Onglet::Prix => (IconName::Coins, "Ingrédients et prix", None),
            };
            barre = barre.child(
                Self::entree_navigation(
                    ("page", position),
                    icone,
                    libelle,
                    compte,
                    self.onglet == onglet,
                    cx,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.onglet = onglet;
                    this.avis = None;
                    cx.notify();
                })),
            );
        }
        barre.child(div().flex_1()).child(
            Self::entree_navigation(
                "theme",
                if sombre {
                    IconName::Sun
                } else {
                    IconName::Moon
                },
                if sombre {
                    "Thème clair"
                } else {
                    "Thème sombre"
                },
                None,
                false,
                cx,
            )
            .on_click(cx.listener(|_, _, window, cx| {
                let mode = if cx.theme().is_dark() {
                    ThemeMode::Light
                } else {
                    ThemeMode::Dark
                };
                Theme::change(mode, Some(window), cx);
                // Le changement de mode recharge les couleurs neutres.
                appliquer_palette(cx);
            })),
        )
    }

    fn etiquettes(&self, recette: usize) -> Div {
        let r = &self.cat.recettes[recette];
        h_flex()
            .gap_1()
            .flex_wrap()
            .child(Tag::secondary().small().child(r.categorie.nom()))
            .when(!r.origine.is_empty(), |row| {
                row.child(Tag::primary().small().outline().child(r.origine.clone()))
            })
            .when(r.vege, |row| {
                row.child(Tag::success().small().child("Végétarien"))
            })
            .child(
                Tag::secondary()
                    .small()
                    .outline()
                    .child(format!("{} min", r.minutes)),
            )
    }
}

impl Render for Mijot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Thème rechargé hors de l'application (bascule clair/sombre du système) :
        // la palette est reposée, ce qui redessine la fenêtre.
        if !style::palette_en_place(cx) {
            cx.defer(appliquer_palette);
        }
        // Volet de droite : la recette ouverte ou en cours de modification à côté
        // du menu et des recettes, la suggestion de prix à côté des prix.
        let volet = match self.onglet {
            Onglet::Menu | Onglet::Recettes => self.volet(cx),
            Onglet::Prix => self.volet_suggestion(cx),
            Onglet::Courses => None,
        };
        // Colonnes de la grille des repas : ce qui tient dans la feuille du
        // contenu, une fois retirés la barre latérale, le volet et les marges.
        let largeur_volet = if volet.is_none() {
            0.
        } else if self.editeur.is_some() {
            468.
        } else {
            408.
        };
        let utile = f32::from(window.viewport_size().width)
            - LARGEUR_NAVIGATION
            - largeur_volet
            - 2. * MARGE_FEUILLE
            - 8.;
        let colonnes = (((utile + 16.) / 276.) as u16).clamp(1, 4);
        let page = match self.onglet {
            Onglet::Menu => self.page_menu(colonnes, cx),
            Onglet::Courses => self.page_courses(cx),
            Onglet::Recettes => self.page_recettes(cx),
            Onglet::Prix => self.page_prix(cx),
        };
        let avis = self.avis.as_ref().map(|(succes, texte)| {
            if *succes {
                Alert::success("avis", texte.clone())
            } else {
                Alert::warning("avis", texte.clone())
            }
        });
        let theme = cx.theme();
        // Le fond de la fenêtre est celui de la barre latérale ; le contenu et
        // le volet y sont posés comme deux feuilles flottantes.
        div()
            .flex()
            .size_full()
            .bg(theme.sidebar)
            .text_color(theme.foreground)
            .child(self.navigation(cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .gap_2()
                    .py_2()
                    .pr_2()
                    .child(
                        style::feuille(cx)
                            .id("contenu")
                            .flex_1()
                            .min_w_0()
                            .overflow_y_scroll()
                            .px(px(MARGE_FEUILLE))
                            .py_7()
                            .gap_4()
                            .children(avis)
                            .child(page),
                    )
                    .children(volet),
            )
    }
}

#[cfg(test)]
mod tests {
    // Pas de `use super::*` : il amènerait `gpui_kit::test`, qui masque `#[test]`.
    use super::{Unite, ecrire_nombre, libelle_repas, lire_nombre, quantite_recette};

    #[test]
    fn noms_des_repas() {
        assert_eq!(libelle_repas(0, 7), "Lundi");
        assert_eq!(libelle_repas(6, 7), "Dimanche");
        assert_eq!(libelle_repas(0, 8), "Lundi midi");
        assert_eq!(libelle_repas(13, 14), "Dimanche soir");
    }

    #[test]
    fn saisie_des_nombres() {
        assert_eq!(lire_nombre("60"), Some(60.));
        assert_eq!(lire_nombre(" 60,50 € "), Some(60.5));
        assert_eq!(lire_nombre("45.9"), Some(45.9));
        assert_eq!(lire_nombre(""), None);
        assert_eq!(lire_nombre("-5"), None);
        assert_eq!(lire_nombre("abc"), None);
        assert_eq!(ecrire_nombre(60.), "60");
        assert_eq!(ecrire_nombre(100.), "100");
        assert_eq!(ecrire_nombre(60.5), "60,5");
        assert_eq!(ecrire_nombre(2.45), "2,45");
        assert_eq!(ecrire_nombre(0.), "0");
        // Ce qui est écrit se relit à l'identique.
        for n in [0.33, 0.5, 12., 1.8, 30.] {
            assert_eq!(lire_nombre(&ecrire_nombre(n)), Some(n));
        }
    }

    #[test]
    fn quantites_des_recettes() {
        assert_eq!(quantite_recette(0.5, Unite::Piece), "× 0,5");
        assert_eq!(quantite_recette(2., Unite::Piece), "× 2");
        assert_eq!(quantite_recette(200., Unite::G), "200 g");
    }
}
