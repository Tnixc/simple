#pragma once

#include "error.hpp"
#include <filesystem>
#include <nlohmann/json.hpp>
#include <string>
#include <unordered_map>

namespace simple::handlers {

struct FileList {
  std::vector<std::string> files;
};

auto extract_frontmatter(std::string_view content) -> Result<
    std::pair<std::unordered_map<std::string, std::string>, std::string>>;

auto load_frontmatter_data(const std::filesystem::path &src,
                           std::string_view name)
    -> std::expected<std::pair<nlohmann::json, std::vector<ProcessError>>,
                     std::vector<ProcessError>>;

} // namespace simple::handlers
