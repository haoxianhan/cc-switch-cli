# Provider And MCP Unsaved Changes Confirm Design

## Goal

When the user edits any dirty surface in the ratatui TUI and presses `Esc`, the app should show one consistent unsaved-changes confirm:

- `Enter`: save and exit
- `N`: exit without saving
- `Esc`: cancel

This should cover the existing text editor flow and the Provider/MCP form flows.

## Current Problem

The app currently has two different dirty-edit experiences:

- Text editors already use a save-before-close confirm.
- Provider and MCP forms used to close immediately on `Esc` and `q`.
- The first pass of this feature added a separate form discard confirm, which made the form flow inconsistent with the editor flow.

## Scope

In scope:

- Existing editor unsaved confirm
- Provider add form
- Provider edit form
- MCP add form
- MCP edit form
- Dirty close from `Esc` or `q`

Out of scope:

- Changing picker or overlay close behavior
- Extending this behavior to other routes

## Design

### 1. Form-level dirty detection

Both `ProviderAddFormState` and `McpAddFormState` will keep a normalized snapshot of their initial serialized content.

The dirty check will compare:

- the snapshot taken when the form is opened
- the current serialized content at the moment the user tries to close the form

This is preferred over a mutable `dirty` flag because form state changes come from several paths:

- direct text input
- picker selections
- toggles
- nested editors that write back into the form
- MCP env row add, edit, and delete

A snapshot comparison covers all of them without having to remember to set `dirty = true` in every branch.

### 2. Unified confirm flow

When a dirty editor, Provider form, or MCP form receives top-level `Esc` or `q`, the app opens an unsaved-changes confirm instead of closing immediately.

Behavior:

- `Enter`: save and exit
- `N`: exit without saving
- `Esc`: cancel and return to the dirty surface
- clean surface: close immediately, same as today

The user-facing behavior should be identical across editor and form surfaces. Internal actions may stay separate if that keeps the implementation simple, but they must produce the same visible confirm title, message, and key-bar semantics.

### 3. Overlay text

The confirm should say the current surface has unsaved changes and present the unified save / exit-without-save / cancel flow.

The current form-specific "discard" wording should be removed from the dirty-form path because it conflicts with the required `Enter = save and exit` behavior.

### 4. Boundaries

The dirty check belongs to the form state and form close path, not to the route layer.

- Form state owns snapshot creation and comparison.
- Surface-specific handlers decide whether to close directly or open the confirm.
- Confirm handling performs either:
  - save and exit
  - exit without saving
  - cancel

This keeps the logic local to the TUI form surface and avoids spreading form-specific conditions into unrelated route handlers.

## File Plan

Expected updates:

- `src-tauri/src/cli/tui/form.rs`
  - add initial snapshot fields to Provider and MCP form state
- `src-tauri/src/cli/tui/form/provider_state.rs`
  - initialize and compare Provider snapshots
- `src-tauri/src/cli/tui/form/mcp.rs`
  - initialize and compare MCP snapshots
- `src-tauri/src/cli/tui/app/types.rs`
  - keep or adjust confirm actions so form and editor dirty confirms can share the same visible behavior
- `src-tauri/src/cli/tui/app/form_handlers/mod.rs`
  - intercept top-level form close and open unified unsaved confirm when dirty
- `src-tauri/src/cli/tui/app/overlay_handlers/dialogs.rs`
  - handle save / exit-without-save / cancel for dirty forms
- `src-tauri/src/cli/tui/app/editor_handlers.rs`
  - keep editor dirty confirm aligned with the unified semantics
- `src-tauri/src/cli/i18n/texts/...`
  - reuse or adjust strings so dirty confirms no longer show duplicate cancel semantics
- `src-tauri/src/cli/tui/app/tests.rs`
  - cover unified dirty-confirm behavior for forms
- `src-tauri/src/cli/tui/ui/overlay/basic.rs`
  - render the same key-bar semantics for dirty editor and dirty form confirms
- `src-tauri/src/cli/tui/ui/tests.rs`
  - verify the visible key-bar text

## Testing

Add focused tests for:

- dirty editor + `Esc` opens unified confirm
- dirty Provider add form + `Esc` opens unified confirm
- dirty Provider form + `Enter` on confirm saves and exits
- dirty Provider form + `N` exits without saving
- dirty Provider form + `Esc` cancels and preserves edits
- dirty MCP form + `Enter` on confirm saves and exits
- dirty MCP form + `N` exits without saving
- dirty MCP form + `Esc` cancels and preserves edits
- clean editor/form surfaces still close immediately

The tests should target the maintained TUI behavior and stay narrow. No broad refactor is needed.

## Risks And Mitigations

- Snapshot drift because of non-deterministic serialization
  - Mitigation: compare the same internal serialization path used by the form state, not ad hoc display text.
- Missing a mutation path
  - Mitigation: use snapshot comparison instead of manual dirty flags.
- Behavior drift between editor and form confirms
  - Mitigation: assert the same visible key-bar semantics in UI tests and the same close outcomes in app tests.
- Saving forms from the confirm path could bypass normal validation
  - Mitigation: route confirm-save through the existing save submit path instead of inventing a second save implementation.

## Success Criteria

- Dirty editor, Provider, and MCP surfaces all show the same save / exit-without-save / cancel behavior.
- A modified Provider form does not close immediately on `Esc`.
- A modified MCP form does not close immediately on `Esc`.
- Clean editor/form surfaces still close immediately.
- `Enter` saves and exits.
- `N` exits without saving.
- `Esc` returns to the current dirty surface with edits intact.
