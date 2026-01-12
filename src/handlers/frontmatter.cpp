#include "handlers/frontmatter.hpp"
#include <format>
#include <fstream>
#include <sstream>
#include <toml++/toml.hpp>
#include <yaml-cpp/yaml.h>

namespace simple::handlers {

auto extract_frontmatter(std::string_view content) -> Result<
    std::pair<std::unordered_map<std::string, std::string>, std::string>> {

  size_t pos = 0;
  while (pos < content.size() && std::isspace(content[pos])) {
    ++pos;
  }
  content.remove_prefix(pos);

  if (!content.starts_with("---")) {
    return std::unexpected(ProcessError{ErrorType::Syntax,
                                        WithItem::Data,
                                        {},
                                        "Frontmatter must start with '---'"});
  }

  auto after_first = content.substr(3);
  auto end_pos = after_first.find("\n---");

  if (end_pos == std::string_view::npos) {
    return std::unexpected(ProcessError{ErrorType::Syntax,
                                        WithItem::Data,
                                        {},
                                        "Frontmatter must end with '---'"});
  }

  auto frontmatter_str = after_first.substr(0, end_pos);
  auto remaining = after_first.substr(end_pos + 4);

  while (!remaining.empty() && std::isspace(remaining[0])) {
    remaining.remove_prefix(1);
  }

  try {
    auto yaml = YAML::Load(std::string{frontmatter_str});
    std::unordered_map<std::string, std::string> map;

    if (yaml.IsMap()) {
      for (const auto &kv : yaml) {
        auto key = kv.first.as<std::string>();
        std::string value;

        if (kv.second.IsScalar()) {
          value = kv.second.as<std::string>();
        } else {
          value = "";
        }

        map[key] = value;
      }
    }

    if (!map.contains("title")) {
      return std::unexpected(
          ProcessError{ErrorType::Syntax,
                       WithItem::Data,
                       {},
                       "Frontmatter must contain a 'title' field"});
    }

    return std::make_pair(std::move(map), std::string{remaining});

  } catch (const YAML::Exception &e) {
    return std::unexpected(ProcessError{
        ErrorType::Syntax,
        WithItem::Data,
        {},
        std::format("Failed to parse YAML frontmatter: {}", e.what())});
  }
}

auto load_frontmatter_data(const std::filesystem::path &src,
                           std::string_view name)
    -> std::expected<std::pair<nlohmann::json, std::vector<ProcessError>>,
                     std::vector<ProcessError>> {

  std::vector<ProcessError> errors;

  std::string name_str{name};
  size_t pos = 0;
  while ((pos = name_str.find(':', pos)) != std::string::npos) {
    name_str.replace(pos, 1, "/");
  }

  auto toml_path = src / "data" / (name_str + ".data.toml");

  toml::table tbl;
  try {
    tbl = toml::parse_file(toml_path.string());
  } catch (const toml::parse_error &e) {
    return std::unexpected(std::vector{
        ProcessError{ErrorType::Syntax, WithItem::Data, toml_path,
                     std::format("Failed to parse TOML: {}", e.what())}});
  }

  nlohmann::json items = nlohmann::json::array();
  auto data_dir = src / "data" / name_str;

  auto files_node = tbl["files"];
  if (!files_node.is_array()) {
    return std::unexpected(
        std::vector{ProcessError{ErrorType::Syntax, WithItem::Data, toml_path,
                                 "Missing 'files' array in TOML"}});
  }

  auto files_array = *files_node.as_array();
  std::vector<std::string> files;
  for (const auto &item : files_array) {
    if (item.is_string()) {
      files.push_back(std::string{*item.value<std::string>()});
    }
  }

  for (const auto &file : files) {
    auto md_path = data_dir / file;

    std::ifstream md_file{md_path};
    if (!md_file) {
      errors.push_back(
          ProcessError{ErrorType::Io, WithItem::Data, md_path,
                       std::format("Failed to read markdown file: {}", file)});
      continue;
    }

    std::stringstream md_buffer;
    md_buffer << md_file.rdbuf();
    auto content = md_buffer.str();

    auto fm_result = extract_frontmatter(content);
    if (!fm_result) {
      auto err = fm_result.error();
      err.path = md_path;
      errors.push_back(err);
      continue;
    }

    auto [frontmatter, remaining] = std::move(*fm_result);

    auto file_stem = md_path.stem().string();
    auto relative_entry_path = std::format("{}/{}", name_str, file);
    auto result_path = std::format("content/{}.html", file_stem);

    frontmatter["--entry-path"] = relative_entry_path;
    frontmatter["--result-path"] = result_path;
    frontmatter["link"] = std::format("./{}", result_path);

    nlohmann::json obj = nlohmann::json::object();
    for (const auto &[k, v] : frontmatter) {
      obj[k] = v;
    }

    items.push_back(obj);
  }

  return std::make_pair(std::move(items), std::move(errors));
}

} // namespace simple::handlers
