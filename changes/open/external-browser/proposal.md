# Proposal: External File Browser Integration
**Status: Note**

## Intent
`tj` ships its own minimal file browser. A dedicated TUI file browser such as Yazi provides a far richer experience (preview, bookmarks, search, bulk selection, mouse support, extensibility) essentially for free. If `tj` can delegate file selection to an external browser, the built-in browser could be removed or kept only as a fallback, reducing code to maintain and improving the user experience.

## Unresolved
- **Integration mechanism**: Yazi exposes a `ya emit open` protocol and can print the chosen path on stdout when launched in picker mode (`yazi --chooser-file`). Does this compose cleanly with `tj`'s TUI lifecycle (suspending/resuming the alternate screen)?
- **Fallback**: should the built-in browser be retained when Yazi (or the configured external browser) is not installed?
- **Scope of replacement**: file selection only, or also directory navigation and playlist population?
- **Other candidates**: `lf`, `ranger`, `fzf` (non-TUI but composable). Is Yazi the right default, or should the external browser be user-configurable?
- **Alternate-screen conflict**: both `tj` and Yazi use the alternate screen. Suspending `tj`'s terminal state cleanly before launching and restoring it on return needs verification.
