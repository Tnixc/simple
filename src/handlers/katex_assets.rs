use once_cell::sync::Lazy;
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};

// Track if KaTeX was used in the current page (thread-local)
thread_local! {
    static KATEX_USED: AtomicBool = const { AtomicBool::new(false) };
}

// Track if we've printed the KaTeX message (global, one-time)
static MESSAGE_PRINTED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

pub fn mark_katex_used() {
    KATEX_USED.with(|used| used.store(true, Ordering::Relaxed));
}

pub fn was_katex_used() -> bool {
    KATEX_USED.with(|used| used.load(Ordering::Relaxed))
}

pub fn reset_katex_flag() {
    KATEX_USED.with(|used| used.store(false, Ordering::Relaxed));
}

pub fn print_katex_message() {
    if !MESSAGE_PRINTED.swap(true, Ordering::Relaxed) {
        println!("  📐 KaTeX CSS will be injected (using CDN)");
    }
}

pub fn is_katex_injection_disabled() -> bool {
    env::var("SIMPLE_DISABLE_KATEX_CSS").is_ok()
}

pub fn get_katex_css_tag() -> &'static str {
    r#"<!-- KaTeX CSS (auto-injected from CDN) -->
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css" integrity="sha384-nB0miv6/jRmo5UMMR1wu3Gz6NLsoTkbqJghGIsx//Rlm+ZU03BU6SQNC66uf4l5+" crossorigin="anonymous">"#
}
