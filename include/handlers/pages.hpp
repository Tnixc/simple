#pragma once

#include "error.hpp"
#include "utils.hpp"
#include <filesystem>
#include <string>
#include <unordered_set>

namespace simple::handlers {

auto page(const std::filesystem::path &src, std::string string,
          std::unordered_set<std::filesystem::path> hist) -> ProcessResult;

auto process_pages(const std::filesystem::path &dir,
                   const std::filesystem::path &src,
                   const std::filesystem::path &source,
                   const std::filesystem::path &pages) -> Results<void>;

} // namespace simple::handlers
