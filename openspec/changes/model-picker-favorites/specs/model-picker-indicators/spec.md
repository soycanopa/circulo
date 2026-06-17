## ADDED Requirements

### Requirement: Reasoning indicator uses brain icon

The reasoning capability indicator on model items SHALL be a `BrainIcon` from lucide-react instead of the text label "reasoning". The icon SHALL be rendered with the same container styling (`shrink-0 rounded bg-muted p-0.5`) and SHALL include `aria-label="Reasoning"` for accessibility.

#### Scenario: Reasoning model displays brain icon
- **WHEN** a model has `capabilities.reasoning === true`
- **THEN** a `BrainIcon` is displayed next to the model name
- **AND** the text "reasoning" is NOT displayed

#### Scenario: Non-reasoning model displays no indicator
- **WHEN** a model does NOT have reasoning capability
- **THEN** no brain icon is displayed
- **AND** no text label is displayed

### Requirement: Selection indicator is a left-side dot

The system SHALL indicate the currently selected model with a `CircleIcon` dot on the LEFT side of the model name (before the name text). The right-side `CheckIcon` SHALL be removed entirely.

#### Scenario: Selected model shows dot
- **WHEN** a model is the currently active/selected model
- **THEN** a filled `CircleIcon` dot is displayed on the left side of the model name
- **AND** the dot uses `text-primary` color
- **AND** no `CheckIcon` is shown on the right side

#### Scenario: Non-selected model shows placeholder
- **WHEN** a model is NOT the currently active model
- **THEN** a transparent placeholder of the same size as the dot is shown on the left
- **AND** model names remain aligned across all items
