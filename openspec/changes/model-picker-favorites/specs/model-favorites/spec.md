## ADDED Requirements

### Requirement: Favorite toggle on model items

Each model item in the picker SHALL display a star icon button on the right side. Clicking the star SHALL toggle the model's favorite status. Clicking the star MUST NOT trigger model selection.

#### Scenario: Favorite a model
- **WHEN** user clicks the star icon on a non-favorited model
- **THEN** the star becomes filled (solid)
- **AND** the model is removed from its provider group
- **AND** the model appears in the "Favorites" section at the top of the list

#### Scenario: Unfavorite a model
- **WHEN** user clicks the star icon on an already-favorited model
- **THEN** the star becomes outlined (empty)
- **AND** the model is removed from the "Favorites" section
- **AND** the model reappears in its original provider group

#### Scenario: Star click does not select model
- **WHEN** user clicks the star icon on a model
- **THEN** the model picker does NOT close
- **AND** the selected model does NOT change

### Requirement: Favorites section

When the model picker is open and the user is NOT searching, the system SHALL display a "Favorites" section at the top of the list, above "Last used". This section SHALL only appear when at least one model is favorited.

#### Scenario: Favorites section with favorited models
- **WHEN** user has at least one favorited model and opens the picker (not searching)
- **THEN** a "Favorites" section is shown at the top
- **AND** each favorited model appears with a filled star icon

#### Scenario: No favorites
- **WHEN** user has no favorited models and opens the picker
- **THEN** the "Favorites" section is NOT displayed

### Requirement: Favorites during search

When the user is actively searching, the "Favorites" section SHALL NOT be displayed. Favorited models SHALL appear inline within their provider groups with the filled star visible.

#### Scenario: Search matches a favorited model
- **WHEN** user searches for a term that matches a favorited model
- **THEN** the model appears inline in its provider group (not in a separate Favorites section)
- **AND** the model shows a filled star icon

### Requirement: Favorites persistence

Favorite model IDs SHALL be persisted per project to localStorage under the key `circulo:favoriteModels`. Persistence SHALL use the same `atomWithStorage` pattern as `circulo:projectModels`.

#### Scenario: Favorites survive page reload
- **WHEN** user favorites a model and reloads the page
- **THEN** the model is still marked as favorite
- **AND** appears in the "Favorites" section when opening the picker

#### Scenario: Favorites are per-project
- **WHEN** user has favorited models in project A
- **AND** switches to project B
- **THEN** project B does NOT show project A's favorites
