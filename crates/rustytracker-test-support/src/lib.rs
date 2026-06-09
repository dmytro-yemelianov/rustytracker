use std::path::PathBuf;

pub fn milkytracker_fixture_root() -> Option<PathBuf> {
    if let Some(root) = std::env::var_os("MILKYTRACKER_ROOT") {
        let root = PathBuf::from(root);
        let candidates = [root.join("resources/music"), root];
        if let Some(path) = candidates.into_iter().find(|path| path.is_dir()) {
            return Some(path);
        }
    }

    let sibling =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../MilkyTracker/resources/music");
    sibling.is_dir().then_some(sibling)
}

pub fn milkytracker_fixtures_available() -> bool {
    milkytracker_fixture_root().is_some()
}

pub fn milkytracker_fixture_path(file_name: &str) -> PathBuf {
    milkytracker_fixture_root()
        .expect("MilkyTracker fixtures not found; set MILKYTRACKER_ROOT or clone MilkyTracker next to rustytracker")
        .join(file_name)
}
