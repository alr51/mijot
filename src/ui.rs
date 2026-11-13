//! Fenêtre principale : menu de la semaine et liste de courses. Le catalogue
//! (recettes, ingrédients et prix) a ses écrans dans les sous-modules.

mod prix;
mod recettes;
mod suggestion;

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Sizable as _, Theme, ThemeMode,
    alert::Alert,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputEvent, InputState},
    progress::Progress,
    searchable_list::SearchableListItem,
    select::{SearchableVec, SelectState},
    switch::Switch,
    tab::{Tab, TabBar},
    tag::Tag,
    v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::catalogue::Catalogue;
use crate::donnees::Unite;
use crate::etat::{Etat, Extra};
use crate::plan::{
    Echec, PERSONNES_MAX, REPAS_MAX, cout_menu, cout_par_personne, euros, generer,
    liste_de_courses, quantite_lisible, remplacantes,
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

    fn bouton_theme(&self, cx: &mut Context<Self>) -> Button {
        let sombre = cx.theme().is_dark();
        Button::new("theme")
            .small()
            .ghost()
            .icon(if sombre {
                IconName::Sun
            } else {
                IconName::Moon
            })
            .tooltip(if sombre {
                "Thème clair"
            } else {
                "Thème sombre"
            })
            .on_click(cx.listener(|_, _, window, cx| {
                let mode = if cx.theme().is_dark() {
                    ThemeMode::Light
                } else {
                    ThemeMode::Dark
                };
                Theme::change(mode, Some(window), cx);
            }))
    }

    fn onglets(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let menu = self.etat.menu_indices(&self.cat);
        let restants = liste_de_courses(&self.cat, &menu, self.etat.personnes)
            .iter()
            .flat_map(|(_, lignes)| lignes)
            .filter(|l| {
                !self
                    .etat
                    .coches
                    .contains(&self.cat.ingredients[l.ingredient].id)
            })
            .count()
            + self.etat.extras.iter().filter(|e| !e.coche).count();
        let libelle = |o: Onglet| match o {
            Onglet::Menu => "Menu de la semaine".to_owned(),
            Onglet::Courses if restants > 0 => format!("Liste de courses ({restants})"),
            Onglet::Courses => "Liste de courses".to_owned(),
            Onglet::Recettes => format!("Recettes ({})", self.cat.recettes.len()),
            Onglet::Prix => "Ingrédients et prix".to_owned(),
        };
        let selection = ONGLETS.iter().position(|o| *o == self.onglet).unwrap_or(0);
        TabBar::new("onglets")
            .underline()
            .px_4()
            .selected_index(selection)
            .suffix(div().pr_3().child(self.bouton_theme(cx)))
            .children(ONGLETS.iter().map(|o| Tab::new().label(libelle(*o))))
            .on_click(cx.listener(|this, ix: &usize, _, cx| {
                if let Some(o) = ONGLETS.get(*ix) {
                    this.onglet = *o;
                    this.avis = None;
                    cx.notify();
                }
            }))
    }

    fn etiquettes(&self, recette: usize) -> Div {
        let r = &self.cat.recettes[recette];
        h_flex()
            .gap_1()
            .flex_wrap()
            .child(Tag::secondary().small().child(r.categorie.nom()))
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

    // --- Menu ----------------------------------------------------------------

    fn compteur(&self, compteur: Compteur, cx: &mut Context<Self>) -> Div {
        let (libelle, valeur, ids) = match compteur {
            Compteur::Personnes => (
                "Personnes",
                self.etat.personnes as usize,
                ("p-moins", "p-plus"),
            ),
            Compteur::Repas => ("Repas", self.etat.repas, ("r-moins", "r-plus")),
        };
        h_flex()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(libelle),
            )
            .child(
                Button::new(ids.0)
                    .small()
                    .outline()
                    .icon(IconName::Minus)
                    .on_click(cx.listener(move |this, _, _, cx| this.ajuster(compteur, -1, cx))),
            )
            .child(
                div()
                    .w(px(24.))
                    .text_center()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(valeur.to_string()),
            )
            .child(
                Button::new(ids.1)
                    .small()
                    .outline()
                    .icon(IconName::Plus)
                    .on_click(cx.listener(move |this, _, _, cx| this.ajuster(compteur, 1, cx))),
            )
    }

    fn reglages(&self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();
        let (bordure, fond, discret, rayon) = (
            theme.border,
            theme.group_box,
            theme.muted_foreground,
            theme.radius,
        );
        h_flex()
            .flex_wrap()
            .gap_x_6()
            .gap_y_3()
            .p_4()
            .rounded(rayon)
            .border_1()
            .border_color(bordure)
            .bg(fond)
            .child(
                h_flex()
                    .gap_2()
                    .child(div().text_sm().text_color(discret).child("Budget"))
                    .child(div().w(px(88.)).child(Input::new(&self.budget)))
                    .child(div().text_sm().text_color(discret).child("€ / semaine")),
            )
            .child(self.compteur(Compteur::Personnes, cx))
            .child(self.compteur(Compteur::Repas, cx))
            .child(
                Switch::new("vege")
                    .checked(self.etat.vege)
                    .label("Végétarien")
                    .on_click(cx.listener(|this, on: &bool, _, cx| {
                        this.etat.vege = *on;
                        this.avis = None;
                        this.sauver(cx);
                    })),
            )
            .child(div().flex_1())
            .child(
                Button::new("generer")
                    .primary()
                    .icon(IconName::RefreshCw)
                    .label(if self.etat.menu.is_empty() {
                        "Générer le menu"
                    } else {
                        "Nouveau menu"
                    })
                    .on_click(cx.listener(|this, _, _, cx| this.generer(cx))),
            )
    }

    fn bilan(&self, menu: &[usize], cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();
        let cout = cout_menu(&self.cat, menu, self.etat.personnes);
        let budget = self.etat.budget;
        let depasse = cout > budget + 1e-9;
        let couleur = if depasse { theme.danger } else { theme.success };
        let portions = (menu.len() as f64 * f64::from(self.etat.personnes)).max(1.);
        let reste = if depasse {
            format!("dépassement de {}", euros(cout - budget))
        } else {
            format!("reste {}", euros(budget - cout))
        };
        let part = if budget > 0. {
            (cout / budget * 100.).min(100.) as f32
        } else {
            100.
        };
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .justify_between()
                    .items_end()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_end()
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(couleur)
                                    .child(euros(cout)),
                            )
                            .child(
                                div()
                                    .pb_1()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child(format!("sur {} · {reste}", euros(budget))),
                            ),
                    )
                    .child(
                        div()
                            .pb_1()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!(
                                "{} repas · {} par assiette",
                                menu.len(),
                                euros(cout / portions)
                            )),
                    ),
            )
            .child(Progress::new("budget").value(part).color(couleur))
    }

    fn carte(
        &self,
        position: usize,
        total: usize,
        recette: usize,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let theme = cx.theme();
        let r = &self.cat.recettes[recette];
        let id = r.id;
        let par_personne = cout_par_personne(&self.cat, recette);
        let ouverte = self.detail == Some(id);
        v_flex()
            .id(("carte", position))
            .w(px(250.))
            .p_4()
            .gap_2()
            .rounded(theme.radius)
            .border_1()
            .border_color(if ouverte { theme.primary } else { theme.border })
            .bg(theme.group_box)
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.detail = Some(id);
                cx.notify();
            }))
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(libelle_repas(position, total)),
            )
            .child(
                div()
                    .h(px(48.))
                    .line_height(px(24.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .line_clamp(2)
                    .child(r.nom.clone()),
            )
            .child(self.etiquettes(recette))
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(format!(
                        "≈ {} · {} par personne",
                        euros(par_personne * f64::from(self.etat.personnes)),
                        euros(par_personne)
                    )),
            )
            .child(
                h_flex()
                    .gap_2()
                    .justify_between()
                    .child(
                        Button::new(("changer", position))
                            .small()
                            .outline()
                            .icon(IconName::RefreshCw)
                            .label("Changer")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.changer(position, cx);
                            })),
                    )
                    .child(
                        Button::new(("retirer", position))
                            .small()
                            .ghost()
                            .icon(IconName::X)
                            .tooltip("Retirer du menu")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.retirer(position, cx);
                            })),
                    ),
            )
    }

    fn page_menu(&self, cx: &mut Context<Self>) -> Div {
        let menu = self.etat.menu_indices(&self.cat);
        let discret = cx.theme().muted_foreground;
        let mut page = v_flex().gap_5().child(self.reglages(cx));
        if menu.is_empty() {
            return page.child(
                div()
                    .py_10()
                    .text_center()
                    .text_color(discret)
                    .child("Indique ton budget, puis clique sur « Générer le menu »."),
            );
        }
        page = page.child(self.bilan(&menu, cx));
        let total = menu.len();
        let mut cartes = h_flex().flex_wrap().items_stretch().gap_4();
        for (position, &recette) in menu.iter().enumerate() {
            cartes = cartes.child(self.carte(position, total, recette, cx));
        }
        page.child(cartes).child(
            div().text_xs().text_color(discret).child(
                "Prix au prorata des quantités utilisées, d'après l'onglet « Ingrédients et prix » : un repère pour tenir le budget, pas un ticket de caisse.",
            ),
        )
    }

    // --- Liste de courses ----------------------------------------------------

    fn page_courses(&self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();
        let (discret, bordure) = (theme.muted_foreground, theme.border);
        let menu = self.etat.menu_indices(&self.cat);
        let liste = liste_de_courses(&self.cat, &menu, self.etat.personnes);
        let total: f64 = liste.iter().flat_map(|(_, l)| l).map(|l| l.prix).sum();
        let articles = liste.iter().map(|(_, l)| l.len()).sum::<usize>() + self.etat.extras.len();
        let coches = self.etat.coches.len() + self.etat.extras.iter().filter(|e| e.coche).count();

        let mut page = v_flex().max_w(px(720.)).gap_5().child(
            h_flex()
                .justify_between()
                .child(
                    v_flex()
                        .child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(format!("Total estimé : {}", euros(total))),
                        )
                        .child(div().text_sm().text_color(discret).child(format!(
                            "{coches} sur {articles} articles cochés · pour {} personne(s)",
                            self.etat.personnes
                        ))),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("decocher")
                                .ghost()
                                .label("Tout décocher")
                                .disabled(coches == 0)
                                .on_click(cx.listener(|this, _, _, cx| this.tout_decocher(cx))),
                        )
                        .child(
                            Button::new("copier")
                                .outline()
                                .icon(IconName::Copy)
                                .label("Copier la liste")
                                .disabled(articles == 0)
                                .on_click(cx.listener(|this, _, _, cx| this.copier(cx))),
                        ),
                ),
        );

        if liste.is_empty() {
            page = page.child(
                div()
                    .text_sm()
                    .text_color(discret)
                    .child("Le menu est vide : génère un menu pour remplir la liste."),
            );
        }

        for (rayon, lignes) in &liste {
            let sous_total: f64 = lignes.iter().map(|l| l.prix).sum();
            let mut section = v_flex().gap_1().child(
                h_flex()
                    .justify_between()
                    .pb_1()
                    .border_b_1()
                    .border_color(bordure)
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(rayon.nom()))
                    .child(div().text_sm().text_color(discret).child(euros(sous_total))),
            );
            for ligne in lignes {
                let i = &self.cat.ingredients[ligne.ingredient];
                let id = i.id;
                let coche = self.etat.coches.contains(&id);
                section = section.child(
                    h_flex()
                        .justify_between()
                        .py_1()
                        .when(coche, |row| row.opacity(0.45))
                        .child(
                            Checkbox::new(("ingredient", ligne.ingredient))
                                .checked(coche)
                                .label(i.nom.clone())
                                .on_click(cx.listener(move |this, on: &bool, _, cx| {
                                    this.cocher(id, *on, cx)
                                })),
                        )
                        .child(
                            h_flex()
                                .gap_4()
                                .text_sm()
                                .child(div().child(quantite_lisible(ligne.quantite, i.unite)))
                                .child(
                                    div()
                                        .w(px(64.))
                                        .text_right()
                                        .text_color(discret)
                                        .child(euros(ligne.prix)),
                                ),
                        ),
                );
            }
            page = page.child(section);
        }

        let mut autres = v_flex().gap_1().child(
            div()
                .pb_1()
                .border_b_1()
                .border_color(bordure)
                .font_weight(FontWeight::SEMIBOLD)
                .child("Autres articles"),
        );
        for (position, extra) in self.etat.extras.iter().enumerate() {
            autres = autres.child(
                h_flex()
                    .justify_between()
                    .py_1()
                    .child(
                        div().when(extra.coche, |d| d.opacity(0.45)).child(
                            Checkbox::new(("extra", position))
                                .checked(extra.coche)
                                .label(extra.nom.clone())
                                .on_click(cx.listener(move |this, on: &bool, _, cx| {
                                    if let Some(e) = this.etat.extras.get_mut(position) {
                                        e.coche = *on;
                                        this.sauver(cx);
                                    }
                                })),
                        ),
                    )
                    .child(
                        Button::new(("supprimer-extra", position))
                            .small()
                            .ghost()
                            .icon(IconName::Trash)
                            .tooltip("Supprimer")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if position < this.etat.extras.len() {
                                    this.etat.extras.remove(position);
                                    this.sauver(cx);
                                }
                            })),
                    ),
            );
        }
        autres = autres.child(
            h_flex()
                .gap_2()
                .pt_2()
                .child(div().flex_1().child(Input::new(&self.extra)))
                .child(
                    Button::new("ajouter-extra")
                        .outline()
                        .icon(IconName::Plus)
                        .label("Ajouter")
                        .on_click(
                            cx.listener(|this, _, window, cx| this.ajouter_extra(window, cx)),
                        ),
                ),
        );

        page.child(autres).child(
            div().text_xs().text_color(discret).child(
                "Quantités cumulées de tous les repas ; les articles à la pièce sont arrondis à l'unité supérieure.",
            ),
        )
    }
}

impl Render for Mijot {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page = match self.onglet {
            Onglet::Menu => self.page_menu(cx),
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
        // Volet de droite : la recette ouverte ou en cours de modification à côté
        // du menu et des recettes, la suggestion de prix à côté des prix.
        let volet = match self.onglet {
            Onglet::Menu | Onglet::Recettes => self.volet(cx),
            Onglet::Prix => self.volet_suggestion(cx),
            Onglet::Courses => None,
        };
        let theme = cx.theme();
        v_flex()
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(self.onglets(cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(
                        v_flex()
                            .id("contenu")
                            .flex_1()
                            .min_w_0()
                            .overflow_y_scroll()
                            .p_6()
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
