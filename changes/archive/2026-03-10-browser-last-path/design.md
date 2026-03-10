# Design: Browser Remembers Last Visited Path
**Status: Draft**

## Approach

### Cache format
`Cache` currently serialises only the `entries` HashMap directly to JSON. Add `last_browser_path: Option<String>` as a sibling field by wrapping in a new `CacheFile` struct:

```rust
#[derive(Serialize, Deserialize, Default)]
struct CacheFile {
    #[serde(default)]
    last_browser_path: Option<String>,
    #[serde(default)]
    entries: HashMap<String, CacheEntry>,
}
```

On load, try deserialising as `CacheFile` first; if that fails, fall back to the old flat `HashMap<String, CacheEntry>` and migrate in-memory (no rewrite until next `save()`).

Add `Cache` methods:
- `last_browser_path() -> Option<PathBuf>`
- `set_last_browser_path(p: &Path)`

### run_browser return type
Change signature to return the final `cwd` regardless of exit reason:

```rust
fn run_browser(...) -> io::Result<(BrowserResult, PathBuf)>
```

The second element is `state.cwd` at the point of exit. This lets callers update `browser_dir` and save to cache after every browser session.

### Browser start directory
Introduce `browser_dir: PathBuf` in `main()`, computed once at startup:
- If the CLI arg is a directory → that directory.
- If the CLI arg is a file → the file's parent directory.
- Otherwise → `cache.last_browser_path()` if set and the path still exists, else CWD.

`browser_dir` is passed into `run_player()` as `&mut PathBuf`. After every browser session (startup browser or mid-session `b`), update it to the returned `cwd` and call `cache.set_last_browser_path` + `cache.save()`.

The "only the first time" behaviour falls out naturally: the CLI arg sets `browser_dir` once at startup; after the first browser close `browser_dir` is updated to wherever the user ended up, and all subsequent opens use that.

### Call-site changes
- `main()`: pass `browser_dir` to `run_browser`; after return update `browser_dir` and save cache.
- `run_player()`: accept `browser_dir: &mut PathBuf`; on `OpenBrowser` use `browser_dir.clone()` as start; after return update `browser_dir` and save cache. Remove the old `file_dir` argument used to root the browser.

## Tasks

1. ✓ **Impl**: Update `Cache` — add `CacheFile` wrapper struct, backward-compat load, `last_browser_path()`/`set_last_browser_path()` methods, update `save()`.
2. ✓ **Impl**: Change `run_browser` to return `(BrowserResult, PathBuf)`; update both call sites.
3. ✓ **Impl**: Add `browser_dir` computation in `main()`; thread it through `run_player()`; update `OpenBrowser` handler and startup browser call to use and update `browser_dir`.
4. ✓ **Verify**: No-arg launch opens last visited path; arg launch opens arg's dir on first open then last-visited on subsequent `b`; navigating and quitting persists path; old cache file loads without error.
5. ✓ **Process**: Archive
