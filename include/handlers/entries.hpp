#pragma once

#include "error.hpp"
#include <filesystem>
#include <string>
#include <vector>

namespace simple::handlers {

auto process_entry(
    const std::filesystem::path &src, std::string_view name,
    std::string entry_path, std::string result_path,
    std::vector<std::pair<std::string_view, std::string_view>> kv)
    -> std::vector<ProcessError>;

}
