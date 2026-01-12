#pragma once

#include "error.hpp"
#include "utils.hpp"
#include <filesystem>
#include <string>
#include <unordered_set>

namespace simple::handlers {

auto get_template(const std::filesystem::path &src, std::string_view name,
                  std::unordered_set<std::filesystem::path> hist)
    -> ProcessResult;

auto process_template(const std::filesystem::path &src, std::string input,
                      std::unordered_set<std::filesystem::path> hist)
    -> ProcessResult;

} // namespace simple::handlers
