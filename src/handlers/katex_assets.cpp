#include "handlers/katex_assets.hpp"
#include <atomic>
#include <cstdlib>
#include <iostream>

namespace simple::handlers {

thread_local std::atomic<bool> katex_used{false};
static std::atomic<bool> message_printed{false};

auto mark_katex_used() -> void {
  katex_used.store(true, std::memory_order_relaxed);
}

auto was_katex_used() -> bool {
  return katex_used.load(std::memory_order_relaxed);
}

auto reset_katex_flag() -> void {
  katex_used.store(false, std::memory_order_relaxed);
}

auto print_katex_message() -> void {
  bool expected = false;
  if (message_printed.compare_exchange_strong(expected, true,
                                              std::memory_order_relaxed)) {
    std::cout << "  📐 KaTeX CSS will be injected (using CDN)\n";
  }
}

auto is_katex_injection_disabled() -> bool {
  return std::getenv("SIMPLE_DISABLE_KATEX_CSS") != nullptr;
}

auto get_katex_css_tag() -> std::string_view {
  return R"(<!-- KaTeX CSS (auto-injected from CDN) -->
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css" integrity="sha384-nB0miv6/jRmo5UMMR1wu3Gz6NLsoTkbqJghGIsx//Rlm+ZU03BU6SQNC66uf4l5+" crossorigin="anonymous">)";
}

} // namespace simple::handlers
