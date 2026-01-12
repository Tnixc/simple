#pragma once

#include <string>
#include <string_view>

namespace simple {

auto minify_html(std::string_view html) -> std::string;

}
