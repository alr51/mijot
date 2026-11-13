# Mijot

Menus de la semaine à budget donné et liste de courses, en application de bureau
100 % locale (Rust, [GPUI Kit](https://gpui-kit.com)).

- **Menu de la semaine** : budget, nombre de personnes et de repas (1 à 14), option
  végétarienne. Le menu proposé tient dans le budget ; chaque repas peut être changé
  ou retiré.
- **Liste de courses** : ingrédients cumulés de tous les repas, par rayon, avec prix
  estimés, cases à cocher, articles ajoutés à la main et copie dans le presse-papiers.
- **Recettes** : le catalogue (40 recettes au départ), avec recherche (nom, catégorie
  ou ingrédient), quantités à l'échelle et étapes. Chaque recette peut être modifiée
  ou supprimée, et tu peux créer les tiennes.
- **Ingrédients et prix** : le prix de chaque ingrédient se modifie sur place
  (enregistré dès la saisie) ; ajout d'ingrédients, suppression de ceux qu'aucune
  recette n'utilise.

## Lancer

```sh
cargo run --release
```

La première compilation est longue (GPUI). Le toolchain est fixé par
`rust-toolchain.toml`.

```sh
cargo test    # catalogue SQLite, planificateur, liste de courses, état
```

## Organisation

| Fichier               | Rôle                                                                |
| --------------------- | ------------------------------------------------------------------- |
| `src/donnees.rs`      | Catalogue de départ, copié dans la base à sa création               |
| `src/catalogue.rs`    | Catalogue modifiable : schéma SQLite, lecture, écritures validées   |
| `src/plan.rs`         | Coûts, composition d'un menu dans le budget, liste de courses       |
| `src/etat.rs`         | État de l'interface (budget, menu, cases cochées), en JSON          |
| `src/ui.rs`           | Fenêtre, menu de la semaine, liste de courses                       |
| `src/ui/recettes.rs`  | Liste des recettes, volet de lecture, formulaire d'édition          |
| `src/ui/prix.rs`      | Onglet « Ingrédients et prix »                                      |

## Données

Tout est dans `~/.local/share/mijot/` :

- `mijot.db` (SQLite) : tables `ingredient`, `recette`, `recette_ingredient`, `etape`.
  La base est créée et remplie au premier lancement ; ensuite le catalogue de départ
  n'est plus relu. Pour repartir de zéro, supprime ce fichier.
- `etat.json` : réglages, menu en cours et cases cochées.

Si la base est illisible, l'application démarre sur le catalogue de départ en
mémoire et le signale : les modifications ne sont alors pas conservées.

## Prix

Les prix de départ sont des moyennes de grande surface, à ajuster dans l'onglet
« Ingrédients et prix ». Ils sont comptés au prorata des quantités utilisées (les
articles à la pièce sont arrondis à l'unité supérieure) : c'est un repère pour
comparer les menus et tenir un budget, pas une prévision de ticket de caisse.
