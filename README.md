# Mijot

Menus de la semaine à budget donné et liste de courses, en application de bureau
(Rust, [GPUI Kit](https://gpui-kit.com)). Les données restent sur la machine ; le
réseau ne sert qu'à suggérer un prix, sur demande.

- **Menu de la semaine** : budget, nombre de personnes et de repas (1 à 14), option
  végétarienne. Le menu proposé tient dans le budget ; chaque repas peut être changé
  ou retiré.
- **Liste de courses** : ingrédients cumulés de tous les repas, par rayon, avec prix
  estimés, cases à cocher, articles ajoutés à la main et copie dans le presse-papiers.
- **Recettes** : le catalogue (64 recettes au départ), avec recherche (nom, catégorie
  ou ingrédient), quantités à l'échelle et étapes. Chaque recette peut être modifiée
  ou supprimée, et tu peux créer les tiennes.
- **Ingrédients et prix** : le prix de chaque ingrédient se modifie sur place
  (enregistré dès la saisie) ; ajout d'ingrédients, suppression de ceux qu'aucune
  recette n'utilise. « Suggérer » propose un prix d'après les relevés
  d'[Open Prices](https://prices.openfoodfacts.org).

## Lancer

```sh
cargo run --release
```

La première compilation est longue (GPUI). Le toolchain est fixé par
`rust-toolchain.toml`.

```sh
cargo test    # catalogue SQLite, planificateur, liste de courses, état, lecture des relevés (sans réseau)
```

## Organisation

| Fichier               | Rôle                                                                |
| --------------------- | ------------------------------------------------------------------- |
| `src/donnees.rs`      | Catalogue de départ, copié dans la base à sa création               |
| `src/catalogue.rs`    | Catalogue modifiable : schéma SQLite, lecture, écritures validées   |
| `src/plan.rs`         | Coûts, composition d'un menu dans le budget, liste de courses       |
| `src/prix_ouverts.rs` | Suggestions de prix : appels à Open Prices, calcul de la médiane    |
| `src/etat.rs`         | État de l'interface (budget, menu, cases cochées), en JSON          |
| `src/ui.rs`           | Fenêtre : état, actions, barre latérale                             |
| `src/ui/style.rs`     | Palette (claire et sombre), pictogrammes, briques de mise en page   |
| `src/ui/menu.rs`      | Page « Cette semaine »                                              |
| `src/ui/courses.rs`   | Page « Liste de courses »                                           |
| `src/ui/recettes.rs`  | Liste des recettes, volet de lecture, formulaire d'édition          |
| `src/ui/prix.rs`      | Onglet « Ingrédients et prix »                                      |
| `src/ui/suggestion.rs`| Volet « Prix suggéré »                                              |

## Données

Tout est dans `~/.local/share/mijot/` :

- `mijot.db` (SQLite) : tables `ingredient`, `recette`, `recette_ingredient`, `etape`.
  La base est créée et remplie au premier lancement ; ensuite le catalogue de départ
  n'est plus relu. Pour repartir de zéro, supprime ce fichier.
- `etat.json` : réglages, menu en cours et cases cochées.

Si la base est illisible, l'application démarre sur le catalogue de départ en
mémoire et le signale : les modifications ne sont alors pas conservées.

## Suggestions de prix

« Suggérer », sur un ingrédient, interroge deux services d'Open Food Facts :

1. la taxonomie, pour trouver la **catégorie** qui correspond à l'ingrédient
   (« Pâtes », « Lentilles corail »…). Le choix est retenu en base ;
2. Open Prices, pour les relevés récents de cette catégorie, en euros et hors
   promotions (100 au plus). Les produits emballés sont ramenés au kilo ou au litre
   grâce à leur contenance ; les produits bruts sont relevés au kilo ou à la pièce.

L'application propose le prix médian et le premier quartile (« prix bas »), d'après
les relevés français s'il y en a au moins trois, sinon tous ceux en euros. Rien
n'est modifié sans clic sur « Appliquer ».

Limites à connaître :

- les relevés sont contributifs : peu nombreux pour certains produits, et pas
  représentatifs de ton magasin (les enseignes bio y sont très présentes) ;
- une catégorie large mêle premiers prix, marques et produits haut de gamme :
  mieux vaut une catégorie précise ;
- pour un ingrédient compté à la pièce, seuls les produits relevés à l'unité sont
  utilisables (un paquet ne dit pas combien de pièces il contient).

Seuls le terme cherché et la catégorie choisie sont envoyés. L'application se
présente sous son nom et sa version (`mijot/0.1.0`, constante `AGENT` de
`src/prix_ouverts.rs`), sans contact ; Open Food Facts recommande d'en ajouter un. Les données d'Open Prices
sont sous licence ODbL.

## Prix

Les prix de départ sont des moyennes de grande surface, à ajuster dans l'onglet
« Ingrédients et prix ». Ils sont comptés au prorata des quantités utilisées (les
articles à la pièce sont arrondis à l'unité supérieure) : c'est un repère pour
comparer les menus et tenir un budget, pas une prévision de ticket de caisse.
