use ignore::WalkBuilder;
use nucleo::{Config, Nucleo};
use std::sync::Arc;

pub fn resolve_anonymous_path(base_dir: &str, pattern: &str) -> Option<String> {
    let mut matcher = Nucleo::<String>::new(Config::DEFAULT, Arc::new(|| {}), None, 1);
    let injector = matcher.injector();

    // ──> NEW: Dynamic Depth Resolution <──
    let max_depth = std::env::var("RSH_FUZZY_DEPTH")
        .ok()
        .and_then(|val| val.parse::<usize>().ok())
        .unwrap_or(4); // Default to 4 if missing or invalid

    let mut builder = WalkBuilder::new(base_dir);
    builder
        .hidden(true)
        .git_ignore(true)
        .max_depth(Some(max_depth)) // <--- Inject the dynamic variable here
        .threads(8)
        .same_file_system(true);

    builder.filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        name != "node_modules" && name != "target"
    });

    // ... (keep crawler execution)

    // 3. Run the crawler
    builder.build_parallel().run(|| {
        let local_injector = injector.clone();
        Box::new(move |result| {
            if let Ok(entry) = result {
                if entry.depth() > 0 {
                    let path_str = entry.path().to_string_lossy().into_owned();
                    local_injector.push(path_str, |s, dst| {
                        dst[0] = s.as_str().into();
                    });
                }
            }
            ignore::WalkState::Continue
        })
    });

    // 4. Feed the pattern to Nucleo
    let exact_pattern = format!("'{}", pattern);

    matcher.pattern.reparse(
        0,
        &exact_pattern,
        nucleo::pattern::CaseMatching::Ignore,
        nucleo::pattern::Normalization::Smart,
        false,
    );

    // 5. Spin the matcher until it finishes scoring the injected paths
    while matcher.tick(10).running {}

    // 6. Return the highest scoring path!
    let snapshot = matcher.snapshot();
    if let Some(item) = snapshot.get_matched_item(0) {
        Some(item.data.clone())
    } else {
        None
    }
}
