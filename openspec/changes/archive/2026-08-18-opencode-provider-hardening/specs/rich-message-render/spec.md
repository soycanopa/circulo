## ADDED Requirements

### Requirement: Opaque reasoning is explained honestly

When an assistant message includes a reasoning part with no readable content (provider-encrypted or empty after the turn completes), the UI MUST show locale copy explaining that the provider hid the reasoning, instead of an empty expandable block.

#### Scenario: Encrypted reasoning after turn completes

- **GIVEN** a completed assistant message with a reasoning part whose content is empty
- **WHEN** the user expands reasoning
- **THEN** the UI shows human copy that reasoning is not available from the provider
- **AND** no fake or garbled text is shown
