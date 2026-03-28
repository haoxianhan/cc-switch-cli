# MCP Env List Editor Design

## Background

Issue `#79` asks for one missing piece in the interactive MCP flow: env configuration for MCP servers.

The backend side is already close to what we need. `McpServer.server` is stored as raw JSON, so `server.env` can already persist in the unified config, database snapshot, and live sync adapters. Codex import/export already knows how to read and write `env`, and OpenCode already maps unified `env` to `environment` on write and back on import.

The gap is the TUI. The current MCP add/edit form only exposes `id`, `name`, `command`, `args`, and app toggles. There is no way to add or edit env pairs from the maintained interactive surface.

This design keeps the change tight:

- add env editing to the existing MCP add/edit form
- stay aligned with upstream env semantics
- avoid turning MCP editing into a generic JSON editor project

The worktree for this design is `/Users/saladday/dev/cc-switch-cli/.worktrees/issue-79-mcp-env` on branch `feat/issue-79-mcp-env`. Baseline verification in that worktree is green: `cargo build` passes and `cargo test` passes before any feature work starts.

## Goal

Add env editing to the interactive MCP add/edit flow so a user can manage simple env pairs without leaving the TUI.

The result should satisfy these conditions:

1. Users can add, edit, and delete env pairs from the MCP form.
2. The saved shape is upstream-compatible: `server.env` is a flat object of string values.
3. The interaction feels native to the current ratatui flow, not like a pasted JSON blob.
4. The implementation stays local to the MCP form and its save path.

## Scope

### In Scope

- Add an `Env` row to the existing MCP add/edit form.
- Add a route-local env list overlay for MCP form editing.
- Add a small row editor for one `key` and one `value`.
- Serialize env rows into `server.env`.
- Load existing `server.env` into the form.
- Add focused tests for form serialization and TUI interaction.

### Out of Scope

- A generic key/value editor reused across the whole TUI.
- A raw JSON editor for MCP env.
- Nested env objects, arrays, or repeated keys.
- Non-interactive CLI improvements for MCP env.
- A broader redesign of the MCP add/edit form.
- Full HTTP/SSE MCP editing parity. This work stays within the current stdio-style form model.

## Current Constraints

The current MCP form is already stdio-shaped. It exposes `Command` and `Args`, but not `type`, `url`, or `headers`. That means this feature should follow the existing form contract instead of trying to broaden the MCP editor in the same change.

There is also no existing key/value editor inside the MCP form. The nearest repo pattern is the OpenClaw config editing flow, but that flow is JSON-oriented and broader than what issue `#79` needs. Reusing it here would pull the MCP editor toward a generic config editor, which is out of scope.

## Chosen Approach

### Summary

Add a new `Env` row to the MCP add/edit form. That row is not edited inline. Pressing `Enter` on it opens a small env overlay that shows one row per env pair as `KEY = VALUE`.

Inside that overlay:

- `a` adds a row
- `Enter` edits the selected row
- `Delete` removes the selected row
- `Esc` returns to the MCP form

Adding and editing a row both use a small popup with two fields:

- `Key`
- `Value`

`Key` is required. `Value` may be empty.

This is the chosen design because it solves the main usability problem in the issue. Users do not need to learn or remember a `KEY=VALUE` text format, and the change still stays local to the MCP form.

### Why Not a Plain Text Field

A plain text field with one `KEY=VALUE` entry per line would be cheaper to implement, but it pushes format knowledge onto the user. That is the part the issue explicitly called out as awkward.

For this repo, the extra cost of a small list editor is worth it:

- the interaction is easier to discover
- the saved data is less likely to be malformed
- the TUI feels more intentional

## Interaction Design

### MCP Main Form

The MCP add/edit form gains one new field row:

- `Env`

The row shows a compact summary, not the full content:

- `None` when there are no env pairs
- `1 entry` when there is one pair
- `N entries` when there are multiple pairs

The row is selected like other MCP fields. Pressing `Enter` opens the env overlay instead of toggling inline edit mode.

The main form does not show masked values. The user explicitly asked for plain values inside the editor.

### Env List Overlay

The env overlay is a small route-local list UI owned by the MCP form. It is not a general overlay shared by unrelated routes.

Each row is rendered as:

`KEY = VALUE`

If the value is empty, the row is rendered as:

`KEY =`

If the list is empty, the overlay shows an explicit empty state instead of `N/A`.

Keyboard behavior:

- `Up` / `Down`: move selection
- `a`: add a new env row
- `Enter`: edit selected row
- `Delete`: remove selected row
- `Esc`: close the overlay and return to the MCP form

### Row Editor Popup

Adding and editing use the same two-field popup.

Fields:

1. `Key`
2. `Value`

Behavior:

- `Tab` switches between the two fields
- `Enter` confirms
- `Esc` cancels

Rules:

- `Key` is trimmed before validation and save
- `Key` must not be empty
- `Key` must be unique within the current env list
- `Value` is stored exactly as entered, except for normal text input handling
- `Value` may be empty

The popup stays open when validation fails. It should not close and force the user to reopen it just to fix a key.

## Data Shape

The saved MCP JSON remains aligned with the upstream shape:

```json
{
  "server": {
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-filesystem", "/"],
    "env": {
      "API_KEY": "abc",
      "PROJECT_ROOT": ""
    }
  }
}
```

`env` is a flat object of string values.

This feature does not introduce a new top-level MCP field, database column, or service-specific schema.

## Loading and Saving

### Loading Existing MCP Data

When opening MCP edit:

- read `server.env` if it is an object
- map each entry to one editable row
- sort rows by key for stable display and stable re-entry

Sorting by key is intentional. The stored shape is an object, not an array. A stable sorted list is easier to scan and avoids noisy ordering changes between edit sessions.

### Saving

The env overlay only edits MCP form state. It does not save on its own.

The final write still happens through the existing MCP form save path:

1. MCP form state serializes to `McpServer` JSON
2. `server.env` is rebuilt from the env row list
3. the existing `McpService::upsert_server` path persists the result
4. enabled live targets continue to sync through the current service layer

If the env list is empty, `server.env` is removed instead of writing an empty object.

## Validation and Error Handling

Validation rules are intentionally simple:

- empty key is rejected
- duplicate key is rejected
- empty value is allowed

This editor only supports simple string key/value pairs. Nested objects, arrays, or repeated keys are not supported.

That matches both the issue and the upstream contract. Upstream models MCP env as a flat string map. This design does not try to preserve malformed nested env content once a user edits and re-saves through the list editor.

Error handling rules:

- validation failure keeps the row editor open
- deleting a row is immediate inside the env overlay, but still local to the MCP form until the full form is saved
- closing the env overlay with `Esc` keeps in-form edits
- cancelling the row popup with `Esc` leaves the current row unchanged

## File-Level Design

The change should stay close to the existing MCP form files.

Expected touch points:

- `src-tauri/src/cli/tui/form.rs`
- `src-tauri/src/cli/tui/form/mcp.rs`
- `src-tauri/src/cli/tui/app/form_handlers/mcp.rs`
- `src-tauri/src/cli/tui/ui/forms/mcp.rs`
- `src-tauri/src/cli/tui/app/types.rs`
- `src-tauri/src/cli/i18n.rs`
- targeted MCP form and app tests

The preferred decomposition is:

- keep env row state inside `McpAddFormState`
- keep env overlay state local to the MCP form flow
- avoid introducing a reusable editor abstraction unless the current implementation clearly needs it

## Testing

The minimum verification set for implementation is:

### Form Serialization

- empty env list omits `server.env`
- one env row serializes to one object entry
- empty value serializes as `""`
- editing an existing MCP server loads env rows correctly

### TUI Interaction

- selecting `Env` and pressing `Enter` opens the env overlay
- `a` opens the row editor for add
- `Enter` opens the row editor for edit
- `Delete` removes the selected row
- `Esc` closes the env overlay and returns to the MCP form
- duplicate key is rejected without closing the popup
- empty key is rejected without closing the popup

### Persistence Regression

- saving the MCP form writes `server.env` through the existing MCP save path
- saved env survives reload
- Codex/OpenCode paths continue to receive the same env data they already know how to sync

## Risks and Follow-Up

The main risk is scope creep. A list editor can easily turn into a generic key/value system or a full MCP schema editor. This design explicitly avoids that.

Known follow-up that is out of scope for this change:

- expose HTTP/SSE MCP fields in the TUI
- support raw JSON editing for advanced MCP specs
- factor a shared key/value editor if a second real use case appears later

## Final Decision

Implement MCP env editing in the TUI as a route-local list overlay with row-level add, edit, and delete.

This design was chosen over a plain text `KEY=VALUE` field because it better fits the TUI, avoids format guesswork, and still keeps the implementation local to the MCP form.
