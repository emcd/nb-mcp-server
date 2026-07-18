## ADDED Requirements
### Requirement: Automatic notebook creation
When the resolved notebook does not exist, the system SHALL create it before
executing the requested command, unless auto-creation is disabled.

#### Scenario: Create missing notebook
- **WHEN** the notebook name is resolved
- **AND** the notebook does not exist
- **THEN** the system creates the notebook and continues the request

#### Scenario: Auto-creation disabled
- **WHEN** the notebook name is resolved
- **AND** the notebook does not exist
- **AND** auto-creation is disabled
- **THEN** the system returns an error instructing how to create or enable it
