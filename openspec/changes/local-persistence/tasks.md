## 1. Schema and open

- [x] 1.1 Add persist dependencies (`rusqlite` bundled, `directories`, `tempfile` for tests)
- [x] 1.2 Implement `Store::open` / `open_default`, PRAGMAs, and migration v1
- [x] 1.3 Default path is Application Support `Circulo/circulo.sqlite`

## 2. Repository API

- [x] 2.1 Project CRUD: create, list active, list archived, archive, restore, delete (cascade)
- [x] 2.2 Session create/list/get/search/assign; unassigned vs by project; hide archived-project sessions
- [x] 2.3 Message save (sets `first_send_at` on first user message) and load with parts
- [x] 2.4 Sidebar view get/set with corrupt/missing fallback to `sessions`

## 3. Tests

- [x] 3.1 Unassigned session persists
- [x] 3.2 Cascade delete
- [x] 3.3 Archive hides and restore shows
- [x] 3.4 Assignment lock after first user message
- [x] 3.5 Preference default and corrupt fallback
- [x] 3.6 Title search
