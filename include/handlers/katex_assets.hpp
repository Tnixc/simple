#pragma once

#include <string_view>

namespace simple::handlers {

auto mark_katex_used() -> void;
auto was_katex_used() -> bool;
auto reset_katex_flag() -> void;
auto print_katex_message() -> void;
auto is_katex_injection_disabled() -> bool;
auto get_katex_css_tag() -> std::string_view;

} // namespace simple::handlers
