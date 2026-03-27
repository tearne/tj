use std::path::Path;

use lofty::config::WriteOptions;
use lofty::file::TaggedFileExt;
use lofty::tag::{Accessor, Tag, TagExt};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTagKey};
use symphonia::core::probe::Hint;


// Symphonia stores ID3v2 tags (MP3) in probed.metadata and container tags (FLAC, etc.)
// in probed.format.metadata(). Prefer probe-level (ID3v2) over format-level (ID3v1/container)
// so that richer ID3v2 data wins on files that carry both.
pub(crate) fn collect_tags(probed: &mut symphonia::core::probe::ProbeResult) -> Vec<symphonia::core::meta::Tag> {
    let from_probe: Option<Vec<_>> = probed.metadata.get()
        .and_then(|m| m.current().map(|r| r.tags().to_vec()))
        .filter(|t| !t.is_empty());
    from_probe.unwrap_or_else(|| {
        let meta = probed.format.metadata();
        meta.current().map(|r| r.tags().to_vec()).unwrap_or_default()
    })
}

pub(crate) fn read_tags_for_editor(path: &Path) -> [String; 7] {
    let empty = || std::array::from_fn(|_| String::new());
    let src = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return empty(),
    };
    let mss = MediaSourceStream::new(Box::new(src), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut probed = match symphonia::default::get_probe().format(
        &hint, mss, &FormatOptions::default(), &MetadataOptions::default(),
    ) {
        Ok(p) => p,
        Err(_) => return empty(),
    };
    let tags = collect_tags(&mut probed);
    let find = |key: StandardTagKey| {
        tags.iter()
            .find(|t| t.std_key == Some(key))
            .map(|t| t.value.to_string())
            .unwrap_or_default()
    };
    [
        find(StandardTagKey::Artist),
        find(StandardTagKey::TrackTitle),
        find(StandardTagKey::Album),
        find(StandardTagKey::Date),
        find(StandardTagKey::TrackNumber),
        find(StandardTagKey::Genre),
        find(StandardTagKey::Comment),
    ]
}

pub(crate) fn propose_rename_stem(path: &Path) -> String {
    let stem_fallback = || {
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string()
    };
    let src = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return stem_fallback(),
    };
    let mss = MediaSourceStream::new(Box::new(src), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut probed = match symphonia::default::get_probe().format(
        &hint, mss, &FormatOptions::default(), &MetadataOptions::default(),
    ) {
        Ok(p) => p,
        Err(_) => return stem_fallback(),
    };
    let tags = collect_tags(&mut probed);
    let find = |key: StandardTagKey| {
        tags.iter()
            .find(|t| t.std_key == Some(key))
            .map(|t| t.value.to_string())
    };
    match (find(StandardTagKey::Artist), find(StandardTagKey::TrackTitle)) {
        (Some(a), Some(t)) => format!("{} - {}", sanitise_for_filename(&t), sanitise_for_filename(&a)),
        _ => stem_fallback(),
    }
}

// Replace characters that are illegal or ambiguous in filenames.
// `/` is the only hard filesystem barrier on Linux, but we also strip the
// Windows-unsafe set so that proposed names are portable.
pub(crate) fn sanitise_for_filename(s: &str) -> String {
    s.chars().map(|c| match c {
        '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
        c => c,
    }).collect()
}

pub(crate) fn write_tags(path: &Path, fields: &[(String, usize)]) -> Result<(), String> {
    let mut tagged_file = lofty::read_from_path(path).map_err(|e| e.to_string())?;
    if tagged_file.primary_tag_mut().is_none() {
        let tag_type = tagged_file.primary_tag_type();
        tagged_file.insert_tag(Tag::new(tag_type));
    }
    let tag = tagged_file.primary_tag_mut()
        .expect("tag was just inserted so must be present");
    tag.set_artist(fields[0].0.clone());
    tag.set_title(fields[1].0.clone());
    tag.set_album(fields[2].0.clone());
    if let Ok(year) = fields[3].0.parse::<u32>() {
        tag.set_year(year);
    }
    if let Ok(track) = fields[4].0.parse::<u32>() {
        tag.set_track(track);
    }
    tag.set_genre(fields[5].0.clone());
    tag.set_comment(fields[6].0.clone());
    tag.save_to_path(path, WriteOptions::default()).map_err(|e| e.to_string())
}

pub(crate) fn read_cover_art(path: &Path) -> Option<Vec<u8>> {
    use lofty::picture::PictureType;
    let tagged_file = lofty::read_from_path(path).ok()?;
    let tag = tagged_file.primary_tag()?;
    let pictures = tag.pictures();
    let pic = pictures.iter()
        .find(|p| p.pic_type() == PictureType::CoverFront)
        .or_else(|| pictures.first())?;
    Some(pic.data().to_vec())
}

pub(crate) fn read_track_name(path: &str) -> String {
    let fallback = || {
        Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path)
            .to_string()
    };
    let src = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return fallback(),
    };
    let mss = MediaSourceStream::new(Box::new(src), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut probed = match symphonia::default::get_probe().format(
        &hint, mss, &FormatOptions::default(), &MetadataOptions::default(),
    ) {
        Ok(p) => p,
        Err(_) => return fallback(),
    };
    let tags = collect_tags(&mut probed);
    let find = |key: StandardTagKey| {
        tags.iter()
            .find(|t| t.std_key == Some(key))
            .map(|t| t.value.to_string())
    };
    let artist = find(StandardTagKey::Artist);
    let title = find(StandardTagKey::TrackTitle);
    match (artist, title) {
        (Some(a), Some(t)) => format!("{t} \u{2013} {a}"),
        (None, Some(t)) => t,
        _ => fallback(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Cleans itself up on drop.
    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("tj_rename_test_{nanos}"));
            std::fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }
        fn path(&self) -> &Path { &self.0 }
    }
    impl Drop for TempDir {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
    }

    /// Reads all audio files from `TEST_AUDIO_DIR`, copies them to a temp directory under
    /// their proposed names, then asserts:
    ///   1. Output count and total byte size match the input.
    ///   2. Re-running the rename logic on every output file proposes no further changes.
    ///
    /// Run with:  TEST_AUDIO_DIR=/path/to/music cargo test rename_roundtrip -- --nocapture
    #[test]
    fn rename_roundtrip() {
        let src_dir = match std::env::var("TEST_AUDIO_DIR") {
            Ok(d) => PathBuf::from(d),
            Err(_) => return, // env var not set — skip
        };

        fn collect_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
            let Ok(entries) = std::fs::read_dir(dir) else { return };
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() { collect_recursive(&path, out); }
                else if path.is_file() && crate::browser::is_audio(&path) { out.push(path); }
            }
        }
        let mut src_files: Vec<PathBuf> = Vec::new();
        collect_recursive(&src_dir, &mut src_files);

        assert!(!src_files.is_empty(), "no audio files found in TEST_AUDIO_DIR");

        let src_count = src_files.len();
        let src_bytes: u64 = src_files.iter()
            .map(|p| std::fs::metadata(p).unwrap().len())
            .sum();

        let out_dir = TempDir::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        for src in &src_files {
            let target_stem = propose_rename_stem(src);
            let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("");
            let dst_name = if ext.is_empty() {
                target_stem.clone()
            } else {
                format!("{target_stem}.{ext}")
            };
            assert!(
                seen.insert(dst_name.clone()),
                "naming collision: two source files map to '{dst_name}'"
            );
            let dst = out_dir.path().join(&dst_name);
            std::fs::copy(src, &dst)
                .unwrap_or_else(|e| panic!("copy failed for '{dst_name}': {e}"));
        }

        // --- assertion 1: count and total size preserved ---

        let dst_files: Vec<PathBuf> = std::fs::read_dir(out_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect();

        assert_eq!(dst_files.len(), src_count,
            "output file count ({}) differs from input ({})", dst_files.len(), src_count);

        let dst_bytes: u64 = dst_files.iter()
            .map(|p| std::fs::metadata(p).unwrap().len())
            .sum();
        assert_eq!(dst_bytes, src_bytes,
            "output total size ({dst_bytes} B) differs from input ({src_bytes} B)");

        // --- assertion 2: rename is idempotent ---
        // Mirrors the second-pass logic exactly: conforming stems are left alone,
        // so only non-conforming stems need to be stable under propose_rename_stem.

        for dst in &dst_files {
            let current_stem = dst.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let proposed = propose_rename_stem(dst);
            assert_eq!(proposed, current_stem,
                "rename not idempotent for '{}': proposed '{proposed}'",
                dst.display());
        }
    }

    #[test]
    fn sanitise_all_illegal() {
        let result = sanitise_for_filename(r#"/\:*?"<>|"#);
        assert!(!result.is_empty());
        assert!(result.chars().all(|c| c == '-'));
    }

    #[test]
    fn sanitise_empty() {
        assert_eq!(sanitise_for_filename(""), "");
    }

    #[test]
    fn sanitise_clean() {
        let s = "hello world";
        assert_eq!(sanitise_for_filename(s), s);
    }

    #[test]
    fn propose_rename_stem_non_empty() {
        let dir = match std::env::var("TEST_AUDIO_DIR") {
            Ok(d) => std::path::PathBuf::from(d),
            Err(_) => return,
        };
        fn first_audio(dir: &std::path::Path) -> Option<std::path::PathBuf> {
            let Ok(entries) = std::fs::read_dir(dir) else { return None };
            for e in entries.filter_map(|e| e.ok()) {
                let p = e.path();
                if p.is_dir() { if let Some(f) = first_audio(&p) { return Some(f); } }
                else if crate::browser::is_audio(&p) { return Some(p); }
            }
            None
        }
        if let Some(path) = first_audio(&dir) {
            let stem = propose_rename_stem(&path);
            assert!(!stem.is_empty(), "propose_rename_stem returned empty for {}", path.display());
            let tags = read_tags_for_editor(&path);
            let artist = &tags[0];
            let title  = &tags[1];
            if !artist.is_empty() && !title.is_empty() {
                let expected = format!("{} - {}", sanitise_for_filename(title), sanitise_for_filename(artist));
                assert_eq!(stem, expected,
                    "propose_rename_stem did not match tags for {}", path.display());
            }
        }
    }

    #[test]
    fn tag_write_roundtrip() {
        let dir = match std::env::var("TEST_AUDIO_DIR") {
            Ok(d) => std::path::PathBuf::from(d),
            Err(_) => return,
        };
        fn find_ext(dir: &std::path::Path, ext: &str) -> Option<std::path::PathBuf> {
            let Ok(entries) = std::fs::read_dir(dir) else { return None };
            for e in entries.filter_map(|e| e.ok()) {
                let p = e.path();
                if p.is_dir() { if let Some(f) = find_ext(&p, ext) { return Some(f); } }
                else if p.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case(ext)).unwrap_or(false) {
                    return Some(p);
                }
            }
            None
        }
        for ext in &["mp3", "flac"] {
            let Some(src) = find_ext(&dir, ext) else { continue };
            let tmp = TempDir::new();
            let dst = tmp.path().join(src.file_name().unwrap());
            std::fs::copy(&src, &dst).unwrap();
            let fields: Vec<(String, usize)> = vec![
                ("Test Artist".into(), 0),
                ("Test Title".into(), 0),
                ("Test Album".into(), 0),
                ("2000".into(), 0),
                ("7".into(), 0),
                ("Test Genre".into(), 0),
                ("Test Comment".into(), 0),
            ];
            write_tags(&dst, &fields).unwrap_or_else(|e| panic!("write_tags failed for {ext}: {e}"));
            let tags = {
                use symphonia::core::formats::FormatOptions;
                use symphonia::core::io::MediaSourceStream;
                use symphonia::core::meta::MetadataOptions;
                use symphonia::core::probe::Hint;
                let f = std::fs::File::open(&dst).unwrap();
                let mss = MediaSourceStream::new(Box::new(f), Default::default());
                let mut hint = Hint::new();
                hint.with_extension(ext);
                let mut probed = symphonia::default::get_probe()
                    .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
                    .unwrap();
                collect_tags(&mut probed)
            };
            use symphonia::core::meta::StandardTagKey;
            let find = |key: StandardTagKey| -> String {
                tags.iter().find(|t| t.std_key == Some(key))
                    .map(|t| t.value.to_string()).unwrap_or_default()
            };
            assert_eq!(find(StandardTagKey::Artist),     "Test Artist", "{ext} artist mismatch");
            assert_eq!(find(StandardTagKey::TrackTitle), "Test Title",  "{ext} title mismatch");
            assert_eq!(find(StandardTagKey::Album),      "Test Album",  "{ext} album mismatch");
            assert_eq!(find(StandardTagKey::Genre),      "Test Genre",  "{ext} genre mismatch");
        }
    }
}
